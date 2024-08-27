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

        log::info!("[UpdateStateWorker] latest_successful_job : {:?}", latest_successful_job);

        match latest_successful_job {
            Some(job) => {
                let latest_successful_job_internal_id = job.internal_id;

                let successful_da_jobs = config
                    .database()
                    .get_jobs_after_internal_id_by_job_type(
                        JobType::DataSubmission,
                        JobStatus::Completed,
                        latest_successful_job_internal_id,
                    )
                    .await?;

                let successful_da_jobs_without_successor = config
                    .database()
                    .get_jobs_without_successor(JobType::DataSubmission, JobStatus::Completed, JobType::StateTransition)
                    .await?;

                let result_vector = intersection(&successful_da_jobs, &successful_da_jobs_without_successor);
                log::info!("[UpdateStateWorker] result_vector : {:?}", result_vector);

                if result_vector.len() == 0 {
                    log::info!("[UpdateStateWorker] not creating job.......!!!");
                    return Ok(());
                }

                let mut metadata = job.metadata;
                metadata.insert(
                    JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY.to_string(),
                    Self::parse_job_items_into_block_number_list(result_vector.clone()),
                );

                log::info!("[UpdateStateWorker] creating job.......!!!");

                // Creating a single job for all the pending blocks.
                create_job(JobType::StateTransition, result_vector[0].internal_id.clone(), metadata).await?;

                Ok(())
            }
            None => {
                // Getting latest DA job in case no latest state update job is present
                let latest_successful_jobs_without_successor = config
                    .database()
                    .get_jobs_without_successor(JobType::DataSubmission, JobStatus::Completed, JobType::StateTransition)
                    .await?;

                log::info!(
                    "[UpdateStateWorker] latest_successful_jobs_without_successor : {:?}",
                    latest_successful_jobs_without_successor
                );

                let job = latest_successful_jobs_without_successor[0].clone();
                log::info!("[UpdateStateWorker] latest_successful_jobs_without_successor : job : {:?}", job);
                let mut metadata = job.metadata;

                metadata.insert(
                    JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY.to_string(),
                    Self::parse_job_items_into_block_number_list(latest_successful_jobs_without_successor.clone()),
                );

                log::info!("[UpdateStateWorker] creating job.......!!!");
                create_job(JobType::StateTransition, job.internal_id, metadata).await?;

                return Ok(());
            }
        }
    }
}

impl UpdateStateWorker {
    /// To parse the block numbers from the vector of jobs.
    pub fn parse_job_items_into_block_number_list(job_items: Vec<JobItem>) -> String {
        job_items.iter().map(|j| j.internal_id.clone()).collect::<Vec<String>>().join(",")
    }
}

fn intersection<T: Eq + Clone>(vec1: &Vec<T>, vec2: &Vec<T>) -> Vec<T> {
    vec1.iter().filter(|&item| vec2.contains(item)).cloned().collect()
}

#[cfg(test)]
mod test_update_state_worker_utils {
    use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
    use crate::workers::update_state::UpdateStateWorker;
    use rstest::rstest;
    use uuid::Uuid;

    #[rstest]
    fn test_parse_job_items_into_block_number_list() {
        let mut job_vec = Vec::new();
        for i in 0..3 {
            job_vec.push(JobItem {
                id: Uuid::new_v4(),
                internal_id: i.to_string(),
                job_type: JobType::ProofCreation,
                status: JobStatus::Completed,
                external_id: ExternalId::Number(0),
                metadata: Default::default(),
                version: 0,
            });
        }

        let block_string = UpdateStateWorker::parse_job_items_into_block_number_list(job_vec);
        assert_eq!(block_string, String::from("0,1,2"));
    }
}
