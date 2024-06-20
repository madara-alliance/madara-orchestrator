pub mod config;
pub mod conversion;

use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::Result;
use config::StarknetSettlementConfig;
use mockall::{automock, predicate::*};
use settlement_client_interface::{SettlementClient, SettlementVerificationStatus};
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};

#[allow(unused)]
pub struct StarknetSettlementClient {
    provider: Arc<JsonRpcClient<HttpTransport>>,
}

#[automock]
#[async_trait]
impl SettlementClient for StarknetSettlementClient {
    /// Should register the proof on the base layer and return an external id
    /// which can be used to track the status.
    #[allow(unused)]
    async fn register_proof(&self, proof: Vec<u8>) -> Result<String> {
        Ok("external_id".to_string())
    }

    /// Should be used to update state on core contract when DA is done in calldata
    #[allow(unused)]
    async fn update_state_calldata(
        &self,
        program_output: Vec<u8>,
        onchain_data_hash: u8,
        onchain_data_size: u8,
    ) -> Result<String> {
        // TODO: We need to figure out how to calculate onchain_data_hash and onchain_data_size here
        Ok("external_id".to_string())
    }

    /// Should be used to update state on core contract when DA is in blobs/alt DA
    #[allow(unused)]
    async fn update_state_blobs(&self, program_output: Vec<u8>, kzg_proof: Vec<u8>) -> Result<String> {
        // TODO: We need to figure out how to calculate kzg_proof here
        Ok("external_id".to_string())
    }

    /// Should verify the inclusion of the state diff in the DA layer and return the status
    #[allow(unused)]
    async fn verify_inclusion(&self, external_id: &str) -> Result<SettlementVerificationStatus> {
        Ok(SettlementVerificationStatus::Verified)
    }
}

impl From<StarknetSettlementConfig> for StarknetSettlementClient {
    fn from(config: StarknetSettlementConfig) -> Self {
        let provider = Arc::new(JsonRpcClient::new(HttpTransport::new(config.rpc_url)));
        StarknetSettlementClient { provider }
    }
}
