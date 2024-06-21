pub mod config;
pub mod conversion;

use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::Result;
use config::StarknetSettlementConfig;
use conversion::{slice_slice_u8_to_vec_field, slice_u8_to_field};
use mockall::{automock, predicate::*};
use settlement_client_interface::{SettlementClient, SettlementVerificationStatus};
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};

#[allow(unused)]
pub struct StarknetSettlementClient {
    pub provider: Arc<JsonRpcClient<HttpTransport>>,
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
        program_output: Vec<Vec<u8>>,
        onchain_data_hash: Vec<u8>,
        onchain_data_size: usize,
    ) -> Result<String> {
        let program_output = slice_slice_u8_to_vec_field(&program_output);
        let onchain_data_hash = slice_u8_to_field(&onchain_data_hash);
        Ok("external_id".to_string())
    }

    /// Should be used to update state on core contract when DA is in blobs/alt DA
    #[allow(unused)]
    async fn update_state_blobs(&self, program_output: Vec<Vec<u8>>, kzg_proof: Vec<u8>) -> Result<String> {
        let program_output = slice_slice_u8_to_vec_field(&program_output);
        let kzg_proof = slice_u8_to_field(&kzg_proof);
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
