use clap::Args;

/// Parameters used to config AWS SQS.
#[derive(Debug, Clone, Args)]
#[group(requires_all = ["sqs_prefix", "sqs_suffix", "queue_base_url"])]
pub struct AWSSQSCliArgs {
    /// Use the AWS sqs client
    #[arg(long)]
    pub aws_sqs: bool,

    /// The name of the S3 bucket.
    #[arg(env = "SQS_PREFIX", long, default_value = Some("madara_orchestrator"))]
    pub sqs_prefix: Option<String>,

    /// The suffix of the queue.    
    #[arg(env = "SQS_SUFFIX", long, default_value = Some("queue"))]
    pub sqs_suffix: Option<String>,

    /// The QUEUE url
    #[arg(env = "SQS_BASE_QUEUE_URL", long)]
    pub queue_base_url: Option<String>,
}
