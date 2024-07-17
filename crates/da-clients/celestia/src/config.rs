use std::fs::File;
use std::path::PathBuf;

use serde::Deserialize;

use crate::DaMode;

pub const DEFAULT_CELESTIA_NODE: &str = "127.0.0.1:8000";
pub const DEFAULT_AUTH_TOKEN: &str = "";
pub const DEFAULT_NID: &str = "Karnot";

#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct CelestiaConfig {
    #[serde(default = "default_http")]
    pub http_provider: String,
    #[serde(default = "default_auth_token")]
    pub auth_token: String,
    #[serde(default = "default_nid")]
    pub nid: String,
    #[serde(default)]
    pub auth_token: Option<String>,
    #[serde(default)]
    pub mode: DaMode,
}

impl TryFrom<&PathBuf> for CelestiaConfig {
    type Error = String;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let file = File::open(path).map_err(|e| format!("error opening da config: {e}"))?;
        serde_json::from_reader(file).map_err(|e| format!("error parsing da config: {e}"))
    }
}

fn default_http() -> String {
    format!("http://{DEFAULT_CELESTIA_NODE}")
}

fn default_auth_token() -> String {
    format!("http://{DEFAULT_AUTH_TOKEN}")
}


fn default_nid() -> String {
    DEFAULT_NID.to_string()
}

impl Default for CelestiaConfig {
    fn default() -> Self {
        Self {
            http_provider: default_http(),
            auth_token: default_auth_token(),
            nid: default_nid(),
            mode: DaMode::default(),
            auth_token: None,
        }
    }
}