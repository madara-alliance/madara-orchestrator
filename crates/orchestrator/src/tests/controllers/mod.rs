use crate::config::config_force_init;
use crate::constants::JOB_PROCESSING_QUEUE;
use crate::controllers::jobs_controller::{create_job, CreateJobRequest};
use crate::database::MockDatabase;
use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
use crate::queue::MockQueueProvider;
use crate::tests::common::init_config;
use axum::Json;
use mockall::predicate::eq;
use rstest::rstest;
use uuid::Uuid;

#[rstest]
#[tokio::test]
async fn test_create_job_jobs_controller() -> color_eyre::Result<()> {
    let mut db = MockDatabase::new();
    let mut queue = MockQueueProvider::new();

    // mocking db get function (when creating job it should return no job existing)
    db.expect_get_last_successful_job_by_type().times(1).with(eq(JobType::SnosRun)).returning(|_| Ok(None));
    // mocking db get function (when creating job to pre-check if job is not existing : worker module)
    db.expect_get_job_by_internal_id_and_type().times(1).with(eq("1"), eq(JobType::SnosRun)).returning(|_, _| Ok(None));
    // mocking creation of the job
    db.expect_create_job().times(1).withf(move |item| item.internal_id == *"1").returning(move |_| {
        Ok(JobItem {
            id: Uuid::new_v4(),
            internal_id: "1".to_string(),
            job_type: JobType::SnosRun,
            status: JobStatus::Created,
            external_id: ExternalId::Number(0),
            metadata: Default::default(),
            version: 0,
        })
    });
    // mocking sending of the job into the queue after the creation
    queue
        .expect_send_message_to_queue()
        .returning(|_, _, _| Ok(()))
        .withf(|queue, _payload, _delay| queue == JOB_PROCESSING_QUEUE);

    let config = init_config(None, Some(db), Some(queue), None, None, None, None).await;
    config_force_init(config).await;

    let create_job_request = CreateJobRequest { job_type: JobType::SnosRun, internal_id: "1".to_string() };

    let _ = create_job(Json::from(create_job_request)).await.unwrap();

    Ok(())
}
