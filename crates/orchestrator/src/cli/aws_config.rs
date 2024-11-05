use clap::Args;

/// Parameters used to config AWS.
#[derive(Debug, Clone, Args)]
pub struct AWSConfigParams {
    /// The access key ID.
    #[arg(env = "AWS_ACCESS_KEY_ID", long)]
    pub access_key_id: String,

    /// The secret access key.
    #[arg(env = "AWS_SECRET_ACCESS_KEY", long)]
    pub secret_access_key: String,

    /// The region.
    #[arg(env = "AWS_REGION", long)]
    pub region: String,

    /// The endpoint URL.
    #[arg(env = "AWS_ENDPOINT_URL", long, default_value = "http://localhost.localstack.cloud:4566")]
    pub endpoint_url: String,

    /// The default region.
    #[arg(env = "AWS_DEFAULT_REGION", long, default_value = "localhost")]
    pub default_region: String,
}
