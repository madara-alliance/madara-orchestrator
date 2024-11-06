pub mod aws_sqs;

#[derive(Clone, Debug)]
pub enum QueueParams {
    AWSSQS(aws_sqs::AWSSQSParams),
}
