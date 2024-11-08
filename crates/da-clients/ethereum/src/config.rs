use std::str::FromStr;

use alloy::network::Ethereum;
use alloy::providers::ProviderBuilder;
use alloy::rpc::client::RpcClient;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::EthereumDaClient;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EthereumDaConfig {
    pub rpc_url: String,
}

impl EthereumDaConfig {
    pub fn new_with_params(ethereum_da_params: &EthereumDaParams) -> color_eyre::Result<Self> {
        Ok(Self { rpc_url: ethereum_da_params.ethereum_da_rpc_url.to_string() })
    }

    pub async fn build_client(&self) -> EthereumDaClient {
        let client =
            RpcClient::new_http(Url::from_str(self.rpc_url.as_str()).expect("Failed to parse SETTLEMENT_RPC_URL"));
        let provider = ProviderBuilder::<_, Ethereum>::new().on_client(client);

        EthereumDaClient { provider }
    }
}

#[derive(Debug, Clone)]
pub struct EthereumDaParams {
    pub ethereum_da_rpc_url: Url,
}
