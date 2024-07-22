use async_trait::async_trait;
use color_eyre::Result;
use mockall::automock;
use serde_json::json;
use snos::io::input::StarknetOsInput;
use starknet_api::block::BlockNumber;

use super::{HttpRpcClient, RpcResponse};

#[automock]
#[async_trait]
pub trait MadaraHttpRpcClient {
    async fn get_snos_input(&self, block_number: &BlockNumber) -> Result<StarknetOsInput>;
}

#[async_trait]
impl MadaraHttpRpcClient for HttpRpcClient {
    async fn get_snos_input(&self, block_number: &BlockNumber) -> Result<StarknetOsInput> {
        let rpc_request = json!(
            {
                "id": 1,
                "jsonrpc": "2.0",
                "method": "madara_getSnosInput",
                "params": [{"block_number": block_number.0}]
            }
        );
        let response: RpcResponse<StarknetOsInput> =
            self.client.post(&self.madara_rpc_url).json(&rpc_request).send().await?.json().await?;
        Ok(response.result)
    }
}
