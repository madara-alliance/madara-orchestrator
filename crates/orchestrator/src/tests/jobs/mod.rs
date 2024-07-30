use super::database::build_job_item;
use crate::config::config;
use crate::jobs::handle_job_failure;
use crate::jobs::types::JobType;
use crate::{jobs::types::JobStatus, tests::config::TestConfigBuilder};
use rstest::rstest;

#[cfg(test)]
pub mod da_job;

#[cfg(test)]
pub mod proving_job;

#[cfg(test)]
pub mod state_update_job;

#[rstest]
#[tokio::test]
async fn create_job_fails_job_already_exists() {
    // TODO
}

#[rstest]
#[tokio::test]
async fn create_job_fails_works_new_job() {
    // TODO
}

use std::str::FromStr;

impl FromStr for JobStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Created" => Ok(JobStatus::Created),
            "LockedForProcessing" => Ok(JobStatus::LockedForProcessing),
            "PendingVerification" => Ok(JobStatus::PendingVerification),
            "Completed" => Ok(JobStatus::Completed),
            "VerificationTimeout" => Ok(JobStatus::VerificationTimeout),
            "Failed" => Ok(JobStatus::Failed),
            s if s.starts_with("VerificationFailed(") && s.ends_with(')') => {
                let reason = s[19..s.len() - 1].to_string();
                Ok(JobStatus::VerificationFailed(reason))
            }
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
#[case("DataSubmission", "Completed")] // code should panic here, how can completed move to dl queue ?
#[case("SnosRun", "PendingVerification")]
#[case("ProofCreation", "LockedForProcessing")]
#[case("ProofRegistration", "Created")]
#[case("StateTransition", "Completed")]
#[case("ProofCreation", "VerificationTimeout")]
#[case("DataSubmission", "VerificationFailed()")]
#[tokio::test]
async fn test_handle_job_failure(#[case] job_type: JobType, #[case] error_type: JobStatus) -> color_eyre::Result<()> {
    use color_eyre::eyre::eyre;

    TestConfigBuilder::new().build().await;
    dotenvy::from_filename("../.env.test")?;

    let internal_id = 1;

    let config = config().await;
    let database_client = config.database();

    // create a job
    let job = build_job_item(job_type, error_type, internal_id);
    let job_id = job.id;

    // feeding the job to DB
    database_client.create_job(job.clone()).await.unwrap();

    // calling handle_job_failure
    let response = handle_job_failure(job_id).await;

    match response {
        Ok(()) => {
            // check job in db
            let job = config.database().get_job_by_id(job_id).await?;

            if let Some(job_item) = job {
                // check if job status is Failure
                assert_eq!(job_item.status, JobStatus::Failed);
                // check if job metadata has `last_job_status`
                assert_ne!(None, job_item.metadata.get("last_job_status"));

                println!("Handle Job Failure for ID {} was handled successfully", job_id);
            } else {
                return Err(eyre!("Unable to fetch Job Data"));
            }
        }
        Err(err) => {
            let expected = eyre!("Invalid state exists on DL queue: Completed");
            // Should only fail for Completed case, anything else : raise error
            assert_eq!(err.to_string(), expected.to_string());
        }
    }
    Ok(())
}
