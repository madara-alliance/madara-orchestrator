use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{SubsecRound, Utc};
use color_eyre::Result;
use uuid::Uuid;

use crate::config::Config;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;

use super::JobError;

pub struct SnosJob;

#[async_trait]
impl Job for SnosJob {
    #[tracing::instrument(skip(self, _config))]
    async fn create_job(
        &self,
        _config: Arc<Config>,
        internal_id: String,
        metadata: HashMap<String, String>,
    ) -> Result<JobItem, JobError> {
        Ok(JobItem {
            id: Uuid::new_v4(),
            internal_id,
            job_type: JobType::SnosRun,
            status: JobStatus::Created,
            external_id: String::new().into(),
            metadata,
            version: 0,
            created_at: Utc::now().round_subsecs(0),
            updated_at: Utc::now().round_subsecs(0),
        })
    }

    async fn process_job(&self, _config: Arc<Config>, _job: &mut JobItem) -> Result<String, JobError> {
        // 1. Fetch SNOS input data from Madara
        // 2. Import SNOS in Rust and execute it with the input data
        // 3. Store the received PIE in DB
        todo!()
    }

    async fn verify_job(&self, _config: Arc<Config>, _job: &mut JobItem) -> Result<JobVerificationStatus, JobError> {
        // No need for verification as of now. If we later on decide to outsource SNOS run
        // to another service, verify_job can be used to poll on the status of the job
        todo!()
    }

    fn max_process_attempts(&self) -> u64 {
        todo!()
    }

    fn max_verification_attempts(&self) -> u64 {
        todo!()
    }

    fn verification_polling_delay_seconds(&self) -> u64 {
        todo!()
    }
}
