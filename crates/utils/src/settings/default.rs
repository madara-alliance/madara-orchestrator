use super::{GetSettings, SettingsProvider, SettingsProviderError};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Default)]
pub struct DefaultSettingsProvider {}

impl SettingsProvider for DefaultSettingsProvider {
    fn get_default_settings<T: DeserializeOwned + Default>(
        &self,
        _section: &'static str,
    ) -> Result<T, SettingsProviderError> {
        Ok(T::default())
    }

    fn get_settings<T: GetSettings>(&self, _name: &'static str) -> Result<T, SettingsProviderError> {
        todo!()
    }
}
