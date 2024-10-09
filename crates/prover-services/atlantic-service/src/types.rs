use serde::{Deserialize, Serialize};
use starknet_os::sharp::InvalidReason;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AtlanticAddJobResponse {
    pub sharp_query_id: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AtlanticGetProofResponse {
    pub code: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)]
pub enum AtlanticJobStatus {
    #[default]
    RECIEVED,
    DONE,
    FAILED,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct AtlanticGetStatusResponse {
    #[serde(default)]
    pub status: AtlanticJobStatus,
    pub invalid_reason: Option<InvalidReason>,
    pub error_log: Option<String>,
}
