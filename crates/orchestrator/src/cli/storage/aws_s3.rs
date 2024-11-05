use clap::Args;

/// Parameters used to config AWS S3.
#[derive(Debug, Clone, Args)]
pub struct AWSS3Params {
    /// The name of the S3 bucket.
    #[arg(env = "AWS_S3_BUCKET_NAME", long)]
    pub bucket_name: String,
}
