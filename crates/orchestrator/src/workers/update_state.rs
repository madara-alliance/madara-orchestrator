use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;

use crate::config::Config;
use crate::jobs::constants::JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY;
use crate::jobs::create_job;
use crate::jobs::types::{JobItem, JobStatus, JobType};
use crate::workers::Worker;

pub struct UpdateStateWorker;

#[async_trait]
impl Worker for UpdateStateWorker {
    async fn run_worker(&self, config: Arc<Config>) -> Result<(), Box<dyn Error>> {
        // TODO : fix this
        // We should look for LockedForProcessing job also.
        // Current assumption : no job will fail.
        tracing::info!(log_type = "starting", category = "UpdateStateWorker", "UpdateStateWorker started.");

        let latest_job = config.database().get_latest_job_by_type(JobType::StateTransition).await?;

        match latest_job {
            Some(job) => {
                if job.status == JobStatus::LockedForProcessing || job.status == JobStatus::PendingVerification {
                    log::warn!(
                        "⚠️  A job is already in processing or verification pending cannot create a new job. May \
                         cause unexpected state update issues."
                    );
                    return Ok(());
                }

                tracing::debug!(job_id = %job.id, "Found latest successful state transition job");
                let successful_da_jobs_without_successor = config
                    .database()
                    .get_jobs_without_successor(JobType::DataSubmission, JobStatus::Completed, JobType::StateTransition)
                    .await?;

                if successful_da_jobs_without_successor.is_empty() {
                    tracing::debug!("No new data submission jobs to process");
                    return Ok(());
                }

                tracing::debug!(
                    count = successful_da_jobs_without_successor.len(),
                    "Found data submission jobs without state transition"
                );

                let mut blocks_processed_in_last_job: Vec<u64> = job
                    .metadata
                    .get(JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY)
                    .unwrap()
                    .split(',')
                    .filter_map(|s| s.parse().ok())
                    .collect();
                blocks_processed_in_last_job.sort();

                log::warn!(">>>> blocks processed in last job : {:?}", blocks_processed_in_last_job);

                let last_block_processed_in_last_job =
                    blocks_processed_in_last_job[blocks_processed_in_last_job.len() - 1];

                let mut blocks_to_process: Vec<u64> = successful_da_jobs_without_successor
                    .iter()
                    .filter_map(|j| {
                        j.internal_id.parse::<u64>().ok().and_then(|internal_id| {
                            if internal_id > last_block_processed_in_last_job { Some(internal_id) } else { None }
                        })
                    })
                    .collect();
                blocks_to_process.sort();

                log::warn!(">>>> blocks to process : {:?}", blocks_to_process);

                let mut blocks_to_process_final = Vec::new();
                for block in blocks_to_process {
                    if !blocks_processed_in_last_job.contains(&block) {
                        blocks_to_process_final.push(block);
                    }
                }
                blocks_to_process_final.sort();

                log::warn!(">>>> blocks to process (final) : {:?}", blocks_to_process_final);

                if blocks_to_process_final.is_empty() {
                    return Ok(());
                }

                if blocks_to_process_final[0] != last_block_processed_in_last_job + 1 {
                    log::warn!("⚠️ Can't create job because of inconsistent blocks.");
                    return Ok(());
                }

                let blocks_to_process = Self::find_successive_blocks_in_vector(blocks_to_process_final.clone());

                // TODO : remove this after testing is done
                // Second check
                if !Self::check_blocks_consecutive(blocks_to_process.clone()) {
                    log::warn!(
                        "⚠️ Can't create job because of non consecutive blocks | blocks : {:?}",
                        blocks_to_process
                    );
                    return Ok(());
                }

                log::warn!(">>> Creating UpdateState job for blocks : {:?}", blocks_to_process);

                let mut metadata = HashMap::new();
                metadata.insert(
                    JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY.to_string(),
                    blocks_to_process.iter().map(|ele| ele.to_string()).collect::<Vec<String>>().join(","),
                );

                // Creating a single job for all the pending blocks.
                let new_job_id = successful_da_jobs_without_successor[0].internal_id.clone();
                match create_job(JobType::StateTransition, new_job_id.clone(), metadata, config.clone()).await {
                    Ok(_) => tracing::info!(job_id = %new_job_id, "Successfully created new state transition job"),
                    Err(e) => {
                        tracing::error!(job_id = %new_job_id, error = %e, "Failed to create new state transition job");
                        return Err(e.into());
                    }
                }

                tracing::info!(log_type = "completed", category = "UpdateStateWorker", "UpdateStateWorker completed.");
                Ok(())
            }
            None => {
                tracing::warn!("No previous state transition job found, fetching latest data submission job");
                // Getting latest DA job in case no latest state update job is present
                let latest_successful_jobs_without_successor = config
                    .database()
                    .get_jobs_without_successor(JobType::DataSubmission, JobStatus::Completed, JobType::StateTransition)
                    .await?;

                if latest_successful_jobs_without_successor.is_empty() {
                    tracing::debug!("No data submission jobs found to process");
                    return Ok(());
                }

                let job = latest_successful_jobs_without_successor[0].clone();
                let mut metadata = job.metadata;

                let blocks_to_settle =
                    Self::parse_job_items_into_block_number_list(latest_successful_jobs_without_successor.clone());
                metadata.insert(JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY.to_string(), blocks_to_settle.clone());

                tracing::trace!(job_id = %job.id, blocks_to_settle = %blocks_to_settle, "Prepared blocks to settle for initial state transition");

                match create_job(JobType::StateTransition, job.internal_id.clone(), metadata, config).await {
                    Ok(_) => tracing::info!(job_id = %job.id, "Successfully created initial state transition job"),
                    Err(e) => {
                        tracing::error!(job_id = %job.id, error = %e, "Failed to create initial state transition job");
                        return Err(e.into());
                    }
                }

                tracing::info!(log_type = "completed", category = "UpdateStateWorker", "UpdateStateWorker completed.");
                Ok(())
            }
        }
    }
}

impl UpdateStateWorker {
    /// To parse the block numbers from the vector of jobs.
    pub fn parse_job_items_into_block_number_list(job_items: Vec<JobItem>) -> String {
        job_items.iter().map(|j| j.internal_id.clone()).collect::<Vec<String>>().join(",")
    }

    /// To check if blocks sent to processing are consecutive
    pub fn check_blocks_consecutive(block_numbers: Vec<u64>) -> bool {
        if block_numbers.len() == 1 {
            return true;
        }

        let mut prev = block_numbers[0];
        for &current in block_numbers.iter().skip(1) {
            if current != prev + 1 {
                return false;
            }
            prev = current;
        }

        true
    }

    /// Gets the successive list of blocks from all the blocks processed in previous jobs
    /// Eg : input_vec : [1,2,3,4,7,8,9,11]
    /// We will take the first 4 block numbers and send it for processing
    pub fn find_successive_blocks_in_vector(block_numbers: Vec<u64>) -> Vec<u64> {
        if block_numbers.is_empty() {
            return Vec::new();
        }

        let mut blocks_to_process = Vec::new();
        let mut prev = block_numbers[0];
        blocks_to_process.push(prev);

        for &current in block_numbers.iter().skip(1) {
            if current == prev + 1 {
                blocks_to_process.push(current);
                prev = current;
            } else {
                break;
            }
        }

        blocks_to_process
    }
}

#[cfg(test)]
mod test_update_state_worker_utils {
    use chrono::{SubsecRound, Utc};
    use rstest::rstest;
    use uuid::Uuid;

    use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
    use crate::workers::update_state::UpdateStateWorker;

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
                created_at: Utc::now().round_subsecs(0),
                updated_at: Utc::now().round_subsecs(0),
            });
        }

        let block_string = UpdateStateWorker::parse_job_items_into_block_number_list(job_vec);
        assert_eq!(block_string, String::from("0,1,2"));
    }
}
