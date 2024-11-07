use clap::Args;

/// Parameters used to config AWS S3.
#[derive(Debug, Clone, Args)]
#[group(requires_all = ["bucket_name"])]
pub struct AWSS3CliArgs {
    /// Use the AWS s3 client
    #[arg(long)]
    pub aws_s3: bool,

    /// The name of the S3 bucket.
    #[arg(env = "AWS_S3_BUCKET_NAME", long, default_value = Some("madara-orchestrator-bucket"))]
    pub bucket_name: Option<String>,
}
