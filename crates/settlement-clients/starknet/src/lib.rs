pub mod config;
pub mod conversion;

use std::sync::Arc;

use async_trait::async_trait;
use color_eyre::Result;
use config::StarknetSettlementConfig;
use conversion::{slice_slice_u8_to_vec_field, slice_u8_to_field};
use mockall::{automock, predicate::*};
use settlement_client_interface::{SettlementClient, SettlementVerificationStatus};
use starknet::{
    accounts::{Account, Call, ExecutionEncoding, SingleOwnerAccount},
    core::{chain_id, types::FieldElement, utils::get_selector_from_name},
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};
use utils::env_utils::get_env_var_or_panic;

#[allow(unused)]
pub struct StarknetSettlementClient {
    pub account: SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>,
    pub core_contract_address: String,
}

pub const ENV_PUBLIC_KEY: &str = "STARKNET_PUBLIC_KEY";
pub const ENV_PRIVATE_KEY: &str = "STARKNET_PRIVATE_KEY";

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
                to: FieldElement::from_hex_be(&self.core_contract_address).expect("invalid contract"),
                selector: get_selector_from_name("update_state").expect("invalid selector"),
                calldata,
            }])
            .send()
            .await?;
        // TODO: external id ?
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
        Ok(SettlementVerificationStatus::Verified)
    }
}

impl From<StarknetSettlementConfig> for StarknetSettlementClient {
    fn from(config: StarknetSettlementConfig) -> Self {
        let provider = Arc::new(JsonRpcClient::new(HttpTransport::new(config.rpc_url)));

        let public_key = get_env_var_or_panic(ENV_PUBLIC_KEY);
        let private_key = get_env_var_or_panic(ENV_PRIVATE_KEY);

        let signer_address = FieldElement::from_hex_be(&public_key).expect("invalid signer address");
        // TODO: Very insecure way of building the signer. Needs to be adjusted.
        let signer = FieldElement::from_hex_be(&private_key).expect("Invalid private key");
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(signer));

        let account = SingleOwnerAccount::new(
            provider.clone(),
            signer,
            signer_address,
            chain_id::MAINNET,
            ExecutionEncoding::Legacy,
        );

        StarknetSettlementClient { account, core_contract_address: config.core_contract_address }
    }
}
