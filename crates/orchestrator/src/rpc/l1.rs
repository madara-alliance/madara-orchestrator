use async_trait::async_trait;
use color_eyre::Result;
use mockall::automock;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{HttpRpcClient, RpcResponse};

const FEE_HISTORY_BLOCK_COUNT: u64 = 300;

#[automock]
#[async_trait]
pub trait L1HttpRpcRequests {
    async fn fee_history(&self) -> Result<EthFeeHistory>;
}

#[async_trait]
impl L1HttpRpcRequests for HttpRpcClient {
    async fn fee_history(&self) -> Result<EthFeeHistory> {
        let rpc_request = json!(
            {
                "id": 83,
                "jsonrpc": "2.0",
                "method": "eth_feeHistory",
                // We choose 300 to get average gas caprice for last one
                // hour (300 * 12 sec block time).
                // TODO: "300" may need to be a parameter + adjusted depending on block time
                "params": [FEE_HISTORY_BLOCK_COUNT, "latest", []],
            }
        );

        let response: RpcResponse<EthFeeHistory> =
            self.client.post(&self.l1_rpc_url).json(&rpc_request).send().await?.json().await?;

        return Ok(response.result);
    }
}

// Reference: https://docs.alchemy.com/reference/eth-feehistory
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthFeeHistory {
    /// An array of block base fees per gas.
    /// This includes the next block after the newest of the returned range,
    /// because this value can be derived from the newest block. Zeroes are
    /// returned for pre-EIP-1559 blocks.
    ///
    /// # Note
    ///
    /// Empty list is skipped only for compatibility with Erigon and Geth.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base_fee_per_gas: Vec<String>,
    /// An array of block gas used ratios. These are calculated as the ratio
    /// of `gasUsed` and `gasLimit`.
    pub gas_used_ratio: Vec<f64>,
    /// An array of block base fees per blob gas. This includes the next block after the newest
    /// of  the returned range, because this value can be derived from the newest block. Zeroes
    /// are returned for pre-EIP-4844 blocks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub base_fee_per_blob_gas: Vec<String>,
    /// An array of block blob gas used ratios. These are calculated as the ratio of gasUsed and
    /// gasLimit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blob_gas_used_ratio: Vec<f64>,
    /// Lowest number block of the returned range.
    pub oldest_block: String,
    /// An (optional) array of effective priority fee per gas data points from a single
    /// block. All zeroes are returned if the block is empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reward: Option<Vec<Vec<u128>>>,
}
