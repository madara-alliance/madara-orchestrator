use std::str::FromStr;

use serde::{Deserialize, Serialize};
use settlement_client_interface::SettlementConfig;
use url::Url;
use utils::env_utils::get_env_var_or_panic;

pub const ENV_STARKNET_RPC_URL: &str = "STARKNET_RPC_URL";
pub const ENV_CORE_CONTRACT_ADDRESS: &str = "STARKNET_CAIRO_CORE_CONTRACT_ADDRESS";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarknetSettlementConfig {
    pub rpc_url: Url,
    pub core_contract_address: String,
}

impl SettlementConfig for StarknetSettlementConfig {
    /// Should create a new instance of the DaConfig from the environment variables
    fn new_from_env() -> Self {
        let rpc_url = get_env_var_or_panic(ENV_STARKNET_RPC_URL);
        let rpc_url = Url::from_str(&rpc_url).unwrap_or_else(|_| panic!("Failed to parse {}", ENV_STARKNET_RPC_URL));
        let core_contract_address = get_env_var_or_panic(ENV_CORE_CONTRACT_ADDRESS);
        StarknetSettlementConfig { rpc_url, core_contract_address }
    }
}

impl Default for StarknetSettlementConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://free-rpc.nethermind.io/sepolia-juno".parse().unwrap(),
            core_contract_address: "TODO:https://github.com/keep-starknet-strange/piltover".into(),
        }
    }
}
