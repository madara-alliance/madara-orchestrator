use rstest::*;

use std::collections::HashMap;

use super::super::common::init_config;

use crate::jobs::{
    state_update_job::StateUpdateJob,
    types::{JobStatus, JobType},
    Job,
};

#[rstest]
#[tokio::test]
async fn test_create_job() {
    let config = init_config(None, None, None, None, None, None).await;

    let job = StateUpdateJob.create_job(&config, String::from("0"), HashMap::default()).await;
    assert!(job.is_ok());

    let job = job.unwrap();

    let job_type = job.job_type;
    assert_eq!(job_type, JobType::StateTransition, "job_type should be StateTransition");
    assert!(!(job.id.is_nil()), "id should not be nil");
    assert_eq!(job.status, JobStatus::Created, "status should be Created");
    assert_eq!(job.version, 0_i32, "version should be 0");
    assert_eq!(job.external_id.unwrap_string().unwrap(), String::new(), "external_id should be empty string");
}

// #[rstest]
// #[tokio::test]
// async fn test_verify_job(#[from(default_job_item)] job_item: JobItem) {
//     let settlement_client = MockSettlementClient::new();

//     let config = init_config(None, None, None, None, None, Some(settlement_client)).await;
//     assert!(StateUpdateJob.verify_job(&config, &job_item).await.is_ok());
// }
