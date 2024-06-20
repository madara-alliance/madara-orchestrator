pub mod config;

use alloy::{rpc::client::RpcClient, transports::http::Http};
use async_trait::async_trait;
use config::EthereumSettlementConfig;
use mockall::{automock, predicate::*};
use reqwest::Client;
use settlement_client_interface::{SettlementClient, SettlementVerificationStatus};
use starknet_core_contract_client::clients::StarknetValidityContractClient;

use color_eyre::Result;

#[allow(dead_code)]
pub struct EthereumSettlementClient {
    provider: RpcClient<Http<Client>>,
}

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

impl From<EthereumSettlementConfig> for EthereumSettlementClient {
    fn from(config: EthereumSettlementConfig) -> Self {
        let provider = RpcClient::builder().reqwest_http(config.rpc_url);
        EthereumSettlementClient { provider }
    }
}
