use std::fmt;

use clap::Args;
use serde::Serialize;

/// Parameters used to config AWS SQS.
#[derive(Debug, Clone, Args)]
pub struct AWSSQSParams {
    /// The name of the S3 bucket.
    #[arg(env = "SQS_PREFIX", long)]
    pub sqs_prefix: String,

    #[arg(env = "SQS_SUFFIX", long, default_value = "queue")]
    pub sqs_suffix: String,

    /// The QUEUE url
    #[arg(env = "SQS_BASE_QUEUE_URL", long)]
    pub queue_base_url: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum QueueType {
    #[serde(rename = "snos_job_processing")]
    SnosJobProcessing,
    #[serde(rename = "snos_job_verification")]
    SnosJobVerification,
    #[serde(rename = "proving_job_processing")]
    ProvingJobProcessing,
    #[serde(rename = "proving_job_verification")]
    ProvingJobVerification,
    #[serde(rename = "proof_registration_job_processing")]
    ProofRegistrationJobProcessing,
    #[serde(rename = "proof_registration_job_verification")]
    ProofRegistrationJobVerification,
    #[serde(rename = "data_submission_job_processing")]
    DataSubmissionJobProcessing,
    #[serde(rename = "data_submission_job_verification")]
    DataSubmissionJobVerification,
    #[serde(rename = "update_state_job_processing")]
    UpdateStateJobProcessing,
    #[serde(rename = "update_state_job_verification")]
    UpdateStateJobVerification,
    #[serde(rename = "job_handle_failure")]
    JobHandleFailure,
    #[serde(rename = "worker_trigger")]
    WorkerTrigger,
}

impl QueueType {
    pub fn iter() -> impl Iterator<Item = QueueType> {
        [
            QueueType::SnosJobProcessing,
            QueueType::SnosJobVerification,
            QueueType::ProvingJobProcessing,
            QueueType::ProvingJobVerification,
            QueueType::ProofRegistrationJobProcessing,
            QueueType::ProofRegistrationJobVerification,
            QueueType::DataSubmissionJobProcessing,
            QueueType::DataSubmissionJobVerification,
            QueueType::UpdateStateJobProcessing,
            QueueType::UpdateStateJobVerification,
            QueueType::JobHandleFailure,
            QueueType::WorkerTrigger,
        ].iter().cloned()
    }
}


impl fmt::Display for QueueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            _ => write!(f, "{}", serde_json::to_string(self).unwrap()),
        }
    }
}


impl AWSSQSParams {
    
    pub fn get_queue_url(&self, queue_type: QueueType) -> String {
        format!("{}/{}", self.queue_base_url, self.get_queue_name(queue_type))
    }

    pub fn get_queue_name(&self, queue_type: QueueType) -> String {
        // TODO: check if serde_json is the best way to convert the enum to string
        let queue_name = serde_json::to_string(&queue_type).unwrap();
        format!("{}_{}_{}", self.sqs_prefix, queue_name, self.sqs_suffix)
    }

}