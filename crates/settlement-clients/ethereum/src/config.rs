use std::str::FromStr;

use settlement_client_interface::SettlementConfig;
use url::Url;
use utils::env_utils::get_env_var_or_panic;

pub const ENV_ETHEREUM_RPC_URL: &str = "ETHEREUM_RPC_URL";
pub const ENV_CORE_CONTRACT_ADDRESS: &str = "STARKNET_SOLIDITY_CORE_CONTRACT_ADDRESS";

#[derive(Clone, Debug)]
pub struct EthereumSettlementConfig {
    pub rpc_url: Url,
    pub core_contract_address: String,
}

impl SettlementConfig for EthereumSettlementConfig {
    fn new_from_env() -> Self {
        let rpc_url = get_env_var_or_panic(ENV_ETHEREUM_RPC_URL);
        let rpc_url = Url::from_str(&rpc_url).unwrap_or_else(|_| panic!("Failed to parse {}", ENV_ETHEREUM_RPC_URL));
        let core_contract_address = get_env_var_or_panic(ENV_CORE_CONTRACT_ADDRESS);
        Self { rpc_url, core_contract_address }
    }
}
