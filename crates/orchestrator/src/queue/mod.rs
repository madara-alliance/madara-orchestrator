pub mod job_queue;
pub mod sqs;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use color_eyre::Result as EyreResult;
use lazy_static::lazy_static;
use mockall::automock;
use omniqueue::{Delivery, QueueError};

use crate::config::Config;
use crate::jobs::JobError;
use crate::setup::SetupConfig;

#[derive(Clone)]
pub struct QueueConfig {
    pub name: String,
    pub visibility_timeout: i32,
    pub max_receive_count: Option<i32>,
    pub dlq_name: Option<String>,
}

lazy_static! {
    pub static ref JOB_HANDLE_FAILURE_QUEUE: String = String::from("madara_orchestrator_job_handle_failure_queue");
    pub static ref QUEUES: Vec<QueueConfig> = vec![
        QueueConfig {
            name: String::from("madara_orchestrator_snos_job_processing_queue"),
            visibility_timeout: 300,
            max_receive_count: Some(5),
            dlq_name: Some(JOB_HANDLE_FAILURE_QUEUE.clone())
        },
        QueueConfig {
            name: String::from("madara_orchestrator_snos_job_verification_queue"),
            visibility_timeout: 300,
            max_receive_count: Some(5),
            dlq_name: Some(JOB_HANDLE_FAILURE_QUEUE.clone())
        },
        QueueConfig {
            name: String::from("madara_orchestrator_proving_job_processing_queue"),
            visibility_timeout: 300,
            max_receive_count: Some(5),
            dlq_name: Some(JOB_HANDLE_FAILURE_QUEUE.clone())
        },
        QueueConfig {
            name: String::from("madara_orchestrator_proving_job_verification_queue"),
            visibility_timeout: 300,
            max_receive_count: Some(5),
            dlq_name: Some(JOB_HANDLE_FAILURE_QUEUE.clone())
        },
        QueueConfig {
            name: String::from("madara_orchestrator_data_submission_job_processing_queue"),
            visibility_timeout: 300,
            max_receive_count: Some(5),
            dlq_name: Some(JOB_HANDLE_FAILURE_QUEUE.clone())
        },
        QueueConfig {
            name: String::from("madara_orchestrator_data_submission_job_verification_queue"),
            visibility_timeout: 300,
            max_receive_count: Some(5),
            dlq_name: Some(JOB_HANDLE_FAILURE_QUEUE.clone())
        },
        QueueConfig {
            name: String::from("madara_orchestrator_update_state_job_processing_queue"),
            visibility_timeout: 300,
            max_receive_count: Some(5),
            dlq_name: Some(JOB_HANDLE_FAILURE_QUEUE.clone())
        },
        QueueConfig {
            name: String::from("madara_orchestrator_update_state_job_verification_queue"),
            visibility_timeout: 300,
            max_receive_count: Some(5),
            dlq_name: Some(JOB_HANDLE_FAILURE_QUEUE.clone())
        },
        QueueConfig {
            name: String::from("madara_orchestrator_job_handle_failure_queue"),
            visibility_timeout: 300,
            max_receive_count: None,
            dlq_name: None
        },
        QueueConfig {
            name: String::from("madara_orchestrator_worker_trigger_queue"),
            visibility_timeout: 300,
            max_receive_count: None,
            dlq_name: None
        },
    ];
}

/// Queue Provider Trait
///
/// The QueueProvider trait is used to define the methods that a queue
/// should implement to be used as a queue for the orchestrator. The
/// purpose of this trait is to allow developers to use any queue of their choice.
#[automock]
#[async_trait]
pub trait QueueProvider: Send + Sync {
    async fn send_message_to_queue(&self, queue: String, payload: String, delay: Option<Duration>) -> EyreResult<()>;
    async fn consume_message_from_queue(&self, queue: String) -> Result<Delivery, QueueError>;
    async fn create_queue(&self, queue_config: &QueueConfig, config: &SetupConfig) -> EyreResult<()>;
    async fn setup(&self, config: SetupConfig) -> EyreResult<()> {
        // Creating the queues :
        for queue in QUEUES.iter() {
            self.create_queue(queue, &config).await?;
        }
        Ok(())
    }
}

pub async fn init_consumers(config: Arc<Config>) -> Result<(), JobError> {
    job_queue::init_consumers(config).await
}
