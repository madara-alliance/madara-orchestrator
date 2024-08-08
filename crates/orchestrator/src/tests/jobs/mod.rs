use rstest::rstest;

use crate::config::config;
use crate::jobs::handle_job_failure;
use crate::jobs::types::JobType;
use crate::{jobs::types::JobStatus, tests::config::TestConfigBuilder};

use super::database::build_job_item;

#[cfg(test)]
pub mod da_job;

#[cfg(test)]
pub mod proving_job;

#[cfg(test)]
pub mod state_update_job;

#[rstest]
#[tokio::test]
async fn create_job_fails_job_already_exists() {
    todo!()
}

#[rstest]
#[tokio::test]
async fn create_job_fails_works_new_job() {
    todo!()
}

#[rstest]
#[case(JobType::SnosRun, JobStatus::Failed)]
#[tokio::test]
async fn handle_job_failure_with_failed_job_status_works(#[case] job_type: JobType, #[case] job_status: JobStatus) {
    TestConfigBuilder::new().build().await;
    let config = config().await;
    let database_client = config.database();
    let internal_id = 1;

    // create a job, with already available "last_job_status"
    let mut job_expected = build_job_item(job_type.clone(), job_status.clone(), internal_id);
    let mut job_metadata = job_expected.metadata.clone();
    job_metadata.insert("last_job_status".to_string(), JobStatus::PendingVerification.to_string());
    job_expected.metadata = job_metadata.clone();

    let job_id = job_expected.id;

    // feeding the job to DB
    database_client.create_job(job_expected.clone()).await.unwrap();

    // calling handle_job_failure
    handle_job_failure(job_id).await.expect("handle_job_failure failed to run");

    let job_fetched = config.database().get_job_by_id(job_id).await.expect("Unable to fetch Job Data").unwrap();

    assert_eq!(job_fetched, job_expected);
}

#[rstest]
#[case::pending_verification(JobType::SnosRun, JobStatus::PendingVerification)]
#[case::verification_timout(JobType::SnosRun, JobStatus::VerificationTimeout)]
#[tokio::test]
async fn handle_job_failure_with_correct_job_status_works(#[case] job_type: JobType, #[case] job_status: JobStatus) {
    TestConfigBuilder::new().build().await;
    let config = config().await;
    let database_client = config.database();
    let internal_id = 1;

    // create a job
    let job = build_job_item(job_type.clone(), job_status.clone(), internal_id);
    let job_id = job.id;

    // feeding the job to DB
    database_client.create_job(job.clone()).await.unwrap();

    // calling handle_job_failure
    handle_job_failure(job_id).await.expect("handle_job_failure failed to run");

    let job_fetched = config.database().get_job_by_id(job_id).await.expect("Unable to fetch Job Data").unwrap();

    // creating expected output
    let mut job_expected = job.clone();
    let mut job_metadata = job_expected.metadata.clone();
    job_metadata.insert("last_job_status".to_string(), job_status.to_string());
    job_expected.metadata = job_metadata.clone();
    job_expected.status = JobStatus::Failed;

    assert_eq!(job_fetched, job_expected);
}

#[rstest]
#[case(JobType::DataSubmission)]
#[tokio::test]
async fn handle_job_failure_job_status_completed_works(#[case] job_type: JobType) {
    let job_status = JobStatus::Completed;

    TestConfigBuilder::new().build().await;
    let config = config().await;
    let database_client = config.database();
    let internal_id = 1;

    // create a job
    let job_expected = build_job_item(job_type.clone(), job_status.clone(), internal_id);
    let job_id = job_expected.id;

    // feeding the job to DB
    database_client.create_job(job_expected.clone()).await.unwrap();

    // calling handle_job_failure
    handle_job_failure(job_id).await.expect("Test call to handle_job_failure should have passed.");

    // The completed job status on db is untouched.
    let job_fetched = config.database().get_job_by_id(job_id).await.expect("Unable to fetch Job Data").unwrap();

    assert_eq!(job_fetched, job_expected);
}
