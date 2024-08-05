use std::future::Future;
use std::time::Duration;

use color_eyre::eyre::Context;
use color_eyre::Result as EyreResult;
use omniqueue::QueueError;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::log;
use uuid::Uuid;

use crate::config::config;
use crate::jobs::{process_job, verify_job, JobError};

pub const JOB_PROCESSING_QUEUE: &str = "madara_orchestrator_job_processing_queue";
pub const JOB_VERIFICATION_QUEUE: &str = "madara_orchestrator_job_verification_queue";

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConsumptionError {
    #[error("Failed to consume message from queue, error {error_msg:?}")]
    FailedToConsumeFromQueue { error_msg: String },

    #[error("Failed to handle job with id {job_id:?}. Error: {error_msg:?}")]
    FailedToHandleJob { job_id: Uuid, error_msg: String },

    #[error("Other error: {0}")]
    Other(#[from] color_eyre::eyre::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobQueueMessage {
    pub(crate) id: Uuid,
}

pub async fn add_job_to_process_queue(id: Uuid) -> EyreResult<()> {
    log::info!("Adding job with id {:?} to processing queue", id);
    add_job_to_queue(id, JOB_PROCESSING_QUEUE.to_string(), None).await
}

pub async fn add_job_to_verification_queue(id: Uuid, delay: Duration) -> EyreResult<()> {
    log::info!("Adding job with id {:?} to verification queue", id);
    add_job_to_queue(id, JOB_VERIFICATION_QUEUE.to_string(), Some(delay)).await
}

pub async fn consume_job_from_queue<F, Fut>(queue: String, handler: F) -> Result<(), ConsumptionError>
where
    F: FnOnce(Uuid) -> Fut,
    Fut: Future<Output = Result<(), JobError>>,
{
    log::info!("Consuming from queue {:?}", queue);
    let config = config().await;
    let delivery = match config.queue().consume_message_from_queue(queue.clone()).await {
        Ok(d) => d,
        Err(QueueError::NoData) => {
            return Ok(());
        }
        Err(e) => {
            return Err(ConsumptionError::FailedToConsumeFromQueue { error_msg: e.to_string() });
        }
    };
    let job_message: Option<JobQueueMessage> = delivery.payload_serde_json().wrap_err("Payload Serde Error ")?;

    match job_message {
        Some(job_message) => {
            log::info!("Handling job with id {:?} for queue {:?}", job_message.id, queue);
            match handler(job_message.id).await {
                Ok(_) => delivery.ack().await.map_err(|(e, _)| e).wrap_err("Queue Error ")?,
                Err(e) => {
                    log::error!("Failed to handle job with id {:?}. Error: {:?}", job_message.id, e);

                    // if the queue as a retry logic at the source, it will be attempted
                    // after the nack
                    match delivery.nack().await {
                        Ok(_) => Err(ConsumptionError::FailedToHandleJob {
                            job_id: job_message.id,
                            error_msg: "Job handling failed, message nack-ed".to_string(),
                        })?,
                        Err(delivery_nack_error) => Err(ConsumptionError::FailedToHandleJob {
                            job_id: job_message.id,
                            error_msg: delivery_nack_error.0.to_string(),
                        })?,
                    }
                }
            };
        }
        None => return Ok(()),
    };

    Ok(())
}

pub async fn init_consumers() -> Result<(), JobError> {
    // TODO: figure out a way to generalize this
    tokio::spawn(async move {
        loop {
            match consume_job_from_queue(JOB_PROCESSING_QUEUE.to_string(), process_job).await {
                Ok(_) => {}
                Err(e) => log::error!("Failed to consume from queue {:?}. Error: {:?}", JOB_PROCESSING_QUEUE, e),
            }
            sleep(Duration::from_secs(1)).await;
        }
    });
    tokio::spawn(async move {
        loop {
            match consume_job_from_queue(JOB_VERIFICATION_QUEUE.to_string(), verify_job).await {
                Ok(_) => {}
                Err(e) => log::error!("Failed to consume from queue {:?}. Error: {:?}", JOB_VERIFICATION_QUEUE, e),
            }
            sleep(Duration::from_secs(1)).await;
        }
    });
    Ok(())
}

async fn add_job_to_queue(id: Uuid, queue: String, delay: Option<Duration>) -> EyreResult<()> {
    let config = config().await;
    let message = JobQueueMessage { id };
    config.queue().send_message_to_queue(queue, serde_json::to_string(&message)?, delay).await?;
    Ok(())
}
