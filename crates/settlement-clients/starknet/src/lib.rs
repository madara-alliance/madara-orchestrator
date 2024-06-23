pub mod config;
pub mod conversion;

use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use lazy_static::lazy_static;
use mockall::{automock, predicate::*};
use starknet::accounts::ConnectedAccount;
use starknet::providers::Provider;
use starknet::{
    accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount},
    core::{
        chain_id,
        types::{BlockId, BlockTag, FieldElement, FunctionCall},
        utils::get_selector_from_name,
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};

use config::StarknetSettlementConfig;
use conversion::{slice_slice_u8_to_vec_field, slice_u8_to_field};
use settlement_client_interface::{parse_and_validate_block_order, SettlementClient, SettlementVerificationStatus};
use utils::env_utils::get_env_var_or_panic;

#[allow(unused)]
pub struct StarknetSettlementClient {
    pub account: SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>,
    pub core_contract_address: FieldElement,
}

pub const ENV_PUBLIC_KEY: &str = "STARKNET_PUBLIC_KEY";
pub const ENV_PRIVATE_KEY: &str = "STARKNET_PRIVATE_KEY";

// Assumed the contract called for settlement l ooks like:
// https://github.com/keep-starknet-strange/piltover

lazy_static! {
    pub static ref CONTRACT_WRITE_UPDATE_STATE_SELECTOR: FieldElement =
        get_selector_from_name("update_state").expect("Invalid update state selector");
    pub static ref CONTRACT_READ_STATE_BLOCK_NUMBER: FieldElement =
        get_selector_from_name("stateBlockNumber").expect("Invalid update state selector");
}

#[automock]
#[async_trait]
impl SettlementClient for StarknetSettlementClient {
    /// Should register the proof on the base layer and return an external id
    /// which can be used to track the status.
    #[allow(unused)]
    async fn register_proof(&self, proof: Vec<u8>) -> Result<String> {
        !unimplemented!("register_proof not implemented yet")
    }

    /// Should be used to update state on core contract when DA is done in calldata
    async fn update_state_calldata(
        &self,
        program_output: Vec<Vec<u8>>,
        onchain_data_hash: Vec<u8>,
        onchain_data_size: usize,
    ) -> Result<String> {
        let program_output = slice_slice_u8_to_vec_field(&program_output);
        let onchain_data_hash = slice_u8_to_field(&onchain_data_hash);
        let mut calldata: Vec<FieldElement> = Vec::with_capacity(program_output.len() + 2);
        calldata.extend(program_output);
        calldata.push(onchain_data_hash);
        calldata.push(FieldElement::from(onchain_data_size));
        let _ = self
            .account
            .execute(vec![Call {
                to: self.core_contract_address,
                selector: *CONTRACT_WRITE_UPDATE_STATE_SELECTOR,
                calldata,
            }])
            .send()
            .await?;
        // TODO(akhercha): external id ?
        Ok("external_id".to_string())
    }

    /// Should be used to update state on core contract when DA is in blobs/alt DA
    #[allow(unused)]
    async fn update_state_blobs(&self, program_output: Vec<Vec<u8>>, kzg_proof: Vec<u8>) -> Result<String> {
        !unimplemented!("not available for starknet settlement layer")
    }

    /// Should verify the inclusion of the state diff in the DA layer and return the status
    #[allow(unused)]
    async fn verify_inclusion(&self, external_id: &str) -> Result<SettlementVerificationStatus> {
        let last_block_settled = self.get_last_settled_block().await?;
        // TODO: We assumed here that external_id is the list of blocks comma separated
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

    /// Returns the last block settled from the core contract.
    async fn get_last_settled_block(&self) -> Result<u64> {
        let block_number = self
            .account
            .provider()
            .call(
                FunctionCall {
                    contract_address: self.core_contract_address,
                    entry_point_selector: *CONTRACT_READ_STATE_BLOCK_NUMBER,
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;
        if block_number.is_empty() {
            return Err(eyre!("Could not fetch last block number from core contract."));
        }
        // TODO(akhercha): unsafe unwrap from conversion
        Ok(block_number[0].try_into().unwrap())
    }
}

impl From<StarknetSettlementConfig> for StarknetSettlementClient {
    fn from(config: StarknetSettlementConfig) -> Self {
        let provider = Arc::new(JsonRpcClient::new(HttpTransport::new(config.rpc_url)));

        let public_key = get_env_var_or_panic(ENV_PUBLIC_KEY);
        let signer_address = FieldElement::from_hex_be(&public_key).expect("invalid signer address");

        // TODO: Very insecure way of building the signer. Needs to be adjusted.
        let private_key = get_env_var_or_panic(ENV_PRIVATE_KEY);
        let signer = FieldElement::from_hex_be(&private_key).expect("Invalid private key");
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(signer));

        let core_contract_address =
            FieldElement::from_hex_be(&config.core_contract_address).expect("Invalid core contract address");

        let account = SingleOwnerAccount::new(
            provider.clone(),
            signer,
            signer_address,
            // TODO: chain_id should be configurable?
            chain_id::MAINNET,
            ExecutionEncoding::Legacy,
        );

        StarknetSettlementClient { account, core_contract_address }
    }
}
