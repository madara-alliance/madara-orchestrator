use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use async_trait::async_trait;
use bytes::Bytes;
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
use chrono::{SubsecRound, Utc};
use color_eyre::Result;
use prove_block::{prove_block, ProveBlockError};
use starknet_os::io::output::StarknetOsOutput;
use tempfile::NamedTempFile;
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

        let (cairo_pie, snos_output) =
            prove_block(block_number, &rpc_url, LayoutName::all_cairo).await.map_err(SnosError::from)?;

        self.store(config.storage(), job.internal_id, block_number, cairo_pie, snos_output).await?;

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

    /// Stores the [CairoPie] and the [StarknetOsOutput] in the Data Storage.
    /// The paths will be:
    ///     - [block_number]/cairo_pie.zip
    ///     - [block_number]/snos_output.json
    async fn store(
        &self,
        data_storage: &dyn DataStorage,
        internal_id: String,
        block_number: u64,
        cairo_pie: CairoPie,
        snos_output: StarknetOsOutput,
    ) -> Result<(), SnosError> {
        let cairo_pie_key = format!("{block_number}/{CAIRO_PIE_FILE_NAME}");
        let cairo_pie_zip_bytes = self.cairo_pie_to_zip_bytes(cairo_pie).await.map_err(|e| {
            SnosError::CairoPieUnserializable { internal_id: internal_id.clone(), message: e.to_string() }
        })?;
        data_storage
            .put_data(cairo_pie_zip_bytes, &cairo_pie_key)
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

    /// Converts the [CairoPie] input as a zip file and returns it as [Bytes].
    async fn cairo_pie_to_zip_bytes(&self, cairo_pie: CairoPie) -> Result<Bytes> {
        let cairo_pie_zipfile = NamedTempFile::new()?;
        cairo_pie.write_zip_file(cairo_pie_zipfile.path())?;
        let cairo_pie_zip_bytes = self.tempfile_to_bytes(cairo_pie_zipfile).unwrap();
        Ok(cairo_pie_zip_bytes)
    }

    /// Converts a [NamedTempFile] to [Bytes].
    fn tempfile_to_bytes(&self, mut tmp_file: NamedTempFile) -> Result<Bytes> {
        let mut buffer = Vec::new();
        tmp_file.as_file_mut().read_to_end(&mut buffer)?;
        Ok(Bytes::from(buffer))
    }
}
