use utils::env_utils::get_env_var_or_panic;

use crate::data_storage::DataStorageConfig;

/// Represents AWS S3 config struct with all the necessary variables.
#[derive(Clone)]
pub struct AWSS3Config {
    /// S3 Bucket Name
    pub bucket_name: String,
}

/// Implementation of `DataStorageConfig` for `AWSS3Config`
impl DataStorageConfig for AWSS3Config {
    /// To return the config struct by creating it from the environment variables.
    fn new_from_env() -> Self {
        Self { bucket_name: get_env_var_or_panic("AWS_S3_BUCKET_NAME") }
    }
}

impl S3LocalStackConfig {
    pub fn new_from_env_with_endpoint(endpoint_url: &str) -> Self {
        Self {
            s3_key_id: get_env_var_or_panic("AWS_ACCESS_KEY_ID"),
            s3_key_secret: get_env_var_or_panic("AWS_SECRET_ACCESS_KEY"),
            s3_bucket_name: get_env_var_or_panic("AWS_S3_BUCKET_NAME"),
            s3_bucket_region: get_env_var_or_panic("AWS_S3_BUCKET_REGION"),
            endpoint_url: endpoint_url.to_string(),
        }
    }
}
