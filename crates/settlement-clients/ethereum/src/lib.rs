pub mod clients;
pub mod config;
pub mod conversion;
pub mod types;

use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use async_trait::async_trait;
use color_eyre::Result;
use config::EthereumSettlementConfig;
use conversion::{slice_slice_u8_to_vec_u256, slice_u8_to_u256};
use mockall::{automock, predicate::*};
use settlement_client_interface::{parse_and_validate_block_order, SettlementClient, SettlementVerificationStatus};
use std::sync::Arc;
use utils::env_utils::get_env_var_or_panic;

use crate::clients::interfaces::validity_interface::StarknetValidityContractTrait;
use crate::clients::StarknetValidityContractClient;

pub const ENV_PRIVATE_KEY: &str = "ETHEREUM_PRIVATE_KEY";

#[allow(dead_code)]
pub struct EthereumSettlementClient {
    core_contract_client: StarknetValidityContractClient,
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
        let program_output: Vec<U256> = slice_slice_u8_to_vec_u256(&program_output);
        let onchain_data_hash: U256 = slice_u8_to_u256(&onchain_data_hash);
        self.core_contract_client.update_state(program_output, onchain_data_hash, U256::from(0)).await?;
        Ok("TODO".to_string())
    }

    /// Should be used to update state on core contract when DA is in blobs/alt DA
    #[allow(unused)]
    async fn update_state_blobs(&self, program_output: Vec<Vec<u8>>, kzg_proof: Vec<u8>) -> Result<String> {
        let program_output: Vec<U256> = slice_slice_u8_to_vec_u256(&program_output);
        self.core_contract_client.update_state_kzg(program_output, kzg_proof).await?;
        Ok("TODO".to_string())
    }

    /// Should verify the inclusion of the state diff in the DA layer and return the status
    #[allow(unused)]
    async fn verify_inclusion(&self, external_id: &str) -> Result<SettlementVerificationStatus> {
        let last_block_settled = self.get_last_settled_block().await?;
        // We assume here that the external_id is the list of blocks comma separated
        let block_numbers: Vec<u64> = parse_and_validate_block_order(external_id)?;

        let first_block_no = block_numbers.first().expect("could not get first block");
        let last_block_no = block_numbers.last().expect("could not get last block");

        let status = if (last_block_settled >= *first_block_no) && (last_block_settled <= *last_block_no) {
            SettlementVerificationStatus::Pending
        } else if last_block_settled > *last_block_no {
            SettlementVerificationStatus::Verified
        } else {
            SettlementVerificationStatus::Rejected
        };
        Ok(status)
    }

    async fn get_last_settled_block(&self) -> Result<u64> {
        let block_number = self.core_contract_client.state_block_number().await?;
        // TODO: unsafe unwrap
        Ok(block_number.try_into().unwrap())
    }
}

impl From<EthereumSettlementConfig> for EthereumSettlementClient {
    fn from(config: EthereumSettlementConfig) -> Self {
        // TODO: Very insecure way of building the signer. Needs to be adjusted.
        let private_key = get_env_var_or_panic(ENV_PRIVATE_KEY);
        let signer: PrivateKeySigner = private_key.parse().expect("Failed to parse private key");
        let wallet = EthereumWallet::from(signer);

        let provider = ProviderBuilder::new().with_recommended_fillers().wallet(wallet).on_http(config.rpc_url);
        let core_contract_client = StarknetValidityContractClient::new(
            Address::from_slice(config.core_contract_address.as_bytes()).0.into(),
            Arc::new(provider),
        );

        EthereumSettlementClient { core_contract_client }
    }
}
