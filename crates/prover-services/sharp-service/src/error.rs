use alloy::primitives::hex::FromHexError;
use color_eyre::eyre::eyre;
use gps_fact_checker::error::FactCheckerError;
use prover_client_interface::ProverClientError;
use reqwest::StatusCode;
use std::fmt;

// ====================================================
/// Wrapper Type for Other(<>) job type
#[derive(Debug)]
pub struct OtherError(color_eyre::eyre::Error);

impl fmt::Display for OtherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for OtherError {}

impl PartialEq for OtherError {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl From<color_eyre::eyre::Error> for OtherError {
    fn from(err: color_eyre::eyre::Error) -> Self {
        OtherError(err)
    }
}

impl From<String> for OtherError {
    fn from(error_string: String) -> Self {
        OtherError(eyre!(error_string))
    }
}
// ====================================================

#[derive(Debug, thiserror::Error)]
pub enum SharpError {
    #[error("Failed to to add SHARP job: {0}")]
    AddJobFailure(#[source] reqwest::Error),
    #[error("Failed to to get status of a SHARP job: {0}")]
    GetJobStatusFailure(#[source] reqwest::Error),
    #[error("Fact checker error: {0}")]
    FactChecker(#[from] FactCheckerError),
    #[error("SHARP service returned an error {0}")]
    SharpService(StatusCode),
    #[error("Failed to parse job key: {0}")]
    JobKeyParse(uuid::Error),
    #[error("Failed to parse fact: {0}")]
    FactParse(FromHexError),
    #[error("Failed to split task id into job key and fact")]
    TaskIdSplit,
    #[error("Failed to encode PIE")]
    PieEncode(#[source] snos::error::SnOsError),
    #[error("Failed to get url as path segment mut. URL is cannot-be-a-base.")]
    PathSegmentMutFailOnUrl,
    #[error("Other error: {0}")]
    Other(#[from] OtherError),
}

impl From<SharpError> for ProverClientError {
    fn from(value: SharpError) -> Self {
        Self::Internal(Box::new(value))
    }
}
