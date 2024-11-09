use std::time::Duration;

use async_trait::async_trait;
use color_eyre::Result;
use omniqueue::backends::{SqsBackend, SqsConfig, SqsConsumer, SqsProducer};
use omniqueue::{Delivery, QueueError};
use serde::Serialize;

use super::job_queue::QueueType;
use crate::queue::QueueProvider;

#[derive(Debug, Clone, Serialize)]
pub struct AWSSQSParams {
    pub queue_base_url: String,
    pub sqs_prefix: String,
    pub sqs_suffix: String,
}

impl AWSSQSParams {
    pub fn get_queue_url(&self, queue_type: QueueType) -> String {
        let name = format!("{}/{}", self.queue_base_url, self.get_queue_name(queue_type));
        name
    }

    pub fn get_queue_name(&self, queue_type: QueueType) -> String {
        format!("{}_{}_{}", self.sqs_prefix, queue_type, self.sqs_suffix)
    }
}

pub struct SqsQueue {
    pub params: AWSSQSParams,
}

impl SqsQueue {
    pub fn new_with_params(params: AWSSQSParams) -> Self {
        Self { params }
    }
}

#[async_trait]
impl QueueProvider for SqsQueue {
    async fn send_message_to_queue(&self, queue: QueueType, payload: String, delay: Option<Duration>) -> Result<()> {
        let queue_url = self.params.get_queue_url(queue);
        let producer = get_producer(queue_url).await?;

        match delay {
            Some(d) => producer.send_raw_scheduled(payload.as_str(), d).await?,
            None => producer.send_raw(payload.as_str()).await?,
        }

        Ok(())
    }

    async fn consume_message_from_queue(&self, queue: QueueType) -> std::result::Result<Delivery, QueueError> {
        let queue_url = self.params.get_queue_url(queue);
        let mut consumer = get_consumer(queue_url).await?;
        consumer.receive().await
    }
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
