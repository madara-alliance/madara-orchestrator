use clap::Args;

/// Parameters used to config AWS SNS.
#[derive(Debug, Clone, Args)]
pub struct AWSSNSParams {
    /// The name of the S3 bucket.
    #[arg(env = "SNS_NAME", long, default_value = "madara-orchestrator-arn")]
    pub sns_arn: String,
}

impl AWSSNSParams {
    // TODO: Implement the logic to get the SNS ARN from aws config
}
