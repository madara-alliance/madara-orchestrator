use super::database::build_job_item;
use crate::config::config;
use crate::jobs::handle_job_failure;
use crate::jobs::types::JobType;
use crate::{jobs::types::JobStatus, tests::config::TestConfigBuilder};
use rstest::rstest;
use std::str::FromStr;
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

impl FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Created" => Ok(JobStatus::Created),
            "LockedForProcessing" => Ok(JobStatus::LockedForProcessing),
            "PendingVerification" => Ok(JobStatus::PendingVerification),
            "Completed" => Ok(JobStatus::Completed),
            "VerificationTimeout" => Ok(JobStatus::VerificationTimeout),
            "VerificationFailed" => Ok(JobStatus::VerificationFailed),
            "Failed" => Ok(JobStatus::Failed),
            _ => Err(format!("Invalid job status: {}", s)),
        }
    }
}

impl FromStr for JobType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "SnosRun" => Ok(JobType::SnosRun),
            "DataSubmission" => Ok(JobType::DataSubmission),
            "ProofCreation" => Ok(JobType::ProofCreation),
            "ProofRegistration" => Ok(JobType::ProofRegistration),
            "StateTransition" => Ok(JobType::StateTransition),
            _ => Err(format!("Invalid job type: {}", s)),
        }
    }
}

#[rstest]
#[case("SnosRun", "PendingVerification")]
#[case("DataSubmission", "Failed")]
#[tokio::test]
async fn handle_job_failure_job_status_typical_works(#[case] job_type: JobType, #[case] job_status: JobStatus) {
    TestConfigBuilder::new().build().await;
    let internal_id = 1;

    let config = config().await;
    let database_client = config.database();

    // create a job
    let mut job = build_job_item(job_type.clone(), job_status.clone(), internal_id);
    let job_id = job.id;

    // if test case is for Failure, add last_job_status to job's metadata
    if job_status == JobStatus::Failed {
        let mut metadata = job.metadata.clone();
        metadata.insert("last_job_status".to_string(), "VerificationTimeout".to_string());
        job.metadata = metadata;
    }

    // feeding the job to DB
    database_client.create_job(job.clone()).await.unwrap();

    // calling handle_job_failure
    let response = handle_job_failure(job_id).await;

    match response {
        Ok(()) => {
            // check job in db
            let job = config.database().get_job_by_id(job_id).await.expect("Unable to fetch Job Data");

            if let Some(job_item) = job {
                // check if job status is Failure
                assert_eq!(job_item.status, JobStatus::Failed);
                // check if job metadata has `last_job_status`
                assert!(job_item.metadata.get("last_job_status").is_some());
                println!("Handle Job Failure for ID {} was handled successfully", job_id);
            }
        }
        Err(err) => {
            panic!("Test case should have passed: {} ", err);
        }
    }
}

#[rstest]
// code should panic here, how can completed move to dl queue ?
#[case("DataSubmission")]
#[tokio::test]
async fn handle_job_failure__job_status_completed_works(#[case] job_type: JobType) {
    let job_status = JobStatus::Completed;
    TestConfigBuilder::new().build().await;
    let internal_id = 1;

    let config = config().await;
    let database_client = config.database();

    // create a job
    let job = build_job_item(job_type.clone(), job_status.clone(), internal_id);
    let job_id = job.id;

    // feeding the job to DB
    database_client.create_job(job.clone()).await.unwrap();

    // calling handle_job_failure
    handle_job_failure(job_id).await.expect("Test call to handle_job_failure should have passed.");

    // The completed job status on db is untouched.
    let job_fetched_result = config.database().get_job_by_id(job_id).await.expect("Unable to fetch Job Data");

    if let Some(job_fetched) = job_fetched_result {
        assert_eq!(job_fetched.status, JobStatus::Completed);
    }
}
