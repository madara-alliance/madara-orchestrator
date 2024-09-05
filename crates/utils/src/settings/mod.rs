pub mod default;
pub mod env;

use serde::de::DeserializeOwned;

#[derive(Debug, thiserror::Error)]
pub enum SettingsProviderError {
    #[error("Internal settings error: {0}")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

pub trait GetSettings: DeserializeOwned {
    fn get_settings() -> Self;
}

pub trait SettingsProvider {
    fn get_default_settings<T: DeserializeOwned + Default>(
        &self,
        name: &'static str,
    ) -> Result<T, SettingsProviderError>;

    fn get_settings<T: GetSettings>(&self, name: &'static str) -> Result<T, SettingsProviderError>;
}
