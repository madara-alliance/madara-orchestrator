use chrono::{DateTime, Utc};
use color_eyre::eyre;
use color_eyre::eyre::eyre;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CommonMetadata {
    pub process_attempt_no: u64,
    pub process_retry_attempt_no: u64,
    pub verification_attempt_no: u64,
    pub verification_retry_attempt_no: u64,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub process_completed_at: Option<DateTime<Utc>>,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub verification_completed_at: Option<DateTime<Utc>>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnosMetadata {
    // Required fields
    pub block_number: u64,
    pub full_output: bool,

    // Optional fields populated during processing
    pub cairo_pie_path: Option<String>,
    pub snos_output_path: Option<String>,
    pub program_output_path: Option<String>,
    pub snos_fact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateUpdateMetadata {
    // Required fields
    pub blocks_to_settle: Vec<u64>,
    // Paths for data
    pub snos_output_paths: Vec<String>,
    pub program_output_paths: Vec<String>,
    pub blob_data_paths: Vec<String>,

    // State tracking
    pub last_failed_block_no: Option<u64>,
    pub tx_hashes: Vec<String>, // key: attempt_no, value: comma-separated tx hashes
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProvingInputType {
    Proof(String),
    CairoPie(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvingMetadata {
    // Required fields
    pub block_number: u64,
    pub input_path: Option<ProvingInputType>,

    pub ensure_on_chain_registration: Option<String>,
    pub download_proof: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaMetadata {
    // Required fields
    pub block_number: u64,

    // DA specific fields
    pub blob_data_path: Option<String>,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum JobSpecificMetadata {
    Snos(SnosMetadata),
    StateUpdate(StateUpdateMetadata),
    Proving(ProvingMetadata),
    Da(DaMetadata),
}

impl TryInto<SnosMetadata> for JobSpecificMetadata {
    type Error = eyre::Error;

    fn try_into(self) -> Result<SnosMetadata, Self::Error> {
        match self {
            JobSpecificMetadata::Snos(metadata) => Ok(metadata.clone()),
            _ => Err(eyre!("Invalid metadata type: expected SNOS metadata")),
        }
    }
}

impl TryInto<ProvingMetadata> for JobSpecificMetadata {
    type Error = eyre::Error;

    fn try_into(self) -> Result<ProvingMetadata, Self::Error> {
        match self {
            JobSpecificMetadata::Proving(metadata) => Ok(metadata.clone()),
            _ => Err(eyre!("Invalid metadata type: expected Proving metadata")),
        }
    }
}

impl TryInto<DaMetadata> for JobSpecificMetadata {
    type Error = eyre::Error;

    fn try_into(self) -> Result<DaMetadata, Self::Error> {
        match self {
            JobSpecificMetadata::Da(metadata) => Ok(metadata.clone()),
            _ => Err(eyre!("Invalid metadata type: expected DA metadata")),
        }
    }
}

impl TryInto<StateUpdateMetadata> for JobSpecificMetadata {
    type Error = eyre::Error;

    fn try_into(self) -> Result<StateUpdateMetadata, Self::Error> {
        match self {
            JobSpecificMetadata::StateUpdate(metadata) => Ok(metadata.clone()),
            _ => Err(eyre!("Invalid metadata type: expected State Update metadata")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobMetadata {
    pub common: CommonMetadata,
    pub specific: JobSpecificMetadata,
}
