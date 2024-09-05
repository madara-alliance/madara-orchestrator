use serde::{Deserialize, Serialize};
use utils::env_utils::get_env_var_or_panic;
use utils::settings::GetSettings;

#[derive(Clone, Serialize, Deserialize)]
pub struct AWSSNSConfig {
    /// AWS SNS ARN
    pub sns_arn: String,
    /// AWS SNS region
    pub sns_arn_region: String,
}

impl Default for AWSSNSConfig {
    fn default() -> Self {
        Self { sns_arn: get_env_var_or_panic("AWS_SNS_ARN"), sns_arn_region: get_env_var_or_panic("AWS_SNS_REGION") }
    }
}

impl GetSettings for AWSSNSConfig {
    fn get_settings() -> Self {
        Self { sns_arn: get_env_var_or_panic("AWS_SNS_ARN"), sns_arn_region: get_env_var_or_panic("AWS_SNS_REGION") }
    }
}
