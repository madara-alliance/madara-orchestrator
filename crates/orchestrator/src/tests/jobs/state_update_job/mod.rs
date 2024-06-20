use rstest::*;
use starknet::core::types::StateUpdate;

use std::collections::HashMap;

use httpmock::prelude::*;
use serde_json::json;

use super::super::common::{
    constants::{ETHEREUM_MAX_BLOB_PER_TXN, ETHEREUM_MAX_BYTES_PER_BLOB},
    default_job_item, init_config,
};
use starknet_core::types::{FieldElement, MaybePendingStateUpdate, StateDiff};
use uuid::Uuid;

use crate::jobs::types::ExternalId;
use crate::jobs::{
    state_update_job::StateUpdateJob,
    types::{JobItem, JobStatus, JobType},
    Job,
};
use da_client_interface::{DaVerificationStatus, MockDaClient};

#[rstest]
#[tokio::test]
async fn test_create_job() {
    let config = init_config(None, None, None, None).await;

    let job = StateUpdateJob.create_job(&config, String::from("0"), HashMap::default()).await;
    assert!(job.is_ok());

    let job = job.unwrap();

    let job_type = job.job_type;
    assert_eq!(job_type, JobType::ProofRegistration, "job_type should be ProofRegistration");
    assert!(!(job.id.is_nil()), "id should not be nil");
    assert_eq!(job.status, JobStatus::Created, "status should be Created");
    assert_eq!(job.version, 0_i32, "version should be 0");
    assert_eq!(job.external_id.unwrap_string().unwrap(), String::new(), "external_id should be empty string");
}
