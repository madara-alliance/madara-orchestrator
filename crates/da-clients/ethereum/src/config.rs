use serde::{Deserialize, Serialize};
use utils::env_utils::get_env_var_or_panic;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EthereumDaConfig {
    pub rpc_url: String,
    pub memory_pages_contract: String,
    pub private_key: String,
}

impl Default for EthereumDaConfig {
    /// Default config for Sepolia testnet
    fn default() -> Self {
        Self {
            rpc_url: get_env_var_or_panic("SETTLEMENT_RPC_URL"),
            memory_pages_contract: get_env_var_or_panic("MEMORY_PAGES_CONTRACT_ADDRESS"),
            private_key: get_env_var_or_panic("PRIVATE_KEY"),
        }
    }
}
