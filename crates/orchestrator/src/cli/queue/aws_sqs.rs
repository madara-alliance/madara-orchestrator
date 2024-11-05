use clap::Args;

/// Parameters used to config AWS SQS.
#[derive(Debug, Clone, Args)]
pub struct AWSSQSParams {
    /// The name of the S3 bucket.
    #[arg(env = "SQS_PREFIX", long)]
    pub sqs_prefix: String,

    /// The QUEUE url
    #[arg(env = "SQS_QUEUE_URL", long)]
    pub queue_url: String,
}


impl AWSSQSParams {
    // TODO: Implement the logic to get the queue url
    // SQS_SNOS_JOB_PROCESSING_QUEUE_URL="http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_snos_job_processing_queue"
    // SQS_SNOS_JOB_VERIFICATION_QUEUE_URL="http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_snos_job_verification_queue"

    // SQS_PROVING_JOB_PROCESSING_QUEUE_URL="http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_proving_job_processing_queue"
    // SQS_PROVING_JOB_VERIFICATION_QUEUE_URL="http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_proving_job_verification_queue"

    // SQS_DATA_SUBMISSION_JOB_PROCESSING_QUEUE_URL="http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_data_submission_job_processing_queue"
    // SQS_DATA_SUBMISSION_JOB_VERIFICATION_QUEUE_URL="http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_data_submission_job_verification_queue"

    // SQS_UPDATE_STATE_JOB_PROCESSING_QUEUE_URL="http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_update_state_job_processing_queue"
    // SQS_UPDATE_STATE_JOB_VERIFICATION_QUEUE_URL="http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_update_state_job_verification_queue"

    // SQS_JOB_HANDLE_FAILURE_QUEUE_URL="http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_job_handle_failure_queue"
    // SQS_WORKER_TRIGGER_QUEUE_URL="http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_worker_trigger_queue"

}