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
    use std::collections::HashMap;
    use std::sync::Arc;

    use mockall::predicate::eq;
    use rstest::rstest;
    use uuid::Uuid;

    use crate::config::config;
    use crate::jobs::constants::{JOB_PROCESS_ATTEMPT_METADATA_KEY, JOB_VERIFICATION_ATTEMPT_METADATA_KEY};
    use crate::jobs::job_handler_factory::mock_factory;
    use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType, JobVerificationStatus};
    use crate::jobs::{create_job, process_job, verify_job, Job, MockJob};
    use crate::tests::common::drop_database;
    use crate::tests::config::TestConfigBuilder;

    #[rstest]
    #[case(JobType::SnosRun, false, true)]
    #[case(JobType::ProofCreation, true, true)]
    #[should_panic]
    #[case(JobType::ProofRegistration, false, false)]
    #[tokio::test]
    async fn test_create_job_handler(
        #[case] job_type: JobType,
        #[case] job_exists_in_db: bool,
        #[case] job_implemented: bool,
    ) {
        let job_item = JobItem {
            id: Uuid::new_v4(),
            internal_id: "0".to_string(),
            job_type: job_type.clone(),
            status: JobStatus::Created,
            external_id: ExternalId::Number(0),
            metadata: Default::default(),
            version: 0,
        };

        let mut job_handler = MockJob::new();
        if job_implemented && !job_exists_in_db {
            // Expecting for create_job handler for that particular job.
            let job_item_clone = job_item.clone();
            job_handler.expect_create_job().times(1).returning(move |_, _, _| Ok(job_item_clone.clone()));
        }

        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();

        let config = config().await;
        let database_client = config.database();

        if job_exists_in_db {
            database_client.create_job(job_item).await.unwrap();
        }

        if job_implemented && !job_exists_in_db {
            let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
            let ctx = mock_factory::get_job_handler_context();
            // Mocking the `get_job_handler` call in create_job function.
            ctx.expect().times(1).with(eq(job_type.clone())).returning(move |_| Arc::clone(&y));
            let _ = create_job(job_type.clone(), "0".to_string(), HashMap::new()).await.is_ok();
        } else {
            let _ = create_job(job_type, "0".to_string(), HashMap::new()).await.is_err();
        }
    }

    #[rstest]
    #[case(JobType::SnosRun, JobStatus::Created, true)]
    #[case(JobType::DataSubmission, JobStatus::VerificationFailed("".to_string()), true)]
    #[case(JobType::SnosRun, JobStatus::VerificationFailed("".to_string()), false)]
    #[tokio::test]
    async fn test_process_job_handler(
        #[case] job_type: JobType,
        #[case] job_status: JobStatus,
        #[case] job_exists_in_db: bool,
    ) {
        let job_item = get_random_job_item_by_type_and_status(job_type.clone(), job_status.clone(), "1".to_string());

        // building config
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();

        let config = config().await;
        let database_client = config.database();
        let mut job_handler = MockJob::new();
        if job_exists_in_db {
            // creating job in database
            database_client.create_job(job_item.clone()).await.unwrap();
            // expecting process job function in job processor to return the external ID
            job_handler.expect_process_job().times(1).returning(move |_, _| Ok("0xbeef".to_string()));
            job_handler.expect_verification_polling_delay_seconds().return_const(1u64);
        }

        if job_exists_in_db && is_valid_job_processing_status(job_status) {
            let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
            let ctx = mock_factory::get_job_handler_context();
            // Mocking the `get_job_handler` call in create_job function.
            ctx.expect().times(1).with(eq(job_type.clone())).returning(move |_| Arc::clone(&y));

            let _ = process_job(job_item.id).await.is_ok();
            // getting the job
            let updated_job = database_client.get_job_by_id(job_item.id).await.unwrap().unwrap();
            // checking if job_status is updated in db
            assert_eq!(updated_job.status, JobStatus::PendingVerification);
        } else {
            let _ = process_job(job_item.id).await.is_err();
        }
    }

    #[rstest]
    #[case(JobType::DataSubmission, JobStatus::PendingVerification, JobVerificationStatus::Verified, true)]
    #[case(JobType::DataSubmission, JobStatus::PendingVerification, JobVerificationStatus::Rejected("".to_string()), true)]
    #[case(JobType::DataSubmission, JobStatus::PendingVerification, JobVerificationStatus::Pending, true)]
    #[case(JobType::SnosRun, JobStatus::Created, JobVerificationStatus::Rejected("".to_string()), false)]
    #[tokio::test]
    async fn test_verify_job_handler(
        #[case] job_type: JobType,
        #[case] job_status: JobStatus,
        #[case] verification_status: JobVerificationStatus,
        #[case] job_exists_in_db: bool,
    ) {
        let job_item = get_random_job_item_by_type_and_status(job_type.clone(), job_status.clone(), "1".to_string());
        let expected_verification_status = verification_status.clone();
        // building config
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();

        let config = config().await;
        let database_client = config.database();
        let mut job_handler = MockJob::new();

        if job_exists_in_db {
            // creating job in database
            database_client.create_job(job_item.clone()).await.unwrap();
            // expecting process job function in job processor to return the external ID
            job_handler.expect_verify_job().times(1).returning(move |_, _| Ok(verification_status.clone()));
            job_handler.expect_max_process_attempts().returning(move || 2u64);
            job_handler.expect_max_verification_attempts().returning(move || 2u64);
            job_handler.expect_verification_polling_delay_seconds().returning(move || 2u64);
        }

        if job_exists_in_db && is_valid_job_verification_status(job_status) {
            let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
            let ctx = mock_factory::get_job_handler_context();
            // Mocking the `get_job_handler` call in create_job function.
            ctx.expect().times(1).with(eq(job_type.clone())).returning(move |_| Arc::clone(&y));

            let _ = verify_job(job_item.id).await.is_ok();

            let updated_job = database_client.get_job_by_id(job_item.id).await.unwrap().unwrap();

            if expected_verification_status == JobVerificationStatus::Verified {
                assert_eq!(updated_job.status, JobStatus::Completed);
            } else if expected_verification_status == JobVerificationStatus::Rejected("".to_string()) {
                assert_eq!(updated_job.status, JobStatus::VerificationFailed("".to_string()));
            } else if expected_verification_status == JobVerificationStatus::Pending {
                assert_eq!(updated_job.metadata.get(JOB_VERIFICATION_ATTEMPT_METADATA_KEY).unwrap(), "1");
            }
        } else {
            let _ = verify_job(job_item.id).await.is_err();
        }
    }

    fn is_valid_job_processing_status(job_status: JobStatus) -> bool {
        matches!(job_status, JobStatus::Created | JobStatus::VerificationFailed(_))
    }

    fn is_valid_job_verification_status(job_status: JobStatus) -> bool {
        matches!(job_status, JobStatus::PendingVerification)
    }

    fn get_random_job_item_by_type_and_status(
        job_type: JobType,
        job_status: JobStatus,
        internal_id: String,
    ) -> JobItem {
        let mut hashmap: HashMap<String, String> = HashMap::new();
        hashmap.insert(JOB_PROCESS_ATTEMPT_METADATA_KEY.to_string(), "0".to_string());
        hashmap.insert(JOB_VERIFICATION_ATTEMPT_METADATA_KEY.to_string(), "0".to_string());
        JobItem {
            id: Uuid::new_v4(),
            internal_id,
            job_type,
            status: job_status,
            external_id: ExternalId::Number(0),
            metadata: hashmap,
            version: 0,
        }
    }
}
