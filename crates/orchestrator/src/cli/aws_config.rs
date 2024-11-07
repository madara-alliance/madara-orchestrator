use clap::Args;
use serde::Serialize;

/// Parameters used to config AWS.
#[derive(Debug, Clone, Args, Serialize)]
#[group(requires_all = ["aws_access_key_id", "aws_secret_access_key", "aws_region"])]
pub struct AWSConfigParams {
    /// The access key ID.
    #[arg(env = "AWS_ACCESS_KEY_ID", long)]
    pub aws_access_key_id: String,

    /// The secret access key.
    #[arg(env = "AWS_SECRET_ACCESS_KEY", long)]
    pub aws_secret_access_key: String,

    /// The region.
    #[arg(env = "AWS_REGION", long)]
    pub aws_region: String,

    /// The endpoint URL.
    #[arg(env = "AWS_ENDPOINT_URL", long, default_value = "http://localhost.localstack.cloud:4566")]
    pub aws_endpoint_url: String,

    /// The default region.
    #[arg(env = "AWS_DEFAULT_REGION", long, default_value = "localhost")]
    pub aws_default_region: String,
}
