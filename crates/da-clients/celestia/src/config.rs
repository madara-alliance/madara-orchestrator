use da_client_interface::DaConfig;
use std::fs::File;
use std::path::PathBuf;
use utils::env_utils::get_env_var_or_panic;
use serde::Deserialize;
use dotenv::dotenv;
#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct CelestiaDaConfig {
    pub http_provider: String,
    pub auth_token: Option<String>,
    pub nid: String,
}

impl TryFrom<&PathBuf> for CelestiaDaConfig {
    type Error = String;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(path).map_err(|e| format!("error opening da config: {e}"))?;
        serde_json::from_reader(file).map_err(|e| format!("error parsing da config: {e}"))
    }
}

impl DaConfig for CelestiaDaConfig {
    fn new_from_env() -> Self {
        dotenv().ok();
        Self {
            http_provider: get_env_var_or_panic("CELESTIA_DA_RPC_URL"),
            auth_token: Some(get_env_var_or_panic("CELESTIA_DA_AUTH_TOKEN")),
            nid: get_env_var_or_panic("CELESTIA_DA_NID"),
        }
    }
}