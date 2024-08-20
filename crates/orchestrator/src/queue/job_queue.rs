use std::future::Future;
use std::time::Duration;

use color_eyre::eyre::Context;
use color_eyre::Result as EyreResult;
use omniqueue::{Delivery, QueueError};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::log;
use uuid::Uuid;

use crate::config::config;
use crate::jobs::{handle_job_failure, process_job, verify_job, JobError, OtherError};

pub const JOB_PROCESSING_QUEUE: &str = "madara_orchestrator_job_processing_queue";
pub const JOB_VERIFICATION_QUEUE: &str = "madara_orchestrator_job_verification_queue";
// Below is the Data Letter Queue for the the above two jobs.
pub const JOB_HANDLE_FAILURE_QUEUE: &str = "madara_orchestrator_job_handle_failure_queue";

// Queues for SNOS worker trigger listening
pub const SNOS_WORKER_TRIGGER_QUEUE: &str = "madara_orchestrator_snos_worker_trigger_queue";
pub const PROVING_WORKER_TRIGGER_QUEUE: &str = "madara_orchestrator_proving_worker_trigger_queue";
pub const PROOF_REGISTRATION_WORKER_TRIGGER_QUEUE: &str = "madara_orchestrator_proof_registration_worker_trigger_queue";
pub const DATA_SUBMISSION_WORKER_TRIGGER_QUEUE: &str = "madara_orchestrator_data_submission_worker_trigger_queue";
pub const UPDATE_STATE_WORKER_TRIGGER_QUEUE: &str = "madara_orchestrator_update_state_worker_trigger_queue";

use crate::workers::data_submission_worker::DataSubmissionWorker;
use crate::workers::proof_registration::ProofRegistrationWorker;
use crate::workers::proving::ProvingWorker;
use crate::workers::snos::SnosWorker;
use crate::workers::update_state::UpdateStateWorker;
use crate::workers::Worker;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ConsumptionError {
    #[error("Failed to consume message from queue, error {error_msg:?}")]
    FailedToConsumeFromQueue { error_msg: String },

    #[error("Failed to handle job with id {job_id:?}. Error: {error_msg:?}")]
    FailedToHandleJob { job_id: Uuid, error_msg: String },

    #[error("Failed to spawn {worker_trigger_type:?} worker. Error: {error_msg:?}")]
    FailedToSpawnWorker { worker_trigger_type: WorkerTriggerType, error_msg: String },

    #[error("Other error: {0}")]
    Other(#[from] OtherError),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobQueueMessage {
    pub(crate) id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum WorkerTriggerType {
    Snos,
    Proving,
    ProofRegistration,
    DataSubmission,
    UpdateState,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkerTriggerMessage {
    pub(crate) worker: WorkerTriggerType,
}

enum DeliveryReturnType {
    Message(Delivery),
    NoMessage,
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
    let delivery = get_delivery_from_queue(queue.clone()).await?;

    match delivery {
        DeliveryReturnType::Message(d) => {
            let job_message: Option<JobQueueMessage> = d
                .payload_serde_json()
                .wrap_err("Payload Serde Error")
                .map_err(|e| ConsumptionError::Other(OtherError::from(e)))?;

            match job_message {
                Some(job_message) => {
                    log::info!("Handling job with id {:?} for queue {:?}", job_message.id, queue);
                    match handler(job_message.id).await {
                        Ok(_) => d
                            .ack()
                            .await
                            .map_err(|(e, _)| e)
                            .wrap_err("Queue Error")
                            .map_err(|e| ConsumptionError::Other(OtherError::from(e)))?,
                        Err(e) => {
                            log::error!("Failed to handle job with id {:?}. Error: {:?}", job_message.id, e);

                            // if the queue as a retry logic at the source, it will be attempted
                            // after the nack
                            match d.nack().await {
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
            }
        }
        DeliveryReturnType::NoMessage => return Ok(()),
    }

    Ok(())
}

/// Function to consume the message from the worker trigger queues and spawn the worker
/// for respective message received.
pub async fn consume_worker_trigger_messages_from_queue<F, Fut>(
    queue: String,
    handler: F,
) -> Result<(), ConsumptionError>
where
    F: FnOnce(Box<dyn Worker>) -> Fut,
    Fut: Future<Output = color_eyre::Result<()>>,
{
    log::info!("Consuming from queue {:?}", queue);
    let delivery = get_delivery_from_queue(queue.clone()).await?;

    match delivery {
        DeliveryReturnType::Message(d) => {
            let job_message: Option<WorkerTriggerMessage> = d
                .payload_serde_json()
                .wrap_err("Payload Serde Error")
                .map_err(|e| ConsumptionError::Other(OtherError::from(e)))?;

            match job_message {
                Some(job_message) => {
                    log::info!("Handling worker trigger for worker type : {:?}", job_message.worker);
                    let worker_handler = get_worker_handler_from_worker_trigger_type(job_message.worker.clone());
                    match handler(worker_handler).await {
                        Ok(_) => d
                            .ack()
                            .await
                            .map_err(|(e, _)| e)
                            .wrap_err("Queue Error")
                            .map_err(|e| ConsumptionError::Other(OtherError::from(e)))?,
                        Err(e) => {
                            log::error!("Failed to handle worker trigger {:?}. Error: {:?}", job_message.worker, e);
                            // if the queue as a retry logic at the source, it will be attempted
                            // after the nack
                            match d.nack().await {
                                Ok(_) => Err(ConsumptionError::FailedToSpawnWorker {
                                    worker_trigger_type: job_message.worker,
                                    error_msg: "Job handling failed, message nack-ed".to_string(),
                                })?,
                                Err(delivery_nack_error) => Err(ConsumptionError::FailedToSpawnWorker {
                                    worker_trigger_type: job_message.worker,
                                    error_msg: delivery_nack_error.0.to_string(),
                                })?,
                            }
                        }
                    };
                }
                None => return Ok(()),
            }
        }
        DeliveryReturnType::NoMessage => return Ok(()),
    }

    Ok(())
}

/// To get Box<dyn Worker> handler from `WorkerTriggerType`.
fn get_worker_handler_from_worker_trigger_type(worker_trigger_type: WorkerTriggerType) -> Box<dyn Worker> {
    match worker_trigger_type {
        WorkerTriggerType::Snos => Box::new(SnosWorker),
        WorkerTriggerType::Proving => Box::new(ProvingWorker),
        WorkerTriggerType::DataSubmission => Box::new(DataSubmissionWorker),
        WorkerTriggerType::ProofRegistration => Box::new(ProofRegistrationWorker),
        WorkerTriggerType::UpdateState => Box::new(UpdateStateWorker),
    }
}

/// To get the delivery from the message queue using the queue name
async fn get_delivery_from_queue(queue: String) -> Result<DeliveryReturnType, ConsumptionError> {
    match config().await.queue().consume_message_from_queue(queue.clone()).await {
        Ok(d) => Ok(DeliveryReturnType::Message(d)),
        Err(QueueError::NoData) => Ok(DeliveryReturnType::NoMessage),
        Err(e) => Err(ConsumptionError::FailedToConsumeFromQueue { error_msg: e.to_string() }),
    }
}

macro_rules! spawn_consumer {
    ($queue_type :expr, $handler : expr) => {
        tokio::spawn(async move {
            loop {
                match consume_job_from_queue($queue_type, $handler).await {
                    Ok(_) => {}
                    Err(e) => log::error!("Failed to consume from queue {:?}. Error: {:?}", $queue_type, e),
                }
                sleep(Duration::from_secs(1)).await;
            }
        });
    };
}

macro_rules! spawn_worker_trigger_consumer {
    ($queue_type: expr, $handler: expr) => {
        tokio::spawn(async move {
            loop {
                match consume_worker_trigger_messages_from_queue($queue_type, $handler).await {
                    Ok(_) => {}
                    Err(e) => log::error!("Failed to consume from queue {:?}. Error: {:?}", $queue_type, e),
                }
                sleep(Duration::from_secs(1)).await;
            }
        })
    };
}

pub async fn init_consumers() -> Result<(), JobError> {
    spawn_consumer!(JOB_PROCESSING_QUEUE.to_string(), process_job);
    spawn_consumer!(JOB_VERIFICATION_QUEUE.to_string(), verify_job);
    spawn_consumer!(JOB_HANDLE_FAILURE_QUEUE.to_string(), handle_job_failure);
    spawn_worker_trigger_consumer!(SNOS_WORKER_TRIGGER_QUEUE.to_string(), spawn_worker);
    spawn_worker_trigger_consumer!(PROVING_WORKER_TRIGGER_QUEUE.to_string(), spawn_worker);
    spawn_worker_trigger_consumer!(DATA_SUBMISSION_WORKER_TRIGGER_QUEUE.to_string(), spawn_worker);
    spawn_worker_trigger_consumer!(PROOF_REGISTRATION_WORKER_TRIGGER_QUEUE.to_string(), spawn_worker);
    spawn_worker_trigger_consumer!(UPDATE_STATE_WORKER_TRIGGER_QUEUE.to_string(), spawn_worker);
    Ok(())
}

/// To spawn the worker by passing the worker struct
async fn spawn_worker(worker: Box<dyn Worker>) -> color_eyre::Result<()> {
    worker.run_worker_if_enabled().await.expect("Error in running the worker.");
    Ok(())
}

async fn add_job_to_queue(id: Uuid, queue: String, delay: Option<Duration>) -> EyreResult<()> {
    let config = config().await;
    let message = JobQueueMessage { id };
    config.queue().send_message_to_queue(queue, serde_json::to_string(&message)?, delay).await?;
    Ok(())
}
