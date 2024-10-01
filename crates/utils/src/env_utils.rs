use std::env::VarError;
use tracing::Level;

pub fn get_env_var(key: &str) -> Result<String, VarError> {
    std::env::var(key)
}

pub fn get_env_var_or_panic(key: &str) -> String {
    get_env_var(key).unwrap_or_else(|e| panic!("Failed to get env var {}: {}", key, e))
}

pub fn get_env_var_or_default(key: &str, default: &str) -> String {
    get_env_var(key).unwrap_or(default.to_string())
}

pub fn get_env_var_optional(key: &str) -> Result<Option<String>, VarError> {
    match get_env_var(key) {
        Ok(s) => Ok(Some(s)),
        Err(VarError::NotPresent) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn get_env_car_optional_or_panic(key: &str) -> Option<String> {
    get_env_var_optional(key).unwrap_or_else(|e| panic!("Failed to get env var {}: {}", key, e))
}

// We default to INFO if the tracing level env is not set properly
pub fn get_tracing_level_from_string(key: &str) -> Level {
    match key.to_ascii_lowercase().as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    }
}
