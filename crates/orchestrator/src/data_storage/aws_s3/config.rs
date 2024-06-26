use utils::env_utils::get_env_var_or_panic;

use crate::data_storage::DataStorageConfig;

pub struct AWSS3Config {
    pub s3_key_id: String,
    pub s3_key_secret: String,
    pub s3_bucket_name: String,
    pub s3_bucket_region: String,
}

impl DataStorageConfig for AWSS3Config {
    fn new_from_env() -> Self {
        Self {
            s3_key_id: get_env_var_or_panic("AWS_ACCESS_KEY_ID"),
            s3_key_secret: get_env_var_or_panic("AWS_SECRET_ACCESS_KEY"),
            s3_bucket_name: get_env_var_or_panic("AWS_S3_BUCKET_NAME"),
            s3_bucket_region: get_env_var_or_panic("AWS_S3_BUCKET_REGION"),
        }
    }
}
