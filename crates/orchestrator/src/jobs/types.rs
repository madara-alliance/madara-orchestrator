use std::collections::HashMap;

// TODO: job types shouldn't depend on mongodb
use chrono::{DateTime, Utc};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use da_client_interface::DaVerificationStatus;
#[cfg(feature = "with_mongodb")]
use mongodb::bson::serde_helpers::{chrono_datetime_as_bson_datetime, uuid_1_as_binary};
use serde::{Deserialize, Serialize};
use settlement_client_interface::SettlementVerificationStatus;
use uuid::Uuid;

/// An external id.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum ExternalId {
    /// A string.
    String(Box<str>),
    /// A number.
    Number(usize),
}

impl From<String> for ExternalId {
    #[inline]
    fn from(value: String) -> Self {
        ExternalId::String(value.into_boxed_str())
    }
}

impl From<usize> for ExternalId {
    #[inline]
    fn from(value: usize) -> Self {
        ExternalId::Number(value)
    }
}

impl ExternalId {
    /// Unwraps the external id as a string.
    ///
    /// # Panics
    ///
    /// This function panics if the provided external id not a string.
    #[track_caller]
    #[inline]
    pub fn unwrap_string(&self) -> Result<&str> {
        match self {
            ExternalId::String(s) => Ok(s),
            _ => Err(unwrap_external_id_failed("string", self)),
        }
    }

    /// Unwraps the external id as a number.
    ///
    /// # Panics
    ///
    /// This function panics if the provided external id is not a number.
    #[track_caller]
    #[inline]
    #[allow(dead_code)] // temporarily unused (until the other pull request uses it)
    pub fn unwrap_number(&self) -> Result<usize> {
        match self {
            ExternalId::Number(n) => Ok(*n),
            _ => Err(unwrap_external_id_failed("number", self)),
        }
    }
}

/// Returns an error indicating that the provided external id coulnd't be unwrapped.
fn unwrap_external_id_failed(expected: &str, got: &ExternalId) -> color_eyre::eyre::Error {
    eyre!("wrong ExternalId type: expected {}, got {:?}", expected, got)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum JobType {
    /// Running SNOS for a block
    SnosRun,
    /// Submitting DA data to the DA layer
    DataSubmission,
    /// Getting a proof from the proving service
    ProofCreation,
    /// Verifying the proof on the base layer
    ProofRegistration,
    /// Updaing the state root on the base layer
    StateTransition,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, strum_macros::Display, Eq)]
pub enum JobStatus {
    /// An acknowledgement that the job has been received by the
    /// orchestrator and is waiting to be processed

    #[strum(to_string = "Created")]
    Created,
    /// Some system has taken a lock over the job for processing and no
    /// other system to process the job

    #[strum(to_string = "Locked for Processing")]
    LockedForProcessing,
    /// The job has been processed and is pending verification

    #[strum(to_string = "Pending Verification")]
    PendingVerification,
    /// The job has been processed and verified. No other actions needs to be taken

    #[strum(to_string = "Completed")]
    Completed,
    /// The job was processed but the was unable to be verified under the given time
    #[strum(to_string = "Verification Timeout")]
    VerificationTimeout,
    /// The job failed processing
    #[strum(to_string = "Verification Failed")]
    VerificationFailed,
    /// The job failed completing
    #[strum(to_string = "Failed")]
    Failed,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct JobItem {
    /// an uuid to identify a job
    #[cfg_attr(feature = "with_mongodb", serde(with = "uuid_1_as_binary"))]
    pub id: Uuid,
    /// a meaningful id used to track a job internally, ex: block_no, txn_hash
    pub internal_id: String,
    /// the type of job
    pub job_type: JobType,
    /// the status of the job
    pub status: JobStatus,
    /// external id to track the status of the job. for ex, txn hash for blob inclusion
    /// or job_id from SHARP
    pub external_id: ExternalId,
    /// additional field to store values related to the job
    pub metadata: HashMap<String, String>,
    /// helps to keep track of the version of the item for optimistic locking
    pub version: i32,
    /// timestamp when the job was created
    #[cfg_attr(feature = "with_mongodb", serde(with = "chrono_datetime_as_bson_datetime"))]
    pub created_at: DateTime<Utc>,
    /// timestamp when the job was last updated
    #[cfg_attr(feature = "with_mongodb", serde(with = "chrono_datetime_as_bson_datetime"))]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobVerificationStatus {
    #[allow(dead_code)]
    Pending,
    #[allow(dead_code)]
    Verified,
    #[allow(dead_code)]
    Rejected(String),
}

impl From<DaVerificationStatus> for JobVerificationStatus {
    fn from(status: DaVerificationStatus) -> Self {
        match status {
            DaVerificationStatus::Pending => JobVerificationStatus::Pending,
            DaVerificationStatus::Verified => JobVerificationStatus::Verified,
            DaVerificationStatus::Rejected(e) => JobVerificationStatus::Rejected(e),
        }
    }
}

impl From<SettlementVerificationStatus> for JobVerificationStatus {
    fn from(status: SettlementVerificationStatus) -> Self {
        match status {
            SettlementVerificationStatus::Pending => JobVerificationStatus::Pending,
            SettlementVerificationStatus::Verified => JobVerificationStatus::Verified,
            SettlementVerificationStatus::Rejected(e) => JobVerificationStatus::Rejected(e),
        }
    }
}
