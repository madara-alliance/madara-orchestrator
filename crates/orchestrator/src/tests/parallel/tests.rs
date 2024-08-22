use rstest::rstest;
use url::Url;
use utils::env_utils::get_env_var_or_panic;
use uuid::Uuid;

use crate::queue::{sqs::SqsQueue, QueueProvider};

#[rstest]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[tokio::test]
async fn testing_parallel_mongo(#[case] id: Uuid) {
    use crate::database::mongodb::config::MongoDbConfig;
    use crate::database::mongodb::MongoDb;
    use crate::database::Database;
    use crate::jobs::types::JobStatus::Created;
    use crate::jobs::types::JobType::DataSubmission;
    use crate::jobs::types::{ExternalId, JobItem};
    use crate::tests::parallel::mongo;
    use std::collections::HashMap;
    use testcontainers::{core::IntoContainerPort, runners::AsyncRunner};

    const LOCAL_PORT: u16 = 27017; // Changed to default MongoDB port

    let node = mongo::Mongo::default().start().await.unwrap();
    let host_ip = node.get_host().await.unwrap();
    let host_port = node.get_host_port_ipv4(LOCAL_PORT.tcp()).await.unwrap();
    let connection_url = format!("mongodb://{host_ip}:{host_port}/");

    let mongo_config = MongoDbConfig { url: connection_url };

    // Use this variable inside TestConfigBuilder
    let database = MongoDb::new(mongo_config).await;

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
    println!("done!");
}

#[rstest]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[tokio::test]
async fn testing_parallel_sqs(#[case] id: Uuid) {
    use crate::queue::job_queue::JobQueueMessage;
    use crate::{queue::job_queue::JOB_PROCESSING_QUEUE, tests::parallel::localstack::LocalStack};
    use aws_config::BehaviorVersion;
    use aws_config::Region;
    use aws_sdk_s3::config::Credentials;
    use aws_sdk_sqs as sqs;
    use testcontainers::runners::AsyncRunner;

    dotenvy::from_filename("../.env.test").unwrap();

    let node = LocalStack::default().start().await.unwrap();
    let host_ip = node.get_host().await.unwrap();
    let host_port = node.get_host_port_ipv4(4566).await.unwrap();

    let aws_access_key_id = get_env_var_or_panic("AWS_ACCESS_KEY_ID");
    let aws_secret_access_key = get_env_var_or_panic("AWS_SECRET_ACCESS_KEY");
    let aws_region = get_env_var_or_panic("AWS_REGION");
    let region_provider = Region::new(aws_region);
    let aws_endpoint_url = format!("http://{host_ip}:{host_port}");

    let creds = Credentials::new(aws_access_key_id, aws_secret_access_key, None, None, "test");
    let config = aws_config::defaults(BehaviorVersion::v2024_03_28())
        .region(region_provider)
        .credentials_provider(creds)
        .endpoint_url(aws_endpoint_url.clone())
        .load()
        .await;

    let client = sqs::Client::new(&config);

    // Queue creation.
    let queue_output = client.create_queue().queue_name(JOB_PROCESSING_QUEUE).send().await.unwrap();

    let queue_host_url = transform_url(queue_output.queue_url().unwrap(), &host_port);
    // Use this variable inside TestConfigBuilder
    let sqs_queue = SqsQueue::new(queue_host_url.to_string());

    let message = JobQueueMessage { id };
    let _ = sqs_queue
        .send_message_to_queue(JOB_PROCESSING_QUEUE.to_string(), serde_json::to_string(&message).unwrap(), None)
        .await;
    let consumed_messages = sqs_queue.consume_message_from_queue(JOB_PROCESSING_QUEUE.to_string()).await;

    assert!(consumed_messages.unwrap().take_payload().is_some());
}

fn transform_url(input: &str, host_port: &u16) -> String {
    // Parse the input URL
    let parsed_url = Url::parse(input).expect("Failed to parse URL");

    // Extract the host and port
    let host = parsed_url.host_str().unwrap();
    // let port = parsed_url.port().unwrap_or(80);

    // Extract the first path segment (which should be "000000000000")
    let first_path_segment = parsed_url.path_segments().and_then(|mut segments| segments.next()).unwrap_or("");

    // Construct the new URL
    format!("http://{}:{}/{}", host, host_port, first_path_segment)
}
