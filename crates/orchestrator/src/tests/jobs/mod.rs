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

#[cfg(test)]
mod job_handler_tests {
    use crate::config::config;
    use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
    use crate::jobs::{create_job, MockJob};
    use crate::tests::common::drop_database;
    use crate::tests::config::TestConfigBuilder;
    use rstest::rstest;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[rstest]
    #[case(JobType::SnosRun, false, true)]
    #[case(JobType::ProofCreation, true, true)]
    // #[case(JobType::ProofRegistration, false, false)]
    #[tokio::test]
    async fn test_create_job_handler(
        #[case] job_type: JobType,
        #[case] job_exists_in_db: bool,
        #[case] job_implemented: bool,
    ) {
        // MODIFY :
        // If queue needs to be spun up.
        // We need to implement it in localstack.

        let job_item = JobItem {
            id: Uuid::new_v4(),
            internal_id: "0".to_string(),
            job_type: job_type.clone(),
            status: JobStatus::Created,
            external_id: ExternalId::Number(0),
            metadata: Default::default(),
            version: 0,
        };

        // Expecting for create_job handler for that particular job.
        let mut job_handler = MockJob::new();

        // if job_implemented && !job_exists_in_db {
        //     job_handler.expect_create_job().times(1);
        // }

        TestConfigBuilder::new().build_with_mock_job(job_handler).await;
        drop_database().await.unwrap();

        let config = config().await;
        let database_client = config.database();

        if job_exists_in_db {
            database_client.create_job(job_item).await.unwrap();
        }

        if job_implemented && !job_exists_in_db {
            let _ = create_job(job_type, "0".to_string(), HashMap::new()).await.is_ok();
        } else {
            let _ = create_job(job_type, "0".to_string(), HashMap::new()).await.is_err();
        }
    }
}
