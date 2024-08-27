pub mod constants;

use std::collections::HashMap;

use ::uuid::Uuid;
use aws_config::Region;
use mongodb::Client;
use rstest::*;
use serde::Deserialize;

use crate::data_storage::aws_s3::config::{AWSS3ConfigType, S3LocalStackConfig};
use crate::data_storage::aws_s3::AWSS3;
use crate::data_storage::{DataStorage, DataStorageConfig};
use crate::database::mongodb::config::MongoDbConfig;
use crate::database::mongodb::MongoDb;
use crate::database::DatabaseConfig;
use crate::jobs::types::JobStatus::Created;
use crate::jobs::types::JobType::DataSubmission;
use crate::jobs::types::{ExternalId, JobItem};
use crate::queue::job_queue::{JOB_PROCESSING_QUEUE, JOB_VERIFICATION_QUEUE};

#[fixture]
pub fn default_job_item() -> JobItem {
    JobItem {
        id: Uuid::new_v4(),
        internal_id: String::from("0"),
        job_type: DataSubmission,
        status: Created,
        external_id: ExternalId::String("0".to_string().into_boxed_str()),
        metadata: HashMap::new(),
        version: 0,
    }
}

#[fixture]
pub fn custom_job_item(default_job_item: JobItem, #[default(String::from("0"))] internal_id: String) -> JobItem {
    let mut job_item = default_job_item;
    job_item.internal_id = internal_id;

    job_item
}

pub async fn drop_database() -> color_eyre::Result<()> {
    let db_client: Client = MongoDb::new(MongoDbConfig::new_from_env()).await.client();
    // dropping all the collection.
    // use .collection::<JobItem>("<collection_name>")
    // if only particular collection is to be dropped
    db_client.database("orchestrator").drop(None).await?;
    Ok(())
}

// SQS structs & functions

pub async fn create_sqs_queues() -> color_eyre::Result<()> {
    let sqs_client = get_sqs_client().await;

    // Dropping sqs queues
    let list_queues_output = sqs_client.list_queues().send().await?;
    let queue_urls = list_queues_output.queue_urls();
    log::debug!("Found {} queues", queue_urls.len());
    for queue_url in queue_urls {
        match sqs_client.delete_queue().queue_url(queue_url).send().await {
            Ok(_) => log::debug!("Successfully deleted queue: {}", queue_url),
            Err(_e) => { },
        }
    }

    // Creating SQS queues
    sqs_client.create_queue().queue_name(JOB_PROCESSING_QUEUE).send().await?;
    sqs_client.create_queue().queue_name(JOB_VERIFICATION_QUEUE).send().await?;
    Ok(())
}

async fn get_sqs_client() -> aws_sdk_sqs::Client {
    // This function is for localstack. So we can hardcode the region for this as of now.
    let region_provider = Region::new("us-east-1");
    let config = aws_config::from_env().region(region_provider).load().await;
    aws_sdk_sqs::Client::new(&config)
}

#[derive(Deserialize, Debug)]
pub struct MessagePayloadType {
    pub(crate) id: Uuid,
}

pub async fn get_storage_client() -> Box<dyn DataStorage + Send + Sync> {
    Box::new(AWSS3::new(AWSS3ConfigType::WithEndpoint(S3LocalStackConfig::new_from_env())).await)
}
