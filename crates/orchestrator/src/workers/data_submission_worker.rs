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
    // 1. Fetch the latest completed Proving jobs without Data Submission jobs as successor jobs
    // 2. Create jobs.
    async fn run_worker(&self, config: Arc<Config>) -> Result<(), Box<dyn Error>> {
        let successful_proving_jobs = config
            .database()
            .get_jobs_without_successor(JobType::ProofCreation, JobStatus::Completed, JobType::DataSubmission)
            .await?;

        for job in successful_proving_jobs {
            if job.metadata.contains_key(SNOS_FAILED_JOB_TAG) {
                Self::create_job_in_db(config.clone(), job.internal_id).await?;
            } else {
                create_job(JobType::DataSubmission, job.internal_id, HashMap::new(), config.clone()).await?;
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
