use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{SubsecRound, Utc};
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
use color_eyre::Result;
use prove_block::{prove_block, ProveBlockError};
use starknet_os::io::output::StarknetOsOutput;
use thiserror::Error;
use utils::env_utils::get_env_var_or_panic;
use uuid::Uuid;

use super::constants::JOB_METADATA_SNOS_BLOCK;
use super::{JobError, OtherError};
use crate::config::Config;
use crate::constants::{CAIRO_PIE_FILE_NAME, SNOS_OUTPUT_FILE_NAME};
use crate::data_storage::DataStorage;
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

    #[error("Could not serialize the Cairo Pie (state update job #{internal_id:?}): {message}")]
    CairoPieUnserializable { internal_id: String, message: String },
    #[error("Could not store the Cairo Pie (state update job #{internal_id:?}): {message}")]
    CairoPieUnstorable { internal_id: String, message: String },

    #[error("Could not serialize the Snos Output (state update job #{internal_id:?}): {message}")]
    SnosOutputUnserializable { internal_id: String, message: String },
    #[error("Could not store the Snos output (state update job #{internal_id:?}): {message}")]
    SnosOutputUnstorable { internal_id: String, message: String },

    #[error("Other error: {0}")]
    Other(#[from] OtherError),
}

// ProveBlockError does not implement PartialEq - can't use #[from]
impl From<ProveBlockError> for SnosError {
    // TODO(akhercha): error conversion
    fn from(_v: ProveBlockError) -> Self {
        Self::UnspecifiedBlockNumber { internal_id: String::from("XD") }
    }
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

    async fn process_job(&self, config: &Config, job: &mut JobItem) -> Result<String, JobError> {
        let block_number = self.get_block_number_from_metadata(job)?;
        let rpc_url = get_env_var_or_panic("MADARA_RPC_URL"); // should never panic at this point

        // TODO: Send directly the _config.starknet_client object instead of the rpc_url when snos allows it
        // let (cairo_pie, snos_output) =
        //     prove_block(block_number, &rpc_url, LayoutName::all_cairo).await.map_err(SnosError::from)?;

        // TODO: Remove this once snos problems are fixed
        let cairo_pie = todo!();
        let snos_output = todo!();

        self.store_snos_outputs(config.storage(), job.internal_id, block_number, cairo_pie, snos_output).await?;

        Ok(String::from("TODO: ID"))
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

    /// Stores the [CairoPie] and the [StarknetOsOutput] in our Data Storage.
    /// The paths will be:
    ///     - [block_number]/cairo_pie.json
    ///     - [block_number]/snos_output.json
    async fn store_snos_outputs(
        &self,
        data_storage: &dyn DataStorage,
        internal_id: String,
        block_number: u64,
        cairo_pie: CairoPie,
        snos_output: StarknetOsOutput,
    ) -> Result<(), SnosError> {
        let cairo_pie_key = format!("{block_number}/{CAIRO_PIE_FILE_NAME}");
        let cairo_pie_json = serde_json::to_vec(&cairo_pie).map_err(|e| SnosError::CairoPieUnserializable {
            internal_id: internal_id.clone(),
            message: e.to_string(),
        })?;
        data_storage
            .put_data(cairo_pie_json.into(), &cairo_pie_key)
            .await
            .map_err(|e| SnosError::CairoPieUnstorable { internal_id: internal_id.clone(), message: e.to_string() })?;

        let snos_output_key = format!("{block_number}/{SNOS_OUTPUT_FILE_NAME}");
        let snos_output_json = serde_json::to_vec(&snos_output).map_err(|e| SnosError::SnosOutputUnserializable {
            internal_id: internal_id.clone(),
            message: e.to_string(),
        })?;
        data_storage.put_data(snos_output_json.into(), &snos_output_key).await.map_err(|e| {
            SnosError::SnosOutputUnstorable { internal_id: internal_id.clone(), message: e.to_string() }
        })?;

        Ok(())
    }
}
