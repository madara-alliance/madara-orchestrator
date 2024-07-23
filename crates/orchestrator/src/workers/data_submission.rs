use crate::config::config;
use crate::jobs::create_job;
use crate::jobs::types::JobType;
use crate::workers::Worker;
use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;

pub struct DataSubmissionWorker;

#[async_trait]
impl Worker for DataSubmissionWorker {
    // 0. All ids are assumed to be block numbers.
    // 1. Fetch the latest completed Proving job.
    // 2. Fetch the latest DA job creation.
    // 3. Create jobs from after the lastest DA job already created till latest completed proving job.
    async fn run_worker(&self) -> Result<(), Box<dyn Error>> {

        // Return without doing anything if the worker is not enabled.
        if !self.is_worker_enabled().await? {
            return Ok(());
        }

        let config = config().await;

        // provides latest completed proof creation job id
        let latest_proven_job_id = config
            .database()
            .get_last_successful_job_by_type(JobType::ProofCreation)
            .await
            .unwrap()
            .map(|item| item.internal_id)
            .unwrap_or("0".to_string());

        // provides latest triggered data submission job id
        let latest_data_submission_job_id = config
            .database()
            .get_latest_job_by_type(JobType::DataSubmission)
            .await
            .unwrap()
            .map(|item| item.internal_id)
            .unwrap_or("0".to_string());

        let latest_data_submission_id: u64 = latest_data_submission_job_id.parse()?;
        let latest_proven_id: u64 = latest_proven_job_id.parse()?;

        // creating data submission jobs for latest blocks without pre-running data submission jobs jobs don't yet exist.
        for x in latest_data_submission_id + 1..latest_proven_id + 1 {
            create_job(JobType::DataSubmission, x.to_string(), HashMap::new()).await?;
        }

        Ok(())
    }
}
