use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{SubsecRound, Utc};
use color_eyre::Result;
use uuid::Uuid;

use super::JobError;
use crate::config::Config;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;

pub struct RegisterProofJob;

#[async_trait]
impl Job for RegisterProofJob {
    #[tracing::instrument(fields(category = "proof_registry"), skip(self, _config, metadata))]
    async fn create_job(
        &self,
        _config: Arc<Config>,
        internal_id: String,
        metadata: HashMap<String, String>,
    ) -> Result<JobItem, JobError> {
        Ok(JobItem {
            id: Uuid::new_v4(),
            internal_id,
            job_type: JobType::ProofRegistration,
            status: JobStatus::Created,
            external_id: String::new().into(),
            // metadata must contain the blocks that have been included inside this proof
            // this will allow state update jobs to be created for each block
            metadata,
            version: 0,
            created_at: Utc::now().round_subsecs(0),
            updated_at: Utc::now().round_subsecs(0),
        })
    }

    #[tracing::instrument(fields(category = "proof_registry"), skip(self, _config))]
    async fn process_job(&self, _config: Arc<Config>, _job: &mut JobItem) -> Result<String, JobError> {
        // Get proof from storage and submit on chain for verification
        // We need to implement a generic trait for this to support multiple
        // base layers
        todo!()
    }

    #[tracing::instrument(fields(category = "proof_registry"), skip(self, _config))]
    async fn verify_job(&self, _config: Arc<Config>, _job: &mut JobItem) -> Result<JobVerificationStatus, JobError> {
        // verify that the proof transaction has been included on chain
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
