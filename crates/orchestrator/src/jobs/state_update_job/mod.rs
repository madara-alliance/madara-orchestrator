use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::PathBuf;

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
            eyre!("Blocks number to settle must be specified (state update job #{})", job.internal_id)
        })?;
        let block_numbers: Vec<u64> = blocks_to_settle
            .split(',')
            .map(|block_no| block_no.parse::<u64>())
            .collect::<Result<Vec<u64>, _>>()
            .map_err(|_| eyre!("Block numbers to settle list is not correctly formatted."))?;

        // Run validations on the block_numbers;
        // i.e: check that there's no overlapping with current block
        // Use starknet provider to call the stateBlockNumber of the core contract
        self.validate_block_numbers(config, &block_numbers).await?;

        // For each block, either update using calldata or blobs
        for block_no in block_numbers.iter() {
            let snos = self.fetch_snos_for_block(*block_no, Some(fetch_from_tests)).await;
            self.update_state_for_block(config, *block_no, snos, Some(fetch_from_tests)).await?;
        }
        Ok("task_id".to_string())
    }

    /// Verify that the proof transaction has been included on chain
    async fn verify_job(&self, config: &Config, job: &JobItem) -> Result<JobVerificationStatus> {
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
    /// Validate that the list of block numbers to process is valid.
    /// Valid in this context means:
    /// - that there is no gap between the last block settled and the first block to settle,
    /// - that the all the specified blocks are greater than the last settled block.
    async fn validate_block_numbers(&self, config: &Config, block_numbers: &[u64]) -> Result<()> {
        let last_settled_block: u64 = config.settlement_client().get_last_settled_block().await?;
        if last_settled_block + 1 != *block_numbers.first().expect("block numbers list is empty") {
            return Err(eyre!("Gap detected between the first block to settle and the last one settled."));
        }
        let valid_count = block_numbers.iter().filter(|&&block| block > last_settled_block).count();
        if valid_count != block_numbers.len() {
            return Err(eyre!("Invalid blocks."));
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
    ) -> Result<()> {
        let starknet_client = config.starknet_client();
        let settlement_client = config.settlement_client();
        if snos.use_kzg_da == Felt252::ZERO {
            let state_update = starknet_client.get_state_update(BlockId::Number(block_no)).await?;
            let _state_update = match state_update {
                MaybePendingStateUpdate::PendingUpdate(_) => {
                    return Err(eyre!("Cannot update state for block {} as it's still in pending state", block_no));
                }
                MaybePendingStateUpdate::Update(state_update) => state_update,
            };
            // TODO: how to build the required arguments?
            settlement_client.update_state_calldata(vec![], vec![], 0).await?;
        } else if snos.use_kzg_da == Felt252::ONE {
            let kzg_proof = self.fetch_kzg_proof_for_block(block_no, fetch_from_tests).await;
            // TODO: how to build program output?
            settlement_client.update_state_blobs(vec![], kzg_proof.into_bytes()).await?;
        } else {
            panic!("SNOS error: [use_kzg_da] should be either 0 or 1.");
        }
        Ok(())
    }

    /// Retrieves the SNOS output for the corresponding block.
    /// TODO: remove the fetch_from_tests argument once we have proper fetching (db/s3)
    async fn fetch_snos_for_block(&self, block_no: u64, fetch_from_tests: Option<bool>) -> StarknetOsOutput {
        let fetch_from_tests = fetch_from_tests.unwrap_or(false);
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
        let fetch_from_tests = fetch_from_tests.unwrap_or(false);
        match fetch_from_tests {
            true => {
                let kzg_path =
                    CURRENT_PATH.join(format!("src/jobs/state_update_job/test_data/{}/kzg_proof.txt", block_no));
                std::fs::read_to_string(&kzg_path).expect("Failed to read the KZG txt file")
            }
            false => unimplemented!("can't fetch KZG Proof from DB/S3"),
        }
    }
}
