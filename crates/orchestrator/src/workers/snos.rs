use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use starknet::providers::Provider;
use utils::env_utils::get_env_var_or_default;

use crate::config::Config;
use crate::jobs::create_job;
use crate::jobs::types::JobType;
use crate::workers::Worker;

pub struct SnosWorker;

#[async_trait]
impl Worker for SnosWorker {
    /// 1. Fetch the latest completed block from the Starknet chain
    /// 2. Fetch the last block that had a SNOS job run.
    /// 3. Create SNOS run jobs for all the remaining blocks
    async fn run_worker(&self, config: Arc<Config>) -> Result<(), Box<dyn Error>> {
        tracing::trace!(log_type = "starting", category = "SnosWorker", "SnosWorker started.");

        let provider = config.starknet_client();
        let block_number_provider = &provider.block_number().await?;
        let latest_block_number =
            get_env_var_or_default("SNOS_LATEST_BLOCK", &block_number_provider.to_string()).parse::<u64>()?;
        tracing::debug!(latest_block_number = %latest_block_number, "Fetched latest block number from starknet");

        let latest_job_in_db = config.database().get_latest_job_by_type(JobType::SnosRun).await?;

        let latest_job_id = match latest_job_in_db {
            Some(job) => job.internal_id,
            None => "0".to_string(),
        };

        // To be used when testing in specific block range
        let block_start = get_env_var_or_default("SNOS_MINIMUM_START_BLOCK", &latest_job_id).parse::<u64>()?;

        for x in block_start..latest_block_number + 1 {
            match create_job(JobType::SnosRun, x.to_string(), HashMap::new(), config.clone()).await {
                Ok(_) => {}
                Err(e) => {
                    log::warn!("Failed to create job: {:?}", e);
                }
            }
        }
        tracing::trace!(log_type = "completed", category = "SnosWorker", "SnosWorker completed.");
        Ok(())
    }
}
