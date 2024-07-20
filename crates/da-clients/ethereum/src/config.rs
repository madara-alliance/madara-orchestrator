use da_client_interface::DaConfig;
use utils::env_utils::get_env_var_or_panic;
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct EthereumDaConfig {
    pub rpc_url: String,
    pub memory_pages_contract: String,
    pub private_key: String,
}

#[async_trait]
impl DaConfig<String> for EthereumDaConfig {
    fn new_from_env() -> Self {
        Self {
            rpc_url: get_env_var_or_panic("ETHEREUM_RPC_URL"),
            memory_pages_contract: get_env_var_or_panic("MEMORY_PAGES_CONTRACT_ADDRESS"),
            private_key: get_env_var_or_panic("PRIVATE_KEY"),
        }
    } 
    async fn build_da_client(&self) -> String{
        "Create Ethereum Client here".to_string()
    }
}
