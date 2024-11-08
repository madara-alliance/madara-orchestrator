pub mod constants;

use std::collections::HashMap;
use std::sync::Arc;

use ::uuid::Uuid;
use aws_config::SdkConfig;
use aws_sdk_sns::error::SdkError;
use aws_sdk_sns::operation::create_topic::CreateTopicError;
use chrono::{SubsecRound, Utc};
use mongodb::Client;
use rstest::*;
use serde::Deserialize;

use crate::cli::alert::AlertParams;
use crate::cli::database::DatabaseParams;
use crate::cli::queue::QueueParams;
use crate::config::ProviderConfig;
use crate::data_storage::aws_s3::config::AWSS3Params;
use crate::data_storage::aws_s3::AWSS3;
use crate::data_storage::DataStorage;
use crate::database::mongodb::MongoDb;
use crate::jobs::types::JobStatus::Created;
use crate::jobs::types::JobType::DataSubmission;
use crate::jobs::types::{ExternalId, JobItem};
use crate::queue::job_queue::QueueType;

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
        created_at: Utc::now().round_subsecs(0),
        updated_at: Utc::now().round_subsecs(0),
    }
}

#[fixture]
pub fn custom_job_item(default_job_item: JobItem, #[default(String::from("0"))] internal_id: String) -> JobItem {
    let mut job_item = default_job_item;
    job_item.internal_id = internal_id;

    job_item
}

pub async fn create_sns_arn(
    provider_config: Arc<ProviderConfig>,
    alert_params: &AlertParams,
) -> Result<(), SdkError<CreateTopicError>> {
    let AlertParams::AWSSNS(aws_sns_params) = alert_params;
    let sns_client = get_sns_client(provider_config.get_aws_client_or_panic()).await;
    sns_client.create_topic().name(aws_sns_params.get_topic_name()).send().await?;
    Ok(())
}

pub async fn get_sns_client(aws_config: &SdkConfig) -> aws_sdk_sns::client::Client {
    aws_sdk_sns::Client::new(aws_config)
}

pub async fn drop_database(database_params: &DatabaseParams) -> color_eyre::Result<()> {
    match database_params {
        DatabaseParams::MongoDB(mongodb_params) => {
            let db_client: Client = MongoDb::new_with_params(mongodb_params).await.client();
            // dropping all the collection.
            // use .collection::<JobItem>("<collection_name>")
            // if only particular collection is to be dropped
            db_client.database(&mongodb_params.database_name).drop(None).await?;
        }
    }
    Ok(())
}

// SQS structs & functions

pub async fn create_queues(provider_config: Arc<ProviderConfig>, queue_params: &QueueParams) -> color_eyre::Result<()> {
    match queue_params {
        QueueParams::AWSSQS(aws_sqs_params) => {
            let sqs_client = get_sqs_client(provider_config).await;

            // Dropping sqs queues
            let list_queues_output = sqs_client.list_queues().send().await?;
            let queue_urls = list_queues_output.queue_urls();
            tracing::debug!("Found {} queues", queue_urls.len());
            for queue_url in queue_urls {
                match sqs_client.delete_queue().queue_url(queue_url).send().await {
                    Ok(_) => tracing::debug!("Successfully deleted queue: {}", queue_url),
                    Err(e) => tracing::error!("Error deleting queue {}: {:?}", queue_url, e),
                }
            }

            for queue_type in QueueType::iter() {
                let queue_name = aws_sqs_params.get_queue_name(queue_type);
                sqs_client.create_queue().queue_name(queue_name).send().await?;
            }
        }
    }
    Ok(())
}

pub async fn get_sqs_client(provider_config: Arc<ProviderConfig>) -> aws_sdk_sqs::Client {
    // This function is for localstack. So we can hardcode the region for this as of now.
    let config = provider_config.get_aws_client_or_panic();
    aws_sdk_sqs::Client::new(config)
}

#[derive(Deserialize, Debug)]
pub struct MessagePayloadType {
    pub(crate) id: Uuid,
}

pub async fn get_storage_client(
    storage_cfg: &AWSS3Params,
    provider_config: Arc<ProviderConfig>,
) -> Box<dyn DataStorage + Send + Sync> {
    Box::new(AWSS3::new_with_params(storage_cfg, provider_config).await)
}
