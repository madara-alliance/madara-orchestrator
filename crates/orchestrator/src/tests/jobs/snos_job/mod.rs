use std::collections::HashMap;

use rstest::rstest;

use super::super::common::default_job_item;
use crate::config::config;
use crate::jobs::snos_job::SnosJob;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;
use crate::tests::config::TestConfigBuilder;

#[rstest]
#[tokio::test]
async fn test_create_job() {
    TestConfigBuilder::new().build().await;
    let config = config().await;

    let job = SnosJob.create_job(&config, String::from("0"), HashMap::new()).await;
    assert!(job.is_ok());

    let job = job.unwrap();

    let job_type = job.job_type;
    assert_eq!(job_type, JobType::SnosRun, "job_type should be SnosRun");
    assert!(!(job.id.is_nil()), "id should not be nil");
    assert_eq!(job.status, JobStatus::Created, "status should be Created");
    assert_eq!(job.version, 0_i32, "version should be 0");
    assert_eq!(job.external_id.unwrap_string().unwrap(), String::new(), "external_id should be empty string");
}

#[rstest]
#[tokio::test]
async fn test_verify_job(#[from(default_job_item)] mut job_item: JobItem) {
    let config = config().await;
    let job_status = SnosJob.verify_job(&config, &mut job_item).await;
    // Should always be [Verified] for the moment.
    assert_eq!(job_status, Ok(JobVerificationStatus::Verified));
}
