use utils::env_utils::get_env_var_or_panic;

pub struct HttpRpcConfig {
    pub l1_rpc_url: String,
    pub madara_rpc_url: String,
}

impl HttpRpcConfig {
    pub fn new_from_env() -> Self {
        let l1_rpc_url = get_env_var_or_panic("ETHEREUM_RPC_URL");
        let madara_rpc_url = get_env_var_or_panic("MADARA_RPC_URL");
        Self { l1_rpc_url, madara_rpc_url }
    }
}
