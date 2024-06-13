use crate::config::config;
use crate::jobs::create_job;
use crate::jobs::types::JobType;
use crate::workers::Worker;
use async_trait::async_trait;
use starknet::providers::Provider;
use std::collections::HashMap;
use tracing::log;

pub struct SnosWorker;

#[async_trait]
impl Worker for SnosWorker {
    /// 1. Fetch the latest completed block from the Starknet chain
    /// 2. Fetch the last block that had a SNOS job run.
    /// 3. Create SNOS run jobs for all the remaining blocks
    // TEST : added config temporarily to test
    async fn run_worker(&self) {
        let config = config().await;
        let provider = config.starknet_client();
        let latest_block_number = provider.block_number().await.expect("Failed to fetch block number from rpc");
        let latest_block_processed_data = config
            .database()
            .get_latest_job_by_type(JobType::SnosRun)
            .await
            .unwrap()
            .map(|item| item.internal_id)
            .unwrap_or("0".to_string());

        let latest_block_processed: u64 =
            latest_block_processed_data.parse().expect("Failed to convert block number from JobItem into u64");

        let block_diff = latest_block_number - latest_block_processed;

        // if all blocks are processed
        if block_diff == 0 {
            return;
        }

        for x in latest_block_processed + 1..latest_block_number + 1 {
            create_job(JobType::SnosRun, x.to_string(), HashMap::new())
                .await
                .expect("Error : failed to create job for snos workers.");
        }

        log::info!("jobs created !!");
    }
}
