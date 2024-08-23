use crate::jobs::types::JobStatus::Created;
use crate::jobs::types::JobType::DataSubmission;
use crate::jobs::types::{ExternalId, JobItem};
use crate::tests::config::{mongodb_testcontainer_setup, s3_testcontainer_setup, sqs_testcontainer_setup};
use std::collections::HashMap;
use uuid::Uuid;
use crate::queue::job_queue::{JobQueueMessage, JOB_PROCESSING_QUEUE};
use utils::env_utils::get_env_var_or_panic;
use rstest::rstest;
use bytes::Bytes;

#[rstest]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[tokio::test]
async fn testing_parallel_sqs(#[case] id: Uuid) {
    let (_node, sqs_queue, _client) = sqs_testcontainer_setup(JOB_PROCESSING_QUEUE.to_string()).await;

    let message = JobQueueMessage { id };
    let _ = sqs_queue
        .send_message_to_queue(JOB_PROCESSING_QUEUE.to_string(), serde_json::to_string(&message).unwrap(), None)
        .await;
    let consumed_messages = sqs_queue.consume_message_from_queue(JOB_PROCESSING_QUEUE.to_string()).await;

    assert!(consumed_messages.unwrap().take_payload().is_some());
}

#[rstest]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[tokio::test]
async fn testing_parallel_s3(#[case] id: Uuid) {
    let (_node, storage_client, client) = s3_testcontainer_setup().await;

    let aws_s3_bucket_name = get_env_var_or_panic("AWS_S3_BUCKET_NAME");

    // Verify bucket creation
    let list_buckets_output = client.list_buckets().send().await.unwrap();
    assert!(list_buckets_output.buckets.is_some());
    let buckets_list = list_buckets_output.buckets.unwrap();
    assert_eq!(1, buckets_list.len());
    assert_eq!(aws_s3_bucket_name.as_str(), buckets_list[0].name.as_ref().unwrap());

    // Testing Putting Data
    let key = "key";

    storage_client.put_data(Bytes::from(id.to_string()), key).await.unwrap();

    let val = storage_client.get_data(key).await.unwrap();

    println!("{:?}", val);
}

#[rstest]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[tokio::test]
async fn testing_parallel_mongo(#[case] id: Uuid) {
    let (_node, database) = mongodb_testcontainer_setup().await;

    let job_item = JobItem {
        id,
        internal_id: String::from("0"),
        job_type: DataSubmission,
        status: Created,
        external_id: ExternalId::String("0".to_string().into_boxed_str()),
        metadata: HashMap::new(),
        version: 0,
    };

    let _ = database.create_job(job_item.clone()).await.unwrap();

    let result = database.get_job_by_id(id).await.unwrap();

    assert!(result.is_some());
    assert_eq!(job_item, result.unwrap());
}