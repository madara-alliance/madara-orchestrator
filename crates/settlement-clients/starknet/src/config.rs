use std::str::FromStr;

use settlement_client_interface::SettlementConfig;
use url::Url;
use utils::env_utils::get_env_var_or_panic;

// TODO(akhercha): do we use madara rpc or another starknet rpc?
pub const ENV_STARKNET_RPC_URL: &str = "MADARA_RPC_URL";

pub struct StarknetSettlementConfig {
    pub rpc_url: Url,
}

impl SettlementConfig for StarknetSettlementConfig {
    /// Should create a new instance of the DaConfig from the environment variables
    fn new_from_env() -> Self {
        let rpc_url = get_env_var_or_panic(ENV_STARKNET_RPC_URL);
        let rpc_url = Url::from_str(&rpc_url).unwrap_or_else(|_| panic!("Failed to parse {}", ENV_STARKNET_RPC_URL));
        StarknetSettlementConfig { rpc_url }
    }
}
