use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use url::Url;
use utils::env_utils::get_env_var_or_panic;

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
            service_url: get_env_var_or_panic("SHARP_URL").parse().unwrap(),
            rpc_node_url: get_env_var_or_panic("ETHEREUM_RPC_URL").parse().unwrap(),
            verifier_address: "0x07ec0D28e50322Eb0C159B9090ecF3aeA8346DFe".parse().unwrap(),
        }
    }
}
