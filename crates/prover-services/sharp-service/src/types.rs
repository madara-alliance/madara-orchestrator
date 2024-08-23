use serde::{Deserialize, Serialize};
use snos::sharp::{CairoJobStatus, InvalidReason};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SharpAddJobResponse {
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SharpGetProofResponse {
    pub code: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize)]
pub struct SharpGetStatusResponse {
    pub status: CairoJobStatus,
    pub invalid_reason: Option<InvalidReason>,
    pub error_log: Option<String>,
    pub validation_done: Option<bool>,
}
