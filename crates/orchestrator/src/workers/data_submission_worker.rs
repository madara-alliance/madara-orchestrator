use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{SubsecRound, Utc};

use crate::config::Config;
use crate::jobs::create_job;
use crate::jobs::snos_job::SNOS_FAILED_JOB_TAG;
use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
use crate::workers::Worker;

pub struct DataSubmissionWorker;

#[async_trait]
impl Worker for DataSubmissionWorker {
    // 0. All ids are assumed to be block numbers.
    // 1. Fetch the latest completed Proving job.
    // 2. Fetch the latest DA job creation.
    // 3. Create jobs from after the lastest DA job already created till latest completed proving job.
    async fn run_worker(&self, config: Arc<Config>) -> Result<(), Box<dyn Error>> {
        // provides latest completed proof creation job id
        let latest_proven_job_id = config
            .database()
            .get_latest_job_by_type_and_status(JobType::ProofCreation, JobStatus::Completed)
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

        // creating data submission jobs for latest blocks that don't have existing data submission jobs
        // yet.
        for new_job_id in latest_data_submission_id + 1..latest_proven_id + 1 {
            let job = config
                .database()
                .get_job_by_internal_id_and_type(&new_job_id.to_string(), &JobType::ProofCreation)
                .await?;
            if job.is_some() {
                // Adding completed status job in db if snos failed on this block
                if job.unwrap().metadata.contains_key(SNOS_FAILED_JOB_TAG) {
                    Self::create_job_in_db(config.clone(), new_job_id.to_string()).await?;
                } else {
                    create_job(JobType::DataSubmission, new_job_id.to_string(), HashMap::new(), config.clone()).await?;
                }
            }
        }

        Ok(())
    }
}

impl DataSubmissionWorker {
    async fn create_job_in_db(config: Arc<Config>, block_number: String) -> color_eyre::Result<()> {
        config
            .database()
            .create_job(JobItem {
                id: Default::default(),
                internal_id: block_number,
                job_type: JobType::DataSubmission,
                status: JobStatus::Completed,
                external_id: ExternalId::Number(0),
                metadata: HashMap::from([(SNOS_FAILED_JOB_TAG.into(), "1".into())]),
                version: 3,
                created_at: Utc::now().round_subsecs(0),
                updated_at: Utc::now().round_subsecs(0),
            })
            .await?;

        Ok(())
    }
}
