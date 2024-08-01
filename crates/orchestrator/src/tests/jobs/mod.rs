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
    use std::time::Duration;

    use mockall::predicate::eq;
    use mongodb::bson::doc;
    use rstest::rstest;
    use tokio::time::sleep;
    use uuid::Uuid;

    use crate::config::config;
    use crate::database::mongodb::config::MongoDbConfig;
    use crate::database::mongodb::MongoDb;
    use crate::database::{Database, DatabaseConfig, MockDatabase};
    use crate::jobs::constants::{JOB_PROCESS_ATTEMPT_METADATA_KEY, JOB_VERIFICATION_ATTEMPT_METADATA_KEY};
    use crate::jobs::job_handler_factory::mock_factory;
    use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType, JobVerificationStatus};
    use crate::jobs::{create_job, increment_key_in_metadata, process_job, verify_job, Job, MockJob};
    use crate::queue::job_queue::{JOB_PROCESSING_QUEUE, JOB_VERIFICATION_QUEUE};
    use crate::tests::common::{create_sqs_queues, drop_database, list_messages_in_queue, MessagePayloadType};
    use crate::tests::config::TestConfigBuilder;

    /// Tests `create_job` function when job is not existing in the db.
    #[rstest]
    #[tokio::test]
    async fn test_create_job_handler_job_does_not_exists_in_db() {
        let job_item = build_job_item_by_type_and_status(JobType::SnosRun, JobStatus::Created, "0".to_string());
        let mut job_handler = MockJob::new();

        // Adding expectation for creation of new job.
        let job_item_clone = job_item.clone();
        job_handler.expect_create_job().times(1).returning(move |_, _, _| Ok(job_item_clone.clone()));

        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        // Mocking the `get_job_handler` call in create_job function.
        let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
        let ctx = mock_factory::get_job_handler_context();
        ctx.expect().times(1).with(eq(JobType::SnosRun)).return_once(move |_| Arc::clone(&y));

        let _ = create_job(JobType::SnosRun, "0".to_string(), HashMap::new()).await.is_ok();

        let messages_in_queue = list_messages_in_queue(JOB_PROCESSING_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_queue.len(), 1);
        let message_0_body: MessagePayloadType =
            serde_json::from_str(&messages_in_queue[0].clone().body.unwrap()).unwrap();
        assert_eq!(message_0_body.id, job_item.id);
    }

    /// Tests `create_job` function when job is already existing in the db.
    #[rstest]
    #[tokio::test]
    async fn test_create_job_handler_job_exists_in_db() {
        let job_item = build_job_item_by_type_and_status(JobType::ProofCreation, JobStatus::Created, "0".to_string());

        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        let config = config().await;
        let database_client = config.database();
        database_client.create_job(job_item).await.unwrap();

        let _ = create_job(JobType::ProofCreation, "0".to_string(), HashMap::new()).await.is_err();

        let messages_in_queue = list_messages_in_queue(JOB_PROCESSING_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_queue.len(), 0);
    }

    /// Tests `create_job` function when job handler is not implemented in the `get_job_handler`
    /// This test should fail as job handler is not implemented in the `factory.rs`
    #[rstest]
    #[should_panic(expected = "Job type not implemented yet.")]
    #[tokio::test]
    async fn test_create_job_handler_job_handler_is_not_implemented() {
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        // Mocking the `get_job_handler` call in create_job function.
        let ctx = mock_factory::get_job_handler_context();
        ctx.expect().times(1).with(eq(JobType::ProofCreation)).returning(|_| panic!("Job type not implemented yet."));

        let _ = create_job(JobType::ProofCreation, "0".to_string(), HashMap::new()).await.is_err();

        let messages_in_queue = list_messages_in_queue(JOB_PROCESSING_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_queue.len(), 0);
    }

    /// Tests `process_job` function when job is already existing in the db and job status is either
    /// `Created` or `VerificationFailed`.
    #[rstest]
    #[case(JobType::SnosRun, JobStatus::Created)]
    #[case(JobType::DataSubmission, JobStatus::VerificationFailed("".to_string()))]
    #[tokio::test]
    async fn test_process_job_handler_job_exists_in_db_and_valid_job_processing_status(
        #[case] job_type: JobType,
        #[case] job_status: JobStatus,
    ) {
        let job_item = build_job_item_by_type_and_status(job_type.clone(), job_status.clone(), "1".to_string());

        // Building config
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        let config = config().await;
        let database_client = config.database();

        let mut job_handler = MockJob::new();

        // Creating job in database
        database_client.create_job(job_item.clone()).await.unwrap();
        // Expecting process job function in job processor to return the external ID.
        job_handler.expect_process_job().times(1).returning(move |_, _| Ok("0xbeef".to_string()));
        job_handler.expect_verification_polling_delay_seconds().return_const(1u64);

        // Mocking the `get_job_handler` call in create_job function.
        let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
        let ctx = mock_factory::get_job_handler_context();
        ctx.expect().times(1).with(eq(job_type.clone())).returning(move |_| Arc::clone(&y));

        let _ = process_job(job_item.id).await.is_ok();
        // Getting the updated job.
        let updated_job = database_client.get_job_by_id(job_item.id).await.unwrap().unwrap();
        // checking if job_status is updated in db
        assert_eq!(updated_job.status, JobStatus::PendingVerification);

        // Queue checks
        let messages_in_queue = list_messages_in_queue(JOB_VERIFICATION_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_queue.len(), 1);
        let message_0_body: MessagePayloadType =
            serde_json::from_str(&messages_in_queue[0].clone().body.unwrap()).unwrap();
        assert_eq!(message_0_body.id, job_item.id);
    }

    /// Tests `process_job` function when job is already existing in the db and job status is not
    /// `Created` or `VerificationFailed`.
    #[rstest]
    #[tokio::test]
    async fn test_process_job_handler_job_exists_in_db_and_invalid_job_processing_status() {
        // Creating a job with Completed status which is invalid processing.
        let job_item = build_job_item_by_type_and_status(JobType::SnosRun, JobStatus::Completed, "1".to_string());

        // building config
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        let config = config().await;
        let database_client = config.database();

        // creating job in database
        database_client.create_job(job_item.clone()).await.unwrap();

        let _ = process_job(job_item.id).await.is_err();

        let job_in_db = database_client.get_job_by_id(job_item.id).await.unwrap().unwrap();
        // Job should be untouched in db.
        assert_eq!(job_in_db.status, JobStatus::Completed);

        let messages_in_queue = list_messages_in_queue(JOB_VERIFICATION_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_queue.len(), 0);
    }

    /// Tests `process_job` function when job is not in the db
    /// This test should fail
    #[rstest]
    #[tokio::test]
    async fn test_process_job_handler_job_does_not_exists_in_db() {
        // Creating a valid job which is not existing in the db.
        let job_item = build_job_item_by_type_and_status(JobType::SnosRun, JobStatus::Created, "1".to_string());

        // building config
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        let _ = process_job(job_item.id).await.is_err();
        let messages_in_queue = list_messages_in_queue(JOB_VERIFICATION_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_queue.len(), 0);
    }

    /// Tests `process_job` function when 2 workers try to process the same job.
    /// This test should fail because once the job is locked for processing on one
    /// worker it should not be accessed by another worker and should throw an error
    /// when updating the job status.
    #[rstest]
    #[tokio::test]
    async fn test_process_job_two_workers_process_same_job() {
        // Loading .env.test to get the db client
        dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

        drop_database().await.unwrap();

        // Creating a valid job which is not existing in the db.
        let mut job_item = build_job_item_by_type_and_status(JobType::SnosRun, JobStatus::Created, "1".to_string());
        let job_item_cloned = job_item.clone();

        let mut job_handler = MockJob::new();
        // Expecting process job function in job processor to return the external ID.
        job_handler.expect_process_job().times(2).returning(move |_, _| Ok("0xbeef".to_string()));
        job_handler.expect_verification_polling_delay_seconds().return_const(1u64);

        // Creating a new mongo db client for actual database.
        let db_client: Box<dyn Database> = Box::new(get_mongo_db_client().await);
        // Creating the job in actual db instance
        db_client.create_job(job_item.clone()).await.unwrap();
        let job_in_db = db_client.get_job_by_id(job_item.id).await.unwrap().unwrap();

        println!("job_in_db : {:?}", job_in_db);

        // Spinning up a new mock database.
        let mut database = MockDatabase::new();

        let metadata =
            crate::jobs::increment_key_in_metadata(&job_item.metadata, JOB_PROCESS_ATTEMPT_METADATA_KEY).unwrap();
        job_item.external_id = "0xbeef".to_string().into();
        job_item.status = JobStatus::PendingVerification;
        job_item.metadata = metadata;

        let job_in_db_clone = job_in_db.clone();

        // Adding expectations for mock database.
        database
            .expect_get_job_by_id()
            .times(1)
            .with(eq(job_in_db_clone.id))
            .return_once(move |_| Ok(Some(job_in_db.clone())));
        database
            .expect_update_job_status()
            .with(eq(job_in_db_clone.clone()), eq(JobStatus::LockedForProcessing))
            .times(1)
            .returning(|_, _| Ok(()));
        database.expect_update_job().with(eq(job_item.clone())).times(1).returning(|_| Ok(()));

        // Mocking the `get_job_handler` call in create_job function.
        let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
        let ctx = mock_factory::get_job_handler_context();
        ctx.expect().times(2).with(eq(JobType::SnosRun)).returning(move |_| Arc::clone(&y));

        // building config
        TestConfigBuilder::new().mock_db_client(Box::new(database)).build().await;

        let _ = process_job(job_item.id).await.is_ok();
        // Updating the job in db.
        db_client.update_job_status(&job_item, JobStatus::LockedForProcessing).await.unwrap();

        // Worker 2 Attempt
        // ===================================================================

        // Spinning up a new mock database.
        let mut database_2 = MockDatabase::new();
        let mut job_item_clone_2 = job_item_cloned.clone();

        let metadata =
            crate::jobs::increment_key_in_metadata(&job_item_clone_2.metadata, JOB_PROCESS_ATTEMPT_METADATA_KEY)
                .unwrap();
        job_item_clone_2.external_id = "0xbeef".to_string().into();
        job_item_clone_2.status = JobStatus::PendingVerification;
        job_item_clone_2.metadata = metadata;

        // Adding expectations for mock database.
        database_2
            .expect_get_job_by_id()
            .times(1)
            .with(eq(job_in_db_clone.id))
            .returning(move |_| Ok(Some(job_item_cloned.clone())));
        database_2
            .expect_update_job_status()
            .with(eq(job_in_db_clone.clone()), eq(JobStatus::LockedForProcessing))
            .times(1)
            .returning(|_, _| Ok(()));
        database_2.expect_update_job().with(eq(job_item_clone_2.clone())).times(1).returning(|_| Ok(()));
        sleep(Duration::from_secs(2)).await;

        // Making new config with database 2 mock
        TestConfigBuilder::new().mock_db_client(Box::new(database_2)).build().await;

        let _ = process_job(job_item_clone_2.id).await.is_ok();

        // This should fail as there would be conflicting versions.
        let _ = db_client.update_job_status(&job_item, JobStatus::LockedForProcessing).await.is_err();
    }

    /// Tests `verify_job` function when job is having expected status
    /// and returns a `Verified` verification status.
    #[rstest]
    #[tokio::test]
    async fn test_verify_job_handler_with_expected_job_status_and_verified_status_return() {
        let job_item =
            build_job_item_by_type_and_status(JobType::DataSubmission, JobStatus::PendingVerification, "1".to_string());

        // building config
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        let config = config().await;
        let database_client = config.database();
        let mut job_handler = MockJob::new();

        // creating job in database
        database_client.create_job(job_item.clone()).await.unwrap();
        // expecting process job function in job processor to return the external ID
        job_handler.expect_verify_job().times(1).returning(move |_, _| Ok(JobVerificationStatus::Verified));
        job_handler.expect_max_process_attempts().returning(move || 2u64);

        let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
        let ctx = mock_factory::get_job_handler_context();
        // Mocking the `get_job_handler` call in create_job function.
        ctx.expect().times(1).with(eq(JobType::DataSubmission)).returning(move |_| Arc::clone(&y));

        let _ = verify_job(job_item.id).await.is_ok();

        // DB checks.
        let updated_job = database_client.get_job_by_id(job_item.id).await.unwrap().unwrap();
        assert_eq!(updated_job.status, JobStatus::Completed);

        // Queue checks.
        let messages_in_process_queue = list_messages_in_queue(JOB_PROCESSING_QUEUE.to_string()).await.unwrap();
        let messages_in_verification_queue = list_messages_in_queue(JOB_VERIFICATION_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_process_queue.len(), 0);
        assert_eq!(messages_in_verification_queue.len(), 0);
    }

    /// Tests `verify_job` function when job is having expected status
    /// and returns a `Rejected` verification status.
    #[rstest]
    #[tokio::test]
    async fn test_verify_job_handler_with_expected_job_status_and_rejected_status_return_and_adds_process_to_job_queue()
    {
        let job_item =
            build_job_item_by_type_and_status(JobType::DataSubmission, JobStatus::PendingVerification, "1".to_string());

        // building config
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        let config = config().await;
        let database_client = config.database();
        let mut job_handler = MockJob::new();

        // creating job in database
        database_client.create_job(job_item.clone()).await.unwrap();
        // expecting process job function in job processor to return the external ID
        job_handler
            .expect_verify_job()
            .times(1)
            .returning(move |_, _| Ok(JobVerificationStatus::Rejected("".to_string())));
        job_handler.expect_max_process_attempts().returning(move || 2u64);

        let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
        let ctx = mock_factory::get_job_handler_context();
        // Mocking the `get_job_handler` call in create_job function.
        ctx.expect().times(1).with(eq(JobType::DataSubmission)).returning(move |_| Arc::clone(&y));

        let _ = verify_job(job_item.id).await.is_ok();

        // DB checks.
        let updated_job = database_client.get_job_by_id(job_item.id).await.unwrap().unwrap();
        assert_eq!(updated_job.status, JobStatus::VerificationFailed("".to_string()));

        // Queue checks.
        let messages_in_queue = list_messages_in_queue(JOB_PROCESSING_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_queue.len(), 1);
        let message_0_body: MessagePayloadType =
            serde_json::from_str(&messages_in_queue[0].clone().body.unwrap()).unwrap();
        assert_eq!(message_0_body.id, job_item.id);
    }

    /// Tests `verify_job` function when job is having expected status
    /// and returns a `Rejected` verification status but doesn't add
    /// the job to process queue because of maximum attempts reached.
    #[rstest]
    #[tokio::test]
    async fn test_verify_job_handler_with_expected_job_status_and_rejected_status_return() {
        let mut job_item =
            build_job_item_by_type_and_status(JobType::DataSubmission, JobStatus::PendingVerification, "1".to_string());

        // increasing JOB_VERIFICATION_ATTEMPT_METADATA_KEY to simulate max. attempts reached.
        let metadata = increment_key_in_metadata(&job_item.metadata, JOB_PROCESS_ATTEMPT_METADATA_KEY).unwrap();
        job_item.metadata = metadata;

        // building config
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        let config = config().await;
        let database_client = config.database();
        let mut job_handler = MockJob::new();

        // creating job in database
        database_client.create_job(job_item.clone()).await.unwrap();
        // expecting process job function in job processor to return the external ID
        job_handler
            .expect_verify_job()
            .times(1)
            .returning(move |_, _| Ok(JobVerificationStatus::Rejected("".to_string())));
        job_handler.expect_max_process_attempts().returning(move || 1u64);

        let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
        let ctx = mock_factory::get_job_handler_context();
        // Mocking the `get_job_handler` call in create_job function.
        ctx.expect().times(1).with(eq(JobType::DataSubmission)).returning(move |_| Arc::clone(&y));

        let _ = verify_job(job_item.id).await.is_ok();

        // DB checks.
        let updated_job = database_client.get_job_by_id(job_item.id).await.unwrap().unwrap();
        assert_eq!(updated_job.status, JobStatus::VerificationFailed("".to_string()));

        // Queue checks.
        let messages_in_queue = list_messages_in_queue(JOB_PROCESSING_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_queue.len(), 0);
    }

    /// Tests `verify_job` function when job is having expected status
    /// and returns a `Pending` verification status.
    #[rstest]
    #[tokio::test]
    async fn test_verify_job_handler_with_expected_job_status_and_pending_status_return_and_adds_job_to_verification_queue(
    ) {
        let job_item =
            build_job_item_by_type_and_status(JobType::DataSubmission, JobStatus::PendingVerification, "1".to_string());

        // building config
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        let config = config().await;
        let database_client = config.database();
        let mut job_handler = MockJob::new();

        // creating job in database
        database_client.create_job(job_item.clone()).await.unwrap();
        // expecting process job function in job processor to return the external ID
        job_handler.expect_verify_job().times(1).returning(move |_, _| Ok(JobVerificationStatus::Pending));
        job_handler.expect_max_verification_attempts().returning(move || 2u64);
        job_handler.expect_verification_polling_delay_seconds().returning(move || 2u64);

        let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
        let ctx = mock_factory::get_job_handler_context();
        // Mocking the `get_job_handler` call in create_job function.
        ctx.expect().times(1).with(eq(JobType::DataSubmission)).returning(move |_| Arc::clone(&y));

        let _ = verify_job(job_item.id).await.is_ok();

        // DB checks.
        let updated_job = database_client.get_job_by_id(job_item.id).await.unwrap().unwrap();
        assert_eq!(updated_job.metadata.get(JOB_VERIFICATION_ATTEMPT_METADATA_KEY).unwrap(), "1");

        // Queue checks.
        let messages_in_queue = list_messages_in_queue(JOB_VERIFICATION_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_queue.len(), 1);
        let message_0_body: MessagePayloadType =
            serde_json::from_str(&messages_in_queue[0].clone().body.unwrap()).unwrap();
        assert_eq!(message_0_body.id, job_item.id);
    }

    /// Tests `verify_job` function when job is having expected status
    /// and returns a `Pending` verification status but doesn't add
    /// the job to process queue because of maximum attempts reached.
    #[rstest]
    #[tokio::test]
    async fn test_verify_job_handler_with_expected_job_status_and_pending_status_return() {
        let mut job_item =
            build_job_item_by_type_and_status(JobType::DataSubmission, JobStatus::PendingVerification, "1".to_string());

        // increasing JOB_VERIFICATION_ATTEMPT_METADATA_KEY to simulate max. attempts reached.
        let metadata = increment_key_in_metadata(&job_item.metadata, JOB_VERIFICATION_ATTEMPT_METADATA_KEY).unwrap();
        job_item.metadata = metadata;

        // building config
        TestConfigBuilder::new().build().await;
        drop_database().await.unwrap();
        create_sqs_queues().await.unwrap();

        let config = config().await;
        let database_client = config.database();
        let mut job_handler = MockJob::new();

        // creating job in database
        database_client.create_job(job_item.clone()).await.unwrap();
        // expecting process job function in job processor to return the external ID
        job_handler.expect_verify_job().times(1).returning(move |_, _| Ok(JobVerificationStatus::Pending));
        job_handler.expect_max_verification_attempts().returning(move || 1u64);
        job_handler.expect_verification_polling_delay_seconds().returning(move || 2u64);

        let y: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
        let ctx = mock_factory::get_job_handler_context();
        // Mocking the `get_job_handler` call in create_job function.
        ctx.expect().times(1).with(eq(JobType::DataSubmission)).returning(move |_| Arc::clone(&y));

        let _ = verify_job(job_item.id).await.is_ok();

        // DB checks.
        let updated_job = database_client.get_job_by_id(job_item.id).await.unwrap().unwrap();
        assert_eq!(updated_job.status, JobStatus::VerificationTimeout);

        // Queue checks.
        let messages_in_queue = list_messages_in_queue(JOB_VERIFICATION_QUEUE.to_string()).await.unwrap();
        assert_eq!(messages_in_queue.len(), 0);
    }

    fn build_job_item_by_type_and_status(job_type: JobType, job_status: JobStatus, internal_id: String) -> JobItem {
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

    async fn get_mongo_db_client() -> MongoDb {
        MongoDb::new(MongoDbConfig::new_from_env()).await
    }
}
