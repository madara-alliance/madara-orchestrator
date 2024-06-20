pub mod config;

use async_trait::async_trait;
use mockall::{automock, predicate::*};
use settlement_client_interface::{SettlementClient, SettlementVerificationStatus};

use color_eyre::Result;

#[allow(dead_code)]
pub struct EthereumSettlementClient {}

#[automock]
#[async_trait]
impl SettlementClient for EthereumSettlementClient {
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
        Ok("external_id".to_string())
    }

    /// Should be used to update state on core contract when DA is in blobs/alt DA
    #[allow(unused)]
    async fn update_state_blobs(&self, program_output: Vec<u8>, kzg_proof: Vec<u8>) -> Result<String> {
        Ok("external_id".to_string())
    }

    /// Should verify the inclusion of the state diff in the DA layer and return the status
    #[allow(unused)]
    async fn verify_inclusion(&self, external_id: &str) -> Result<SettlementVerificationStatus> {
        Ok(SettlementVerificationStatus::Verified)
    }
}
