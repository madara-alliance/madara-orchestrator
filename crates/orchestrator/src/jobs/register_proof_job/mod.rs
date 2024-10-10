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
        tracing::info!(job_id = %internal_id, "Creating proof registration job");
        let job_item = JobItem {
            id: Uuid::new_v4(),
            internal_id: internal_id.clone(),
            job_type: JobType::ProofRegistration,
            status: JobStatus::Created,
            external_id: String::new().into(),
            // metadata must contain the blocks that have been included inside this proof
            // this will allow state update jobs to be created for each block
            metadata: metadata.clone(),
            version: 0,
            created_at: Utc::now().round_subsecs(0),
            updated_at: Utc::now().round_subsecs(0),
        };
        tracing::debug!(job_id = %internal_id, "Proof registration job created: {:?}", job_item);
        Ok(job_item)
    }

    #[tracing::instrument(fields(category = "proof_registry"), skip(self, _config))]
    async fn process_job(&self, _config: Arc<Config>, _job: &mut JobItem) -> Result<String, JobError> {
        tracing::info!(job_id = %_job.internal_id, "Processing proof registration job");
        // Get proof from storage and submit on chain for verification
        // We need to implement a generic trait for this to support multiple
        // base layers
        tracing::warn!(job_id = %_job.internal_id, "Proof registration job processing not implemented");
        todo!()
    }

    #[tracing::instrument(fields(category = "proof_registry"), skip(self, _config))]
    async fn verify_job(&self, _config: Arc<Config>, _job: &mut JobItem) -> Result<JobVerificationStatus, JobError> {
        tracing::info!(job_id = %_job.internal_id, "Verifying proof registration job");
        // verify that the proof transaction has been included on chain
        tracing::warn!(job_id = %_job.internal_id, "Proof registration job verification not implemented");
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
