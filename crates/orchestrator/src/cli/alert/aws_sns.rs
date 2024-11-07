use clap::Args;

/// Parameters used to config AWS SNS.
#[derive(Debug, Clone, Args)]
#[group(requires_all = ["sns_arn"])]
pub struct AWSSNSCliArgs {
    /// Use the AWS SNS client
    #[arg(long)]
    pub aws_sns: bool,

    /// The name of the S3 bucket.
    #[arg(env = "SNS_NAME", long, default_value = Some("madara-orchestrator-arn"))]
    pub sns_arn: Option<String>,
}
