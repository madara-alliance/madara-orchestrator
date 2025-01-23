use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommonMetadata {
    pub process_attempt_no: u64,
    pub process_retry_attempt_no: u64,
    pub verification_attempt_no: u64,
    pub verification_retry_attempt_no: u64,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateUpdateMetadata {
    // Required fields
    pub blocks_to_settle: Vec<u64>,
    pub fetch_from_test_data: Option<bool>,
    
    // Paths for data
    pub snos_output_paths: Option<Vec<String>>,
    pub program_output_paths: Option<Vec<String>>,
    pub blob_data_paths: Option<Vec<String>>,
    
    // State tracking
    pub last_failed_block_no: Option<String>,
    pub attempt_tx_hashes: HashMap<String, String>, // key: attempt_no, value: comma-separated tx hashes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvingMetadata {
    // Required fields
    pub block_number: u64,
    pub program_output_path: Option<String>,
    pub cairo_pie_path: Option<String>,
    
    // Proof related fields
    pub proof_path: Option<String>,
    pub verification_key_path: Option<String>,
    pub proof_parameters: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaMetadata {
    // Required fields
    pub block_number: u64,
    
    // DA specific fields
    pub blob_data_path: Option<String>,
    pub blob_commitment: Option<String>,
    pub blob_proof: Option<String>,
    pub tx_hash: Option<String>,
    pub blob_versioned_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum JobSpecificMetadata {
    Snos(SnosMetadata),
    StateUpdate(StateUpdateMetadata),
    Proving(ProvingMetadata),
    Da(DaMetadata),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetadata {
    pub common: CommonMetadata,
    pub specific: JobSpecificMetadata,
}
