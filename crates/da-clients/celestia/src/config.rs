use da_client_interface::DaConfig;
use async_trait::async_trait;
use serde::Deserialize;
use dotenv::dotenv;
use celestia_rpc::Client;
use utils::env_utils::{get_env_car_optional_or_panic, get_env_var_or_panic};

#[derive(Clone, PartialEq, Deserialize, Debug)]
pub struct CelestiaDaConfig {
    pub http_provider: String,
    pub auth_token: Option<String>,
    pub nid: String,
}


#[async_trait]
impl DaConfig<Client> for CelestiaDaConfig {
    // TODO: Possibility to merge these two ?
    fn new_from_env() -> Self {
        dotenv().ok();
        Self {
            http_provider: get_env_var_or_panic("CELESTIA_DA_RPC_URL"),
            auth_token: get_env_car_optional_or_panic("CELESTIA_DA_AUTH_TOKEN"),
            nid: get_env_var_or_panic("CELESTIA_DA_NID"),
            
        }
    }
    async fn build_da_client(&self) -> Client{
        Client::new(&self.http_provider, self.auth_token.as_deref()).await.expect("Failed to create Client: ")
    }
} 