use std::collections::HashMap;

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
        // TODO(akhercha): assert that this is the way to use metadata + block nbrs to process
        let blocks_to_settle = job.metadata.get(JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY).ok_or_else(|| {
            eyre!(eyre!("Blocks number to settle must be specified (prover job #{})", job.internal_id))
        })?;
        // Assume that blocks nbrs are formatted "2,3,4,5,6" ?
        let block_numbers: Vec<String> = blocks_to_settle.split(',').map(String::from).collect();
        let starnet_client = config.starknet_client();
        let settlement_client = config.settlement_client();
        // For each block, get the program output (from the PIE?) and the
        for block_no in block_numbers.iter() {
            let block_no = block_no.parse::<u64>()?;
            // TODO: get snos for block n_o from somewhere ?_?
            let snos: StarknetOsOutput = StarknetOsOutput {
                initial_root: Felt252::ZERO,
                final_root: Felt252::ZERO,
                block_number: Felt252::ZERO,
                block_hash: Felt252::ZERO,
                starknet_os_config_hash: Felt252::ZERO,
                use_kzg_da: Felt252::ZERO,
                messages_to_l1: vec![Felt252::ZERO],
                messages_to_l2: vec![Felt252::ZERO],
                contracts: vec![],
                classes: Default::default(),
            };
            if snos.use_kzg_da == Felt252::ZERO {
                let state_update = starnet_client.get_state_update(BlockId::Number(block_no)).await?;
                let state_update = match state_update {
                    MaybePendingStateUpdate::PendingUpdate(_) => {
                        return Err(eyre!(
                            "Cannot process block {} for job id {} as it's still in pending state",
                            block_no,
                            job.id
                        ));
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
        }
        todo!()
    }

    /// Verify that the proof transaction has been included on chain
    async fn verify_job(&self, config: &Config, job: &JobItem) -> Result<JobVerificationStatus> {
        let external_id: String = job.external_id.unwrap_string()?.into();
        let settlement_client = config.settlement_client();
        // TODO(akhercha): verify_inclusion isn't implemented yet
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
