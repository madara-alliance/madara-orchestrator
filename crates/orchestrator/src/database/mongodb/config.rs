use serde::{Deserialize, Serialize};
use utils::env_utils::get_env_var_or_panic;
use utils::settings::GetSettings;

use crate::database::DatabaseConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct MongoDbConfig {
    pub url: String,
}

impl DatabaseConfig for MongoDbConfig {
    fn new_from_env() -> Self {
        Self { url: get_env_var_or_panic("MONGODB_CONNECTION_STRING") }
    }
}

impl Default for MongoDbConfig {
    fn default() -> Self {
        Self { url: get_env_var_or_panic("MONGODB_CONNECTION_STRING") }
    }
}

impl GetSettings for MongoDbConfig {
    fn get_settings() -> Self {
        Self { url: get_env_var_or_panic("MONGODB_CONNECTION_STRING") }
    }
}
