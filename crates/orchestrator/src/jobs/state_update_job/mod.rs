use std::collections::HashMap;

use async_trait::async_trait;
use cairo_vm::Felt252;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use snos::io::output::StarknetOsOutput;
use starknet::providers::Provider;
use starknet_core::types::{BlockId, MaybePendingStateUpdate};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::config::Config;
use crate::jobs::constants::JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;

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
        // Read the metadata to get the blocks for which state update will be performed.
        // We assume that blocks nbrs are formatted as follow: "2,3,4,5,6".
        let blocks_to_settle = job.metadata.get(JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY).ok_or_else(|| {
            eyre!(eyre!("Blocks number to settle must be specified (prover job #{})", job.internal_id))
        })?;
        let block_numbers: Vec<String> = blocks_to_settle.split(',').map(String::from).collect();

        // TODO: remove when SNOS is correctly stored in DB/S3
        // Test metadata to fetch the snos output from the test folder
        let fetch_snos_from_tests = job.metadata.get("FETCH_SNOS_FROM_TESTS").map_or(false, |value| value == "TRUE");

        // For each block, either update using calldata or blobs
        for block_no in block_numbers.iter() {
            let block_no = block_no.parse::<u64>()?;
            let snos = self.get_snos_for_block(block_no, Some(fetch_snos_from_tests)).await;
            self.update_state_for_block(config, block_no, snos).await?;
        }
        Ok(blocks_to_settle.to_string())
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
    async fn get_snos_for_block(&self, block_no: u64, fetch_snos_from_tests: Option<bool>) -> StarknetOsOutput {
        let fetch_from_tests = fetch_snos_from_tests.unwrap_or(false);
        match fetch_from_tests {
            true => {
                let file_path = format!("./test_data/{}/snos_output_block.json", block_no);
                if !Path::new(&file_path).exists() {
                    panic!("SNOS file not found for block number {}", block_no);
                }
                let mut file = File::open(file_path).await.expect("SNOS file not found");
                let mut contents = String::new();
                file.read_to_string(&mut contents).await.expect("Failed to read file");
                serde_json::from_str(&contents).expect("Failed to deserialize JSON into SNOS")
            }
            false => unimplemented!("fetch snos from DB/S3 not implemented yet"),
        }
    }

    async fn update_state_for_block(&self, config: &Config, block_no: u64, snos: StarknetOsOutput) -> Result<()> {
        let starknet_client = config.starknet_client();
        let settlement_client = config.settlement_client();
        if snos.use_kzg_da == Felt252::ZERO {
            let state_update = starknet_client.get_state_update(BlockId::Number(block_no)).await?;
            let state_update = match state_update {
                MaybePendingStateUpdate::PendingUpdate(_) => {
                    return Err(eyre!("Cannot update state for block {} as it's still in pending state", block_no));
                }
                MaybePendingStateUpdate::Update(state_update) => state_update,
            };
            let _state_diff = state_update.state_diff;
            settlement_client.update_state_calldata(vec![], vec![], 0).await?;
        } else if snos.use_kzg_da == Felt252::ONE {
            let _kzg_proof = String::from("something from s3/or txt_file");
            settlement_client.update_state_blobs(vec![], vec![]).await?;
        } else {
            panic!("uuuh");
        }
        Ok(())
    }
}
