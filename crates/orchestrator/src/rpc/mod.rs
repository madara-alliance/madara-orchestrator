pub mod config;
pub mod l1;
pub mod madara;

use mockall::automock;
use serde::{Deserialize, Serialize};

use crate::rpc::config::HttpRpcConfig;

pub struct HttpRpcClient {
    client: reqwest::Client,
    l1_rpc_url: String,
    madara_rpc_url: String,
}

#[automock]
impl HttpRpcClient {
    pub fn new(config: HttpRpcConfig) -> Self {
        HttpRpcClient {
            client: reqwest::Client::new(),
            l1_rpc_url: config.l1_rpc_url,
            madara_rpc_url: config.madara_rpc_url,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: T,
}
