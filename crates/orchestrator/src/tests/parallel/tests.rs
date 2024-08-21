use rstest::rstest;
use uuid::Uuid;

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
    use crate::tests::parallel::localstack::LocalStack;
    use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
    use aws_sdk_sqs as sqs;
    use testcontainers::runners::AsyncRunner;

    let node = LocalStack::default().start().await.unwrap();
    let host_ip = node.get_host().await.unwrap();
    let host_port = node.get_host_port_ipv4(4566).await.unwrap();

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let creds = sqs::config::Credentials::new("fake", "fake", None, None, "test");
    let config = aws_config::defaults(BehaviorVersion::v2024_03_28())
        .region(region_provider)
        .credentials_provider(creds)
        .endpoint_url(format!("http://{host_ip}:{host_port}"))
        .load()
        .await;
    let client = sqs::Client::new(&config);

    client.create_queue().queue_name("example-queue").send().await.unwrap();

    let list_result = client.list_queues().send().await.unwrap();

    println!("ENDPOINT URL: {:?}", list_result.queue_urls());

    assert_eq!(list_result.queue_urls().len(), 1);
}
