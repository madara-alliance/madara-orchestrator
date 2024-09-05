use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use url::Url;
use utils::env_utils::get_env_var_or_panic;
use utils::settings::GetSettings;

use crate::client::DEFAULT_SHARP_URL;

/// SHARP proving service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharpConfig {
    /// SHARP service url
    pub service_url: Url,
    /// EVM RPC node url
    pub rpc_node_url: Url,
    /// GPS verifier contract address (implements FactRegistry)
    pub verifier_address: Address,
}

impl Default for SharpConfig {
    /// Default config for Sepolia testnet
    fn default() -> Self {
        Self {
            service_url: DEFAULT_SHARP_URL.parse().unwrap(),
            rpc_node_url: "https://ethereum-sepolia-rpc.publicnode.com".parse().unwrap(),
            verifier_address: "0x07ec0D28e50322Eb0C159B9090ecF3aeA8346DFe".parse().unwrap(),
        }
    }
}

impl GetSettings for SharpConfig {
    fn get_settings() -> Self {
        Self {
            service_url: get_env_var_or_panic("SHARP_URL").parse().unwrap(),
            rpc_node_url: get_env_var_or_panic("SETTLEMENT_RPC_URL").parse().unwrap(),
            verifier_address: get_env_var_or_panic("MEMORY_PAGES_CONTRACT_ADDRESS").parse().unwrap(),
        }
    }
}
