use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use aws_config::environment::EnvironmentVariableCredentialsProvider;
use aws_config::meta::region::RegionProviderChain;
use aws_config::{from_env, SdkConfig};
use aws_credential_types::provider::ProvideCredentials;
use aws_sdk_eventbridge::config::Region;
use aws_sdk_eventbridge::types::{InputTransformer, RuleState, Target};
use aws_sdk_sqs::types::QueueAttributeName;
use aws_sdk_sqs::Client;
use color_eyre::Result;
use lazy_static::lazy_static;
use omniqueue::backends::{SqsBackend, SqsConfig, SqsConsumer, SqsProducer};
use omniqueue::{Delivery, QueueError};
use utils::env_utils::get_env_var_or_panic;

use crate::queue::job_queue::{
    WorkerTriggerMessage, WorkerTriggerType, DATA_SUBMISSION_JOB_PROCESSING_QUEUE,
    DATA_SUBMISSION_JOB_VERIFICATION_QUEUE, JOB_HANDLE_FAILURE_QUEUE, PROOF_REGISTRATION_JOB_PROCESSING_QUEUE,
    PROOF_REGISTRATION_JOB_VERIFICATION_QUEUE, PROVING_JOB_PROCESSING_QUEUE, PROVING_JOB_VERIFICATION_QUEUE,
    SNOS_JOB_PROCESSING_QUEUE, SNOS_JOB_VERIFICATION_QUEUE, UPDATE_STATE_JOB_PROCESSING_QUEUE,
    UPDATE_STATE_JOB_VERIFICATION_QUEUE, WORKER_TRIGGER_QUEUE,
};
use crate::queue::QueueProvider;
pub struct SqsQueue;

lazy_static! {
    /// Maps Queue Name to Env var of queue URL.
    pub static ref QUEUE_NAME_TO_ENV_VAR_MAPPING: HashMap<&'static str, &'static str> = HashMap::from([
        (DATA_SUBMISSION_JOB_PROCESSING_QUEUE, "SQS_DATA_SUBMISSION_JOB_PROCESSING_QUEUE_URL"),
        (DATA_SUBMISSION_JOB_VERIFICATION_QUEUE, "SQS_DATA_SUBMISSION_JOB_VERIFICATION_QUEUE_URL"),
        (PROOF_REGISTRATION_JOB_PROCESSING_QUEUE, "SQS_PROOF_REGISTRATION_JOB_PROCESSING_QUEUE_URL"),
        (PROOF_REGISTRATION_JOB_VERIFICATION_QUEUE, "SQS_PROOF_REGISTRATION_JOB_VERIFICATION_QUEUE_URL"),
        (PROVING_JOB_PROCESSING_QUEUE, "SQS_PROVING_JOB_PROCESSING_QUEUE_URL"),
        (PROVING_JOB_VERIFICATION_QUEUE, "SQS_PROVING_JOB_VERIFICATION_QUEUE_URL"),
        (SNOS_JOB_PROCESSING_QUEUE, "SQS_SNOS_JOB_PROCESSING_QUEUE_URL"),
        (SNOS_JOB_VERIFICATION_QUEUE, "SQS_SNOS_JOB_VERIFICATION_QUEUE_URL"),
        (UPDATE_STATE_JOB_PROCESSING_QUEUE, "SQS_UPDATE_STATE_JOB_PROCESSING_QUEUE_URL"),
        (UPDATE_STATE_JOB_VERIFICATION_QUEUE, "SQS_UPDATE_STATE_JOB_VERIFICATION_QUEUE_URL"),
        (JOB_HANDLE_FAILURE_QUEUE, "SQS_JOB_HANDLE_FAILURE_QUEUE_URL"),
        (WORKER_TRIGGER_QUEUE, "SQS_WORKER_TRIGGER_QUEUE_URL"),
    ]);
}

#[async_trait]
impl QueueProvider for SqsQueue {
    async fn send_message_to_queue(&self, queue: String, payload: String, delay: Option<Duration>) -> Result<()> {
        let queue_url = get_queue_url(queue);
        let producer = get_producer(queue_url).await?;

        match delay {
            Some(d) => producer.send_raw_scheduled(payload.as_str(), d).await?,
            None => producer.send_raw(payload.as_str()).await?,
        }

        Ok(())
    }

    async fn consume_message_from_queue(&self, queue: String) -> std::result::Result<Delivery, QueueError> {
        let queue_url = get_queue_url(queue);
        let mut consumer = get_consumer(queue_url).await?;
        consumer.receive().await
    }

    async fn create_and_setup_queues(&self) -> Result<()> {
        // Setting up queues
        let client = Self::create_sqs_client().await;
        let dlq_arn = Self::create_dlq(&client).await?;
        log::debug!("DLQ created. DLQ ARN: {}", dlq_arn);

        for queue in Self::get_queue_configs() {
            let redrive_policy = if queue.needs_dlq {
                Some(format!(r#"{{"deadLetterTargetArn":"{}","maxReceiveCount":"{}"}}"#, dlq_arn, MAX_RECEIVE_COUNT))
            } else {
                None
            };

            let queue_url = Self::create_queue(&client, &queue.name, redrive_policy.as_deref()).await?;
            log::info!("Created queue: {} at URL: {}", queue.name, queue_url);
        }

        // Setting up event bridge :
        Self::setup_event_bridge().await?;

        Ok(())
    }
}

#[derive(Debug)]
struct QueueConfig {
    name: String,
    needs_dlq: bool,
}

const VISIBILITY_TIMEOUT: i32 = 1800; // 30 minutes in seconds
const DLQ_NAME: &str = "madara_orchestrator_job_handle_failure_queue";
const MAX_RECEIVE_COUNT: &str = "5";

impl SqsQueue {
    async fn get_aws_configs() -> SdkConfig {
        let region_provider = RegionProviderChain::default_provider();
        from_env().region(region_provider).load().await
    }
    async fn create_sqs_client() -> Client {
        Client::new(&Self::get_aws_configs().await)
    }

    async fn create_event_bridge_client() -> aws_sdk_eventbridge::Client {
        let region_provider = Region::new(get_env_var_or_panic("AWS_REGION"));
        let creds = EnvironmentVariableCredentialsProvider::new().provide_credentials().await.unwrap();
        let config = from_env().region(region_provider).credentials_provider(creds).load().await;
        aws_sdk_eventbridge::Client::new(&config)
    }

    fn get_queue_configs() -> Vec<QueueConfig> {
        vec![
            QueueConfig { name: "madara_orchestrator_snos_job_processing_queue".to_string(), needs_dlq: true },
            QueueConfig { name: "madara_orchestrator_snos_job_verification_queue".to_string(), needs_dlq: true },
            QueueConfig { name: "madara_orchestrator_proving_job_processing_queue".to_string(), needs_dlq: true },
            QueueConfig { name: "madara_orchestrator_proving_job_verification_queue".to_string(), needs_dlq: true },
            QueueConfig {
                name: "madara_orchestrator_data_submission_job_processing_queue".to_string(),
                needs_dlq: true,
            },
            QueueConfig {
                name: "madara_orchestrator_data_submission_job_verification_queue".to_string(),
                needs_dlq: true,
            },
            QueueConfig { name: "madara_orchestrator_update_state_job_processing_queue".to_string(), needs_dlq: true },
            QueueConfig {
                name: "madara_orchestrator_update_state_job_verification_queue".to_string(),
                needs_dlq: true,
            },
            QueueConfig { name: "madara_orchestrator_worker_trigger_queue".to_string(), needs_dlq: true },
        ]
    }

    async fn create_queue(client: &Client, queue_name: &str, redrive_policy: Option<&str>) -> Result<String> {
        let mut attributes = HashMap::new();
        attributes.insert(QueueAttributeName::VisibilityTimeout, VISIBILITY_TIMEOUT.to_string());
        if let Some(policy) = redrive_policy {
            attributes.insert(QueueAttributeName::RedrivePolicy, policy.to_string());
        }
        let response = client.create_queue().queue_name(queue_name).set_attributes(Some(attributes)).send().await?;
        Ok(response.queue_url().unwrap().to_string())
    }

    async fn get_queue_arn(client: &Client, queue_url: &str) -> Result<String> {
        let attributes = client
            .get_queue_attributes()
            .queue_url(queue_url)
            .attribute_names(QueueAttributeName::QueueArn)
            .send()
            .await?;

        Ok(attributes.attributes().unwrap().get(&QueueAttributeName::QueueArn).unwrap().to_string())
    }

    async fn create_dlq(client: &Client) -> Result<String> {
        let dlq_url = Self::create_queue(client, DLQ_NAME, None).await?;
        Self::get_queue_arn(client, &dlq_url).await
    }

    async fn setup_event_bridge() -> Result<()> {
        for trigger_type in [
            WorkerTriggerType::Snos,
            WorkerTriggerType::Proving,
            WorkerTriggerType::DataSubmission,
            WorkerTriggerType::UpdateState,
        ] {
            Self::setup_event_bridge_for_trigger_type(trigger_type.clone()).await?;
            log::info!("Event Bridge trigger created for trigger type: {:?}", trigger_type);
        }
        Ok(())
    }

    async fn setup_event_bridge_for_trigger_type(worker_trigger_type: WorkerTriggerType) -> Result<()> {
        let rule_name = "worker_trigger_scheduled";
        let client = Self::create_event_bridge_client().await;
        let sqs_client = Self::create_sqs_client().await;

        client
            .put_rule()
            .name(rule_name)
            .schedule_expression("rate(1 minute)")
            .state(RuleState::Enabled)
            .send()
            .await?;
        let queue_url = sqs_client.get_queue_url().queue_name(WORKER_TRIGGER_QUEUE).send().await?;

        let queue_attributes = sqs_client
            .get_queue_attributes()
            .queue_url(queue_url.queue_url.unwrap())
            .attribute_names(QueueAttributeName::QueueArn)
            .send()
            .await?;
        let queue_arn = queue_attributes.attributes().unwrap().get(&QueueAttributeName::QueueArn).unwrap();

        // Create a sample WorkerTriggerMessage
        let message = WorkerTriggerMessage { worker: worker_trigger_type.clone() };
        let event_detail = serde_json::to_string(&message)?;

        // Create the EventBridge target with the input transformer
        let input_transformer = InputTransformer::builder()
            .input_paths_map("$.time", "time")
            .input_template(event_detail.to_string())
            .build()?;

        client
            .put_targets()
            .rule(rule_name)
            .targets(
                Target::builder()
                    .id(format!("worker-trigger-target-{:?}", worker_trigger_type))
                    .arn(queue_arn)
                    .input_transformer(input_transformer)
                    .build()?,
            )
            .send()
            .await?;

        Ok(())
    }
}

/// To fetch the queue URL from the environment variables
fn get_queue_url(queue_name: String) -> String {
    get_env_var_or_panic(
        QUEUE_NAME_TO_ENV_VAR_MAPPING.get(queue_name.as_str()).expect("Not able to get the queue env var name."),
    )
}

// TODO: store the producer and consumer in memory to avoid creating a new one every time
async fn get_producer(queue: String) -> Result<SqsProducer> {
    let (producer, _) =
        SqsBackend::builder(SqsConfig { queue_dsn: queue, override_endpoint: true }).build_pair().await?;
    Ok(producer)
}

async fn get_consumer(queue: String) -> std::result::Result<SqsConsumer, QueueError> {
    let (_, consumer) =
        SqsBackend::builder(SqsConfig { queue_dsn: queue, override_endpoint: true }).build_pair().await?;
    Ok(consumer)
}
