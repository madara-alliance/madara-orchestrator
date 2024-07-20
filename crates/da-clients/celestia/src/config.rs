use da_client_interface::DaConfig;
use async_trait::async_trait;
use serde::Deserialize;
use dotenv::dotenv;
use celestia_rpc::Client;
use utils::env_utils::{get_env_car_optional_or_panic, get_env_var_or_panic};
use celestia_types::nmt::Namespace;

use crate::{error::CelestiaDaError, CelestiaDaClient};

#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct CelestiaDaConfig {
    pub http_provider: String,
    pub auth_token: Option<String>,
    pub nid: String,
}


#[async_trait]
impl DaConfig<CelestiaDaClient> for CelestiaDaConfig {
    fn new_from_env() -> Self {
        dotenv().ok();
        Self {
            http_provider: get_env_var_or_panic("CELESTIA_DA_RPC_URL"),
            auth_token: get_env_car_optional_or_panic("CELESTIA_DA_AUTH_TOKEN"),
            nid: get_env_var_or_panic("CELESTIA_DA_NID"),
        }
    }
    async fn build_client(&self) -> CelestiaDaClient{
        let bytes = self.nid.as_bytes();

        let nid = Namespace::new_v0(bytes)
        .map_err(|e| CelestiaDaError::Generic(format!("could not init namespace: {e}")))
        .unwrap();
        let celestia_da_client = Client::new(&self.http_provider, self.auth_token.as_deref()).await.expect("Failed to create Client: ");

        CelestiaDaClient {
            client : celestia_da_client,
            nid
        }
    }
} 