use crate::settings::{GetSettings, SettingsProvider, SettingsProviderError};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, Default)]
pub struct EnvSettingsProvider {}

impl SettingsProvider for EnvSettingsProvider {
    fn get_default_settings<T: DeserializeOwned + Default>(
        &self,
        _section: &'static str,
    ) -> Result<T, SettingsProviderError> {
        todo!()
    }

    fn get_settings<T: GetSettings>(&self, _name: &'static str) -> Result<T, SettingsProviderError> {
        Ok(T::get_settings())
    }
}
