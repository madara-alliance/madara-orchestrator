use crate::queue::sqs::AWSSQSParams;

pub mod aws_sqs;

#[derive(Clone, Debug)]
pub enum QueueParams {
    AWSSQS(AWSSQSParams),
}
