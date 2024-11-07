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

lazy_static! {
    pub static ref JOB_QUEUES: Vec<String> = vec![
        String::from("madara_orchestrator_snos_job_processing_queue"),
        String::from("madara_orchestrator_snos_job_verification_queue"),
        String::from("madara_orchestrator_proving_job_processing_queue"),
        String::from("madara_orchestrator_proving_job_verification_queue"),
        String::from("madara_orchestrator_data_submission_job_processing_queue"),
        String::from("madara_orchestrator_data_submission_job_verification_queue"),
        String::from("madara_orchestrator_update_state_job_processing_queue"),
        String::from("madara_orchestrator_update_state_job_verification_queue"),
    ];
    pub static ref OTHER_QUEUES: Vec<String> = vec![
        String::from("madara_orchestrator_job_handle_failure_queue"),
        String::from("madara_orchestrator_worker_trigger_queue"),
    ];
    pub static ref JOB_HANDLE_FAILURE_QUEUE: String = String::from("madara_orchestrator_job_handle_failure_queue");
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
    async fn create_queue(&self, queue_name: &str, config: &SetupConfig) -> EyreResult<()>;
    async fn setup_queue(
        &self,
        queue_name: &str,
        config: &SetupConfig,
        needs_dlq: Option<String>,
        visibility_timeout: u32,
        max_receive_count: Option<u32>,
    ) -> EyreResult<()>;
    async fn setup(&self, config: SetupConfig, visibility_timeout: u32, max_receive_count: u32) -> EyreResult<()> {
        // Creating the job queues :
        for queue in JOB_QUEUES.iter() {
            self.create_queue(queue, &config).await?;
        }
        // Creating the other queues :
        for queue in OTHER_QUEUES.iter() {
            self.create_queue(queue, &config).await?;
        }

        // Setting up the job queues :
        for queue in JOB_QUEUES.iter() {
            self.setup_queue(
                queue,
                &config,
                Some(JOB_HANDLE_FAILURE_QUEUE.clone()),
                visibility_timeout,
                Some(max_receive_count),
            )
            .await?;
        }
        Ok(())
    }
}

pub async fn init_consumers(config: Arc<Config>) -> Result<(), JobError> {
    job_queue::init_consumers(config).await
}
