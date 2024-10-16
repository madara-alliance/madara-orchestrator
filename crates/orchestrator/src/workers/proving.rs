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

pub struct ProvingWorker;

#[async_trait]
impl Worker for ProvingWorker {
    /// 1. Fetch all successful SNOS job runs that don't have a proving job
    /// 2. Create a proving job for each SNOS job run
    async fn run_worker(&self, config: Arc<Config>) -> Result<(), Box<dyn Error>> {
        let successful_snos_jobs = config
            .database()
            .get_jobs_without_successor(JobType::SnosRun, JobStatus::Completed, JobType::ProofCreation)
            .await?;

        for job in successful_snos_jobs {
            // If snos is not successful for this job then do not process it for proving
            if job.metadata.contains_key(SNOS_FAILED_JOB_TAG) {
                Self::create_job_in_db(config.clone(), job.internal_id).await?;
            } else {
                create_job(JobType::ProofCreation, job.internal_id.to_string(), job.metadata, config.clone()).await?;
            }
        }

        Ok(())
    }
}

impl ProvingWorker {
    async fn create_job_in_db(config: Arc<Config>, block_number: String) -> color_eyre::Result<()> {
        config
            .database()
            .create_job(JobItem {
                id: Default::default(),
                internal_id: block_number,
                job_type: JobType::ProofCreation,
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
