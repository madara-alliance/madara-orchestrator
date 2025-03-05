pub mod utils;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use ::utils::collections::{has_dup, is_sorted};
use async_trait::async_trait;
use cairo_vm::Felt252;
use chrono::{SubsecRound, Utc};
use color_eyre::eyre::eyre;
use settlement_client_interface::SettlementVerificationStatus;
use starknet::core::types::Felt;
use starknet_os::io::output::{ContractChanges, StarknetOsOutput};
use swiftness_proof_parser::{parse, StarkProof};
use thiserror::Error;
use tokio::time::sleep;
use uuid::Uuid;

use super::constants::{
    JOB_METADATA_STATE_UPDATE_ATTEMPT_PREFIX, JOB_METADATA_STATE_UPDATE_LAST_FAILED_BLOCK_NO,
    JOB_PROCESS_ATTEMPT_METADATA_KEY,
};
use super::{JobError, OtherError};
use crate::config::{self, Config};
use crate::constants::{
    ON_CHAIN_DATA_FILE_NAME, PROGRAM_OUTPUT_FILE_NAME, PROOF_FILE_NAME, PROOF_PART2_FILE_NAME, SNOS_OUTPUT_FILE_NAME,
};
use crate::jobs::constants::JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY;
use crate::jobs::snos_job::fact_info::OnChainData;
use crate::jobs::state_update_job::utils::fetch_blob_data_for_block;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;

#[derive(Error, Debug, PartialEq)]
pub enum StateUpdateError {
    #[error("Block numbers list should not be empty.")]
    EmptyBlockNumberList,

    #[error("Could not find current attempt number.")]
    AttemptNumberNotFound,

    #[error("last_failed_block should be a positive number")]
    LastFailedBlockNonPositive,

    #[error("Block numbers to settle must be specified (state update job #{internal_id:?})")]
    UnspecifiedBlockNumber { internal_id: String },

    #[error("Could not find tx hashes metadata for the current attempt")]
    TxnHashMetadataNotFound,

    #[error("Tx {tx_hash:?} should not be pending.")]
    TxnShouldNotBePending { tx_hash: String },

    #[error(
        "Last number in block_numbers array returned as None. Possible Error : Delay in job processing or Failed job \
         execution."
    )]
    LastNumberReturnedError,

    #[error("No block numbers found.")]
    BlockNumberNotFound,

    #[error("Duplicated block numbers.")]
    DuplicateBlockNumbers,

    #[error("Block numbers aren't sorted in increasing order.")]
    UnsortedBlockNumbers,

    #[error("Gap detected between the first block to settle and the last one settled.")]
    GapBetweenFirstAndLastBlock,

    #[error("Block #{block_no:?} - SNOS error, [use_kzg_da] should be either 0 or 1.")]
    UseKZGDaError { block_no: u64 },

    #[error("Other error: {0}")]
    Other(#[from] OtherError),
}

pub struct StateUpdateJob;
#[async_trait]
impl Job for StateUpdateJob {
    #[tracing::instrument(fields(category = "state_update"), skip(self, _config, metadata), ret, err)]
    async fn create_job(
        &self,
        _config: Arc<Config>,
        internal_id: String,
        metadata: HashMap<String, String>,
    ) -> Result<JobItem, JobError> {
        tracing::info!(log_type = "starting", category = "state_update", function_type = "create_job",  block_no = %internal_id, "State update job creation started.");
        // Inserting the metadata (If it doesn't exist)
        let mut metadata = metadata.clone();
        if !metadata.contains_key(JOB_PROCESS_ATTEMPT_METADATA_KEY) {
            tracing::debug!(job_id = %internal_id, "Inserting initial process attempt metadata");
            metadata.insert(JOB_PROCESS_ATTEMPT_METADATA_KEY.to_string(), "0".to_string());
        }

        let job_item = JobItem {
            id: Uuid::new_v4(),
            internal_id: internal_id.clone(),
            job_type: JobType::StateTransition,
            status: JobStatus::Created,
            external_id: String::new().into(),
            // metadata must contain the blocks for which state update will be performed
            // we don't do one job per state update as that makes nonce management complicated
            metadata,
            version: 0,
            created_at: Utc::now().round_subsecs(0),
            updated_at: Utc::now().round_subsecs(0),
        };
        tracing::info!(log_type = "completed", category = "state_update", function_type = "create_job",  block_no = %internal_id, "State update job created.");
        Ok(job_item)
    }

    #[tracing::instrument(fields(category = "state_update"), skip(self, config), ret, err)]
    async fn process_job(&self, config: Arc<Config>, job: &mut JobItem) -> Result<String, JobError> {
        let internal_id = job.internal_id.clone();
        tracing::info!(log_type = "starting", category = "state_update", function_type = "process_job", job_id = %job.id,  block_no = %internal_id, "State update job processing started.");
        let attempt_no = job
            .metadata
            .get(JOB_PROCESS_ATTEMPT_METADATA_KEY)
            .ok_or_else(|| StateUpdateError::AttemptNumberNotFound)?
            .clone();

        // Read the metadata to get the blocks for which state update will be performed.
        // We assume that blocks nbrs are formatted as follow: "2,3,4,5,6".
        let mut block_numbers = self.get_block_numbers_from_metadata(job)?;
        self.validate_block_numbers(config.clone(), &block_numbers).await?;

        if let Some(last_failed_block) = job.metadata.get(JOB_METADATA_STATE_UPDATE_LAST_FAILED_BLOCK_NO) {
            let last_failed_block =
                last_failed_block.parse().map_err(|_| StateUpdateError::LastFailedBlockNonPositive)?;
            block_numbers = block_numbers.into_iter().filter(|&block| block >= last_failed_block).collect::<Vec<u64>>();
        }

        let mut nonce = config.settlement_client().get_nonce().await.map_err(|e| JobError::Other(OtherError(e)))?;
        let mut sent_tx_hashes: Vec<String> = Vec::with_capacity(block_numbers.len());
        for block_no in block_numbers.iter() {
            sleep(Duration::from_secs(20)).await;
            let nonce = config.settlement_client().get_nonce().await.map_err(|e| JobError::Other(OtherError(e)))?;
            tracing::debug!(job_id = %job.internal_id, block_no = %block_no, "Processing block");

            let snos = self.fetch_snos_for_block(*block_no, config.clone()).await?;
            let txn_hash = self
                .update_state_for_block(config.clone(), *block_no, snos, nonce)
                .await
                .map_err(|e| {
                    tracing::error!(job_id = %job.internal_id, block_no = %block_no, error = %e, "Error updating state for block");
                    job.metadata.insert(JOB_METADATA_STATE_UPDATE_LAST_FAILED_BLOCK_NO.into(), block_no.to_string());
                    self.insert_attempts_into_metadata(job, &attempt_no, &sent_tx_hashes);
                    OtherError(eyre!("Block #{block_no} - Error occurred during the state update: {e}"));
                })
                .unwrap();
            sent_tx_hashes.push(txn_hash);
        }

        self.insert_attempts_into_metadata(job, &attempt_no, &sent_tx_hashes);

        let val = block_numbers.last().ok_or_else(|| StateUpdateError::LastNumberReturnedError)?;
        tracing::info!(log_type = "completed", category = "state_update", function_type = "process_job", job_id = %job.id,  block_no = %internal_id, last_settled_block = %val, "State update job processed successfully.");

        Ok(val.to_string())
    }

    /// Returns the status of the passed job.
    /// Status will be verified if:
    /// 1. the last settlement tx hash is successful,
    /// 2. the expected last settled block from our configuration is indeed the one found in the
    ///    provider.
    #[tracing::instrument(fields(category = "state_update"), skip(self, config), ret, err)]
    async fn verify_job(&self, config: Arc<Config>, job: &mut JobItem) -> Result<JobVerificationStatus, JobError> {
        let internal_id = job.internal_id.clone();
        tracing::info!(log_type = "starting", category = "state_update", function_type = "verify_job", job_id = %job.id,  block_no = %internal_id, "State update job verification started.");
        let attempt_no = job
            .metadata
            .get(JOB_PROCESS_ATTEMPT_METADATA_KEY)
            .ok_or_else(|| StateUpdateError::AttemptNumberNotFound)?;
        tracing::debug!(job_id = %job.internal_id, attempt_no = %attempt_no, "Retrieved attempt number");

        // We are doing attempt_no - 1 because the attempt number is increased in the
        // global process job function and the transaction hash is stored with attempt
        // number : 0
        let metadata_tx_hashes = job
            .metadata
            .get(&format!(
                "{}{}",
                JOB_METADATA_STATE_UPDATE_ATTEMPT_PREFIX,
                attempt_no.parse::<u32>().map_err(|e| JobError::Other(OtherError(eyre!(e))))? - 1
            ))
            .ok_or_else(|| StateUpdateError::TxnHashMetadataNotFound)?
            .clone()
            .replace(' ', "");

        let tx_hashes: Vec<&str> = metadata_tx_hashes.split(',').collect();
        let block_numbers = self.get_block_numbers_from_metadata(job)?;
        tracing::debug!(job_id = %job.internal_id, "Retrieved block numbers from metadata");
        let settlement_client = config.settlement_client();

        for (tx_hash, block_no) in tx_hashes.iter().zip(block_numbers.iter()) {
            tracing::trace!(job_id = %job.internal_id, tx_hash = %tx_hash, block_no = %block_no, "Verifying transaction inclusion");
            let tx_inclusion_status =
                settlement_client.verify_tx_inclusion(tx_hash).await.map_err(|e| JobError::Other(OtherError(e)))?;
            match tx_inclusion_status {
                SettlementVerificationStatus::Rejected(_) => {
                    tracing::warn!(job_id = %job.internal_id, tx_hash = %tx_hash, block_no = %block_no, "Transaction rejected");
                    job.metadata.insert(JOB_METADATA_STATE_UPDATE_LAST_FAILED_BLOCK_NO.into(), block_no.to_string());
                    return Ok(tx_inclusion_status.into());
                }
                // If the tx is still pending, we wait for it to be finalized and check again the status.
                SettlementVerificationStatus::Pending => {
                    tracing::debug!(job_id = %job.internal_id, tx_hash = %tx_hash, "Transaction pending, waiting for finality");
                    settlement_client
                        .wait_for_tx_finality(tx_hash)
                        .await
                        .map_err(|e| JobError::Other(OtherError(e)))?;
                    let new_status = settlement_client
                        .verify_tx_inclusion(tx_hash)
                        .await
                        .map_err(|e| JobError::Other(OtherError(e)))?;
                    match new_status {
                        SettlementVerificationStatus::Rejected(_) => {
                            tracing::warn!(job_id = %job.internal_id, tx_hash = %tx_hash, block_no = %block_no, "Transaction rejected after finality");
                            job.metadata
                                .insert(JOB_METADATA_STATE_UPDATE_LAST_FAILED_BLOCK_NO.into(), block_no.to_string());
                            return Ok(new_status.into());
                        }
                        SettlementVerificationStatus::Pending => {
                            tracing::error!(job_id = %job.internal_id, tx_hash = %tx_hash, "Transaction still pending after finality check");
                            Err(StateUpdateError::TxnShouldNotBePending { tx_hash: tx_hash.to_string() })?
                        }
                        SettlementVerificationStatus::Verified => {
                            tracing::debug!(job_id = %job.internal_id, tx_hash = %tx_hash, "Transaction verified after finality");
                        }
                    }
                }
                SettlementVerificationStatus::Verified => {
                    tracing::debug!(job_id = %job.internal_id, tx_hash = %tx_hash, "Transaction verified");
                }
            }
        }
        // verify that the last settled block is indeed the one we expect to be
        let expected_last_block_number = block_numbers.last().ok_or_else(|| StateUpdateError::EmptyBlockNumberList)?;

        let out_last_block_number =
            settlement_client.get_last_settled_block().await.map_err(|e| JobError::Other(OtherError(e)))?;
        let block_status = if out_last_block_number == *expected_last_block_number {
            tracing::info!(log_type = "completed", category = "state_update", function_type = "verify_job", job_id = %job.id,  block_no = %internal_id, last_settled_block = %out_last_block_number, "Last settled block verified.");
            SettlementVerificationStatus::Verified
        } else {
            tracing::warn!(log_type = "failed/rejected", category = "state_update", function_type = "verify_job", job_id = %job.id,  block_no = %internal_id, expected = %expected_last_block_number, actual = %out_last_block_number, "Last settled block mismatch.");
            SettlementVerificationStatus::Rejected(format!(
                "Last settle bock expected was {} but found {}",
                expected_last_block_number, out_last_block_number
            ))
        };
        Ok(block_status.into())
    }

    fn max_process_attempts(&self) -> u64 {
        1
    }

    fn max_verification_attempts(&self) -> u64 {
        10
    }

    fn verification_polling_delay_seconds(&self) -> u64 {
        60
    }

    fn job_processing_lock(
        &self,
        _config: Arc<Config>,
    ) -> std::option::Option<std::sync::Arc<config::JobProcessingState>> {
        None
    }
}

impl StateUpdateJob {
    /// Read the metadata and parse the block numbers
    fn get_block_numbers_from_metadata(&self, job: &JobItem) -> Result<Vec<u64>, JobError> {
        let blocks_to_settle = job
            .metadata
            .get(JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY)
            .ok_or_else(|| StateUpdateError::UnspecifiedBlockNumber { internal_id: job.internal_id.clone() })?;

        self.parse_block_numbers(blocks_to_settle)
    }

    /// Parse a list of blocks comma separated
    fn parse_block_numbers(&self, blocks_to_settle: &str) -> Result<Vec<u64>, JobError> {
        let sanitized_blocks = blocks_to_settle.replace(' ', "");
        let block_numbers: Vec<u64> = sanitized_blocks
            .split(',')
            .map(|block_no| block_no.parse::<u64>())
            .collect::<Result<Vec<u64>, _>>()
            .map_err(|e| eyre!("Block numbers to settle list is not correctly formatted: {e}"))
            .map_err(|e| JobError::Other(OtherError(e)))?;
        Ok(block_numbers)
    }

    /// Validate that the list of block numbers to process is valid.
    async fn validate_block_numbers(&self, config: Arc<Config>, block_numbers: &[u64]) -> Result<(), JobError> {
        if block_numbers.is_empty() {
            Err(StateUpdateError::BlockNumberNotFound)?;
        }
        if has_dup(block_numbers) {
            Err(StateUpdateError::DuplicateBlockNumbers)?;
        }
        if !is_sorted(block_numbers) {
            Err(StateUpdateError::UnsortedBlockNumbers)?;
        }
        // Check for gap between the last settled block and the first block to settle
        let last_settled_block: u64 =
            config.settlement_client().get_last_settled_block().await.map_err(|e| JobError::Other(OtherError(e)))?;
        if last_settled_block + 1 != block_numbers[0] {
            Err(StateUpdateError::GapBetweenFirstAndLastBlock)?;
        }
        Ok(())
    }

    /// Update the state for the corresponding block using the settlement layer.
    async fn update_state_for_block(
        &self,
        config: Arc<Config>,
        block_no: u64,
        snos: StarknetOsOutput,
        nonce: u64,
    ) -> Result<String, JobError> {
        let settlement_client = config.settlement_client();

        // We use KZG_DA flag in order to determine whether we are using L1 or L2 as
        // settlement layer.
        // So in case of KZG flag == 0 :
        //      We send the transaction using `update_state_calldata`
        // And in case of KZG flag == 1 :
        //      We send the transaction using `update_state_with_blobs

        let last_tx_hash_executed = if snos.use_kzg_da == Felt252::ZERO {
            let proof_key = format!("{block_no}/{PROOF_FILE_NAME}");
            // tracing::debug!(job_id = %job.internal_id, %proof_key, "Fetching snos proof file");

            let proof_file = config.storage().get_data(&proof_key).await.map_err(|e| {
                // tracing::error!(job_id = %job.internal_id, error = %e, "Failed to fetch proof file");
                JobError::Other(OtherError(e))
            })?;

            let snos_proof = String::from_utf8(proof_file.to_vec()).map_err(|e| {
                // tracing::error!(job_id = %job.internal_id, error = %e, "Failed to parse proof file as UTF-8");
                JobError::Other(OtherError(eyre!("{}", e)))
            })?;

            let parsed_snos_proof: StarkProof = parse(snos_proof.clone()).map_err(|e| {
                // tracing::error!(job_id = %job.internal_id, error = %e, "Failed to parse proof file as UTF-8");
                JobError::Other(OtherError(eyre!("{}", e)))
            })?;

            let proof_key = format!("{block_no}/{PROOF_PART2_FILE_NAME}");
            // tracing::debug!(job_id = %job.internal_id, %proof_key, "Fetching 2nd proof file");

            let proof_file = config.storage().get_data(&proof_key).await.map_err(|e| {
                // tracing::error!(job_id = %job.internal_id, error = %e, "Failed to fetch 2nd proof file");
                JobError::Other(OtherError(e))
            })?;

            let second_proof = String::from_utf8(proof_file.to_vec()).map_err(|e| {
                // tracing::error!(job_id = %job.internal_id, error = %e, "Failed to parse proof file as UTF-8");
                JobError::Other(OtherError(eyre!("{}", e)))
            })?;

            let parsed_bridge_proof: StarkProof = parse(second_proof.clone()).map_err(|e| {
                // tracing::error!(job_id = %job.internal_id, error = %e, "Failed to parse proof file as UTF-8");
                JobError::Other(OtherError(eyre!("{}", e)))
            })?;

            let snos_output = vec_felt_to_vec_bytes32(calculate_output(parsed_snos_proof));
            let program_output = vec_felt_to_vec_bytes32(calculate_output(parsed_bridge_proof));

            // let program_output = self.fetch_program_output_for_block(block_no, config.clone()).await;
            let onchain_data = self.fetch_onchain_data_for_block(block_no, config.clone()).await;
            settlement_client
                .update_state_calldata(
                    snos_output,
                    program_output,
                    onchain_data.on_chain_data_hash.0,
                    usize_to_bytes(onchain_data.on_chain_data_size),
                )
                .await
                .map_err(|e| JobError::Other(OtherError(e)))?
        } else if snos.use_kzg_da == Felt252::ONE {
            let blob_data = fetch_blob_data_for_block(block_no, config.clone())
                .await
                .map_err(|e| JobError::Other(OtherError(e)))?;

            let program_output = self.fetch_program_output_for_block(block_no, config.clone()).await?;

            // TODO :
            // Fetching nonce before the transaction is run
            // Sending update_state transaction from the settlement client
            settlement_client
                .update_state_with_blobs(program_output, blob_data, nonce)
                .await
                .map_err(|e| JobError::Other(OtherError(e)))?
        } else {
            Err(StateUpdateError::UseKZGDaError { block_no })?
        };
        Ok(last_tx_hash_executed)
    }

    /// Retrieves the SNOS output for the corresponding block.
    async fn fetch_snos_for_block(&self, block_no: u64, config: Arc<Config>) -> Result<StarknetOsOutput, JobError> {
        let storage_client = config.storage();
        let key = block_no.to_string() + "/" + SNOS_OUTPUT_FILE_NAME;

        let snos_output_bytes = storage_client.get_data(&key).await.map_err(|e| JobError::Other(OtherError(e)))?;

        serde_json::from_slice(snos_output_bytes.iter().as_slice()).map_err(|e| {
            JobError::Other(OtherError(eyre!("Failed to deserialize SNOS output for block {}: {}", block_no, e)))
        })
    }

    /// Retrieves the Program output for the corresponding block.
    async fn fetch_program_output_for_block(
        &self,
        block_number: u64,
        config: Arc<Config>,
    ) -> Result<Vec<[u8; 32]>, JobError> {
        let storage_client = config.storage();
        let key = block_number.to_string() + "/" + PROGRAM_OUTPUT_FILE_NAME;

        let program_output = storage_client.get_data(&key).await.map_err(|e| JobError::Other(OtherError(e)))?;

        bincode::deserialize(&program_output).map_err(|e| {
            JobError::Other(OtherError(eyre!("Failed to deserialize program output for block {}: {}", block_number, e)))
        })
    }

    /// Retrieves the OnChain data for the corresponding block.
    async fn fetch_onchain_data_for_block(&self, block_number: u64, config: Arc<Config>) -> OnChainData {
        let storage_client = config.storage();
        let key = block_number.to_string() + "/" + ON_CHAIN_DATA_FILE_NAME;
        let onchain_data_bytes = storage_client.get_data(&key).await.expect("Unable to fetch onchain data for block");
        serde_json::from_slice(onchain_data_bytes.iter().as_slice())
            .expect("Unable to convert the data into onchain data")
    }

    /// Insert the tx hashes into the metadata for the attempt number - will be used later by
    /// verify_job to make sure that all tx are successful.
    fn insert_attempts_into_metadata(&self, job: &mut JobItem, attempt_no: &str, tx_hashes: &[String]) {
        let new_attempt_metadata_key = format!("{}{}", JOB_METADATA_STATE_UPDATE_ATTEMPT_PREFIX, attempt_no);
        job.metadata.insert(new_attempt_metadata_key, tx_hashes.join(","));
    }
}

pub fn calculate_output(proof: StarkProof) -> Vec<Felt> {
    let output_segment = proof.public_input.segments[2].clone();
    let output_len = output_segment.stop_ptr - output_segment.begin_addr;
    let start = proof.public_input.main_page.len() - output_len as usize;
    let end = proof.public_input.main_page.len();
    let program_output =
        proof.public_input.main_page[start..end].iter().map(|cell| cell.value.clone()).collect::<Vec<_>>();
    let mut felts = vec![];
    for elem in &program_output {
        felts.push(Felt::from_dec_str(&elem.to_string()).unwrap());
    }
    felts
}

pub fn vec_felt_to_vec_bytes32(felts: Vec<Felt>) -> Vec<[u8; 32]> {
    felts
        .into_iter()
        .map(|felt| {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&felt.to_bytes_be());
            bytes
        })
        .collect()
}

fn convert_snos_output_into_bytes_vec(snos: StarknetOsOutput) -> Vec<[u8; 32]> {
    let mut snos_vec = Vec::new();

    snos_vec.push(snos.initial_root.to_bytes_be());
    snos_vec.push(snos.final_root.to_bytes_be());
    snos_vec.push(snos.prev_block_number.to_bytes_be());
    snos_vec.push(snos.new_block_number.to_bytes_be());
    snos_vec.push(snos.prev_block_hash.to_bytes_be());
    snos_vec.push(snos.new_block_hash.to_bytes_be());
    snos_vec.push(snos.os_program_hash.to_bytes_be());
    snos_vec.push(snos.starknet_os_config_hash.to_bytes_be());
    snos_vec.push(snos.use_kzg_da.to_bytes_be());
    snos_vec.push(snos.full_output.to_bytes_be());

    // Processing Messages to L1
    snos_vec.push(usize_to_bytes(snos.messages_to_l1.len()));
    for messages in snos.messages_to_l1 {
        snos_vec.push(messages.to_bytes_be());
    }

    // Processing Messages to L2
    snos_vec.push(usize_to_bytes(snos.messages_to_l2.len()));
    for messages in snos.messages_to_l2 {
        snos_vec.push(messages.to_bytes_be());
    }

    // Processing Contract Changes
    snos_vec.push(usize_to_bytes(snos.contracts.len()));
    for contract in snos.contracts {
        snos_vec.extend(convert_contract_changes_into_vec(contract));
    }

    // Processing Class Changes
    snos_vec.extend(convert_class_changes_into_vec(snos.classes, snos.full_output));

    snos_vec
}

fn convert_contract_changes_into_vec(contract_changes: ContractChanges) -> Vec<[u8; 32]> {
    let mut result = Vec::new();

    result.push(contract_changes.addr.to_bytes_be());

    // Calculate the bound (2^64)
    let bound = Felt252::from(18446744073709551616u128);

    // Calculate was_class_updated
    let was_class_updated = if contract_changes.class_hash.is_some() { Felt252::ONE } else { Felt252::ZERO };

    // Pack the values:
    // new_value = (was_class_updated * bound + new_state_nonce)
    // value = new_value * bound + n_actual_updates
    let new_value = was_class_updated * bound + contract_changes.nonce;
    let n_actual_updates = Felt252::from(contract_changes.storage_changes.len());
    let packed_value = new_value * bound + n_actual_updates;

    result.push(packed_value.to_bytes_be());

    if let Some(class_hash) = contract_changes.class_hash {
        result.push(class_hash.to_bytes_be());
    }

    // Sort the keys to ensure consistent ordering
    let mut storage_entries: Vec<_> = contract_changes.storage_changes.iter().collect();
    storage_entries.sort_by_key(|&(k, _)| k);

    for (key, value) in storage_entries {
        result.push(key.to_bytes_be());
        result.push(value.to_bytes_be());
    }

    result
}

fn convert_class_changes_into_vec(changes: HashMap<Felt252, Felt252>, full_output: Felt252) -> Vec<[u8; 32]> {
    let mut result = Vec::new();

    result.push(Felt252::from(changes.len()).to_bytes_be());

    // Add all changes in sorted order for deterministic output
    let mut sorted_changes: Vec<_> = changes.iter().collect();
    sorted_changes.sort_by_key(|&(k, _)| k);

    for (class_hash, compiled_class_hash) in sorted_changes {
        // Add class_hash
        result.push(class_hash.to_bytes_be());

        // If full_output is true, add a dummy value
        // This matches the Cairo code's behavior where it reads and discards a value
        if full_output == Felt252::ONE {
            result.push(Felt252::ZERO.to_bytes_be()); // or another appropriate dummy value
        }

        // Add compiled_class_hash
        result.push(compiled_class_hash.to_bytes_be());
    }

    result
}

fn usize_to_bytes(n: usize) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[..8].copy_from_slice(&n.to_le_bytes());
    bytes
}
