use crate::database::mongodb::config::MongoDbConfig;
use crate::database::mongodb::MongoDb;
use crate::database::Database;
use crate::jobs::types::JobStatus::Created;
use crate::jobs::types::JobType::DataSubmission;
use rstest::rstest;
use testcontainers::{core::ContainerPort, runners::AsyncRunner, GenericImage};

use testcontainers::core::WaitFor;
use testcontainers::ImageExt;
use uuid::Uuid;

#[rstest]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[case(Uuid::new_v4())]
#[tokio::test]
async fn testing_parallel_mongo(#[case] id: Uuid) {
    use crate::jobs::types::{ExternalId, JobItem};
    use std::collections::HashMap;

    const LOCAL_PORT: u16 = 27017; // Changed to default MongoDB port

    let msg = WaitFor::message_on_stdout("Waiting for connections");
    let container_port = ContainerPort::from(LOCAL_PORT);
    let container = GenericImage::new("mongo", "latest")
        .with_wait_for(msg)
        .with_exposed_port(container_port)
        .with_env_var("MONGO_INITDB_DATABASE", "product_info")
        .with_env_var("MONGO_INITDB_ROOT_USERNAME", "root")
        .with_env_var("MONGO_INITDB_ROOT_PASSWORD", "root")
        .start()
        .await
        .unwrap();

    let host = container.get_host().await.unwrap();
    let host_port = container.get_host_port_ipv4(LOCAL_PORT).await.unwrap();

    let connection_url = format!("mongodb://root:root@{}:{}", host, host_port);

    let mongo_config = MongoDbConfig { url: connection_url };

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
