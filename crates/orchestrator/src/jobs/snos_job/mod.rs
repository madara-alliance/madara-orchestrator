use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{SubsecRound, Utc};
use color_eyre::Result;
use thiserror::Error;
use uuid::Uuid;

use super::constants::JOB_METADATA_SNOS_BLOCK;
use super::{JobError, OtherError};
use crate::config::Config;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;

#[derive(Error, Debug, PartialEq)]
pub enum SnosError {
    #[error("Block numbers to settle must be specified (state update job #{internal_id:?})")]
    UnspecifiedBlockNumber { internal_id: String },
    #[error("No block numbers found (state update job #{internal_id:?})")]
    BlockNumberNotFound { internal_id: String },
    #[error("Invalid specified block number \"{block_number:?}\" (state update job #{internal_id:?})")]
    InvalidBlockNumber { internal_id: String, block_number: String },

    #[error("Other error: {0}")]
    Other(#[from] OtherError),
}

pub struct SnosJob;

#[async_trait]
impl Job for SnosJob {
    async fn create_job(
        &self,
        _config: &Config,
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

    async fn process_job(&self, _config: &Config, job: &mut JobItem) -> Result<String, JobError> {
        // 0. Get block number from metadata
        let _block_number = self.get_block_number_from_metadata(job)?;

        // 1. Build the required inputs for snos::prove_block

        // 2. Run snos::prove_block

        // 3. Store stuff

        todo!()
    }

    async fn verify_job(&self, _config: &Config, _job: &mut JobItem) -> Result<JobVerificationStatus, JobError> {
        // No need for verification as of now. If we later on decide to outsource SNOS run
        // to another service, verify_job can be used to poll on the status of the job
        Ok(JobVerificationStatus::Verified)
    }

    fn max_process_attempts(&self) -> u64 {
        1
    }

    fn max_verification_attempts(&self) -> u64 {
        1
    }

    fn verification_polling_delay_seconds(&self) -> u64 {
        1
    }
}

impl SnosJob {
    /// Get the block number that needs to be run with SNOS for the current
    /// job.
    fn get_block_number_from_metadata(&self, job: &JobItem) -> Result<u64, SnosError> {
        let block_number: u64 = job
            .metadata
            .get(JOB_METADATA_SNOS_BLOCK)
            .ok_or(SnosError::UnspecifiedBlockNumber { internal_id: job.internal_id.clone() })?
            .parse()
            .map_err(|_| SnosError::InvalidBlockNumber {
                internal_id: job.internal_id.clone(),
                block_number: job.metadata[JOB_METADATA_SNOS_BLOCK].clone(),
            })?;

        Ok(block_number)
    }
}
