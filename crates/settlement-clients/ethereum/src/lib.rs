pub mod clients;
pub mod config;
pub mod interfaces;
pub mod types;

use alloy::{
    network::EthereumWallet,
    primitives::{Address, B160},
    providers::{ProviderBuilder, RootProvider},
    rpc::client::RpcClient,
    signers::{
        k256::ecdsa::{SigningKey, VerifyingKey},
        local::PrivateKeySigner,
    },
    transports::http::Http,
};
use async_trait::async_trait;
use color_eyre::Result;
use config::EthereumSettlementConfig;
use mockall::{automock, predicate::*};
use reqwest::Client;
use settlement_client_interface::{SettlementClient, SettlementVerificationStatus};
use snos::io::output::StarknetOsOutput;
use std::sync::Arc;
use utils::env_utils::get_env_var_or_panic;

use crate::clients::StarknetValidityContractClient;
use crate::types::LocalWalletSignerMiddleware;

#[allow(dead_code)]
pub struct EthereumSettlementClient {
    core_contract_client: StarknetValidityContractClient,
    memory_pages_contract: String,
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
        program_output: Vec<Vec<u8>>,
        onchain_data_hash: Vec<u8>,
        onchain_data_size: usize,
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
        // TODO: VERY INSECURE ⚠⚠⚠⚠⚠
        let private_key = get_env_var_or_panic("PK");
        let signer: PrivateKeySigner = private_key.parse().expect("Failed to parse private key");
        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new().with_recommended_fillers().wallet(wallet).on_http(config.rpc_url);
        let core_contract_client = StarknetValidityContractClient::new(
            B160::from_slice(config.core_contract.as_bytes()).0.into(),
            Arc::new(provider),
        );

        EthereumSettlementClient { memory_pages_contract: config.memory_pages_contract, core_contract_client }
    }
}
