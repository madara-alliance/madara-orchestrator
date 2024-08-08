use std::error::Error;

use async_trait::async_trait;

use crate::config::config;
use crate::jobs::constants::JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY;
use crate::jobs::create_job;
use crate::jobs::types::{JobItem, JobStatus, JobType};
use crate::workers::Worker;

pub struct UpdateStateWorker;

#[async_trait]
impl Worker for UpdateStateWorker {
    /// 1. Fetch the last successful state update job
    /// 2. Fetch all successful proving jobs covering blocks after the last state update
    /// 3. Create state updates for all the blocks that don't have a state update job
    async fn run_worker(&self) -> Result<(), Box<dyn Error>> {
        let config = config().await;
        let latest_successful_job =
            config.database().get_latest_job_by_type_and_status(JobType::StateTransition, JobStatus::Completed).await?;

        match latest_successful_job {
            Some(job) => {
                let latest_successful_job_internal_id = job.internal_id;

                let successful_proving_jobs = config
                    .database()
                    .get_jobs_after_internal_id_by_job_type(
                        JobType::ProofCreation,
                        JobStatus::Completed,
                        latest_successful_job_internal_id,
                    )
                    .await?;

                let mut metadata = job.metadata;
                metadata.insert(
                    JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY.to_string(),
                    Self::parse_job_items_into_block_number_list(successful_proving_jobs.clone()),
                );

                // Creating a single job for all the pending blocks.
                create_job(JobType::StateTransition, successful_proving_jobs[0].internal_id.clone(), metadata).await?;

                Ok(())
            }
            None => {
                log::info!("No successful state update jobs found");
                return Ok(());
            }
        }
    }
}

impl UpdateStateWorker {
    /// To parse the block numbers from the vector of jobs.
    fn parse_job_items_into_block_number_list(job_items: Vec<JobItem>) -> String {
        let mut block_number_string = String::from("");
        for job in job_items {
            block_number_string.push_str(&job.internal_id);
            block_number_string += ",";
        }
        block_number_string
    }
}
