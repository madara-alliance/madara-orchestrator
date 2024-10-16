use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use starknet::providers::Provider;
use utils::env_utils::{get_env_var_or_default, get_env_var_or_panic};

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
        tracing::info!(log_type = "starting", category = "SnosWorker", "SnosWorker started.");

        let provider = config.starknet_client();
        println!(">>>> SNOS_LATEST_BLOCK : {:?}", get_env_var_or_panic("SNOS_LATEST_BLOCK"));
        let latest_block_number =
            get_env_var_or_default("SNOS_LATEST_BLOCK", &provider.block_number().await?.to_string()).parse::<u64>()?;
        tracing::debug!(latest_block_number = %latest_block_number, "Fetched latest block number from starknet");

        println!(">>>> Latest block number: {}", latest_block_number);

        // TODO : This needs to be optimized.
        // TODO : This is not scalable.
        let mut snos_jobs_in_db_block_numbers: Vec<u64> = config
            .database()
            .get_jobs_by_type(JobType::SnosRun)
            .await?
            .iter()
            .map(|job| job.internal_id.parse::<u64>().unwrap())
            .collect();
        snos_jobs_in_db_block_numbers.sort();

        // TODO : temp solution
        // Just for testing purposes
        let first_block = snos_jobs_in_db_block_numbers.first();
        let block = match first_block {
            Some(first_block) => *first_block,
            None => 0,
        };

        for x in block..latest_block_number + 1 {
            if !snos_jobs_in_db_block_numbers.contains(&x) {
                create_job(JobType::SnosRun, x.to_string(), HashMap::new(), config.clone()).await?;
            }
        }
        tracing::info!(log_type = "completed", category = "SnosWorker", "SnosWorker completed.");
        Ok(())
    }
}