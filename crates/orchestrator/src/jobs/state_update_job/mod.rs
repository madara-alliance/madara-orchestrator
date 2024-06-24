use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::PathBuf;
use utils::collections::{has_dup, is_sorted};

use async_trait::async_trait;
use cairo_vm::Felt252;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use snos::io::output::StarknetOsOutput;
use starknet::providers::Provider;
use starknet_core::types::{BlockId, MaybePendingStateUpdate};
use uuid::Uuid;

use crate::config::Config;
use crate::jobs::constants::JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;

pub const METADATA_FETCH_FROM_TESTS: &str = "FETCH_FROM_TESTS";

lazy_static! {
    pub static ref CURRENT_PATH: PathBuf = std::env::current_dir().unwrap();
}

pub struct StateUpdateJob;
#[async_trait]
impl Job for StateUpdateJob {
    async fn create_job(
        &self,
        _config: &Config,
        internal_id: String,
        metadata: HashMap<String, String>,
    ) -> Result<JobItem> {
        Ok(JobItem {
            id: Uuid::new_v4(),
            internal_id,
            job_type: JobType::StateTransition,
            status: JobStatus::Created,
            external_id: String::new().into(),
            // metadata must contain the blocks for which state update will be performed
            // we don't do one job per state update as that makes nonce management complicated
            metadata,
            version: 0,
        })
    }

    async fn process_job(&self, config: &Config, job: &JobItem) -> Result<String> {
        // TODO: remove when SNOS is correctly stored in DB/S3
        // Test metadata to fetch the snos output from the test folder
        let fetch_from_tests = job.metadata.get(METADATA_FETCH_FROM_TESTS).map_or(false, |value| value == "TRUE");

        // Read the metadata to get the blocks for which state update will be performed.
        // We assume that blocks nbrs are formatted as follow: "2,3,4,5,6".
        let blocks_to_settle = job.metadata.get(JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY).ok_or_else(|| {
            eyre!("Block numbers to settle must be specified (state update job #{})", job.internal_id)
        })?;
        let block_numbers: Vec<u64> = self.parse_block_numbers(blocks_to_settle)?;

        self.validate_block_numbers(config, &block_numbers).await?;

        let mut last_tx_hash: Option<String> = None;
        for block_no in block_numbers.iter() {
            let snos = self.fetch_snos_for_block(*block_no, Some(fetch_from_tests)).await;
            last_tx_hash = Some(self.update_state_for_block(config, *block_no, snos, Some(fetch_from_tests)).await?);
        }

        if let Some(tx_hash) = last_tx_hash {
            Ok(tx_hash)
        } else {
            return Err(eyre!("No settlement TX executed (state update job #{})", job.internal_id));
        }
    }

    /// Verify that the proof transaction has been included on chain
    async fn verify_job(&self, config: &Config, job: &JobItem) -> Result<JobVerificationStatus> {
        // external_id corresponds to the last tx executed by the job
        let external_id: String = job.external_id.unwrap_string()?.into();
        let settlement_client = config.settlement_client();
        let inclusion_status = settlement_client.verify_inclusion(&external_id).await?;
        Ok(inclusion_status.into())
    }

    fn max_process_attempts(&self) -> u64 {
        1
    }

    fn max_verification_attempts(&self) -> u64 {
        1
    }

    fn verification_polling_delay_seconds(&self) -> u64 {
        60
    }
}

impl StateUpdateJob {
    /// Parse a list of blocks comma separated
    fn parse_block_numbers(&self, blocks_to_settle: &str) -> Result<Vec<u64>> {
        let sanitized_blocks = blocks_to_settle.replace(' ', "");
        let block_numbers: Vec<u64> = sanitized_blocks
            .split(',')
            .map(|block_no| block_no.parse::<u64>())
            .collect::<Result<Vec<u64>, _>>()
            .map_err(|e| eyre!("Block numbers to settle list is not correctly formatted: {e}"))?;
        Ok(block_numbers)
    }

    /// Validate that the list of block numbers to process is valid.
    async fn validate_block_numbers(&self, config: &Config, block_numbers: &[u64]) -> Result<()> {
        if block_numbers.is_empty() {
            return Err(eyre!("No block numbers found."));
        }
        if has_dup(block_numbers) {
            return Err(eyre!("Duplicated block numbers."));
        }
        if !is_sorted(block_numbers) {
            return Err(eyre!("Block numbers aren't sorted in increasing order."));
        }
        // Check for gap between the last settled block and the first block to settle
        let last_settled_block: u64 = config.settlement_client().get_last_settled_block().await?;
        if last_settled_block + 1 != block_numbers[0] {
            return Err(eyre!("Gap detected between the first block to settle and the last one settled."));
        }
        Ok(())
    }

    /// Update the state for the corresponding block using the settlement layer.
    async fn update_state_for_block(
        &self,
        config: &Config,
        block_no: u64,
        snos: StarknetOsOutput,
        fetch_from_tests: Option<bool>,
    ) -> Result<String> {
        let starknet_client = config.starknet_client();
        let settlement_client = config.settlement_client();
        let last_tx_hash_executed = if snos.use_kzg_da == Felt252::ZERO {
            let state_update = starknet_client.get_state_update(BlockId::Number(block_no)).await?;
            let _state_update = match state_update {
                MaybePendingStateUpdate::PendingUpdate(_) => {
                    return Err(eyre!("Block #{} - Cannot update state as it's still in pending state", block_no));
                }
                MaybePendingStateUpdate::Update(state_update) => state_update,
            };
            // TODO: how to build the required arguments?
            settlement_client.update_state_calldata(vec![], vec![], 0).await?
        } else if snos.use_kzg_da == Felt252::ONE {
            let kzg_proof = self.fetch_kzg_proof_for_block(block_no, fetch_from_tests).await;
            // TODO: Build the blob & the KZG proof & send them to update_state_blobs
            settlement_client.update_state_blobs(vec![], kzg_proof.into_bytes()).await?
        } else {
            return Err(eyre!("Block #{} - SNOS error, [use_kzg_da] should be either 0 or 1.", block_no));
        };
        Ok(last_tx_hash_executed)
    }

    /// Retrieves the SNOS output for the corresponding block.
    /// TODO: remove the fetch_from_tests argument once we have proper fetching (db/s3)
    async fn fetch_snos_for_block(&self, block_no: u64, fetch_from_tests: Option<bool>) -> StarknetOsOutput {
        let fetch_from_tests = fetch_from_tests.unwrap_or(true);
        match fetch_from_tests {
            true => {
                let snos_path =
                    CURRENT_PATH.join(format!("src/jobs/state_update_job/test_data/{}/snos_output.json", block_no));
                let snos_str = std::fs::read_to_string(snos_path).expect("Failed to read the SNOS json file");
                serde_json::from_str(&snos_str).expect("Failed to deserialize JSON into SNOS")
            }
            false => unimplemented!("can't fetch SNOS from DB/S3"),
        }
    }

    /// Retrieves the KZG Proof for the corresponding block.
    /// TODO: remove the fetch_from_tests argument once we have proper fetching (db/s3)
    async fn fetch_kzg_proof_for_block(&self, block_no: u64, fetch_from_tests: Option<bool>) -> String {
        let fetch_from_tests = fetch_from_tests.unwrap_or(true);
        match fetch_from_tests {
            true => {
                let kzg_path =
                    CURRENT_PATH.join(format!("src/jobs/state_update_job/test_data/{}/kzg_proof.txt", block_no));
                std::fs::read_to_string(kzg_path).expect("Failed to read the KZG txt file")
            }
            false => unimplemented!("can't fetch KZG Proof from DB/S3"),
        }
    }
}
