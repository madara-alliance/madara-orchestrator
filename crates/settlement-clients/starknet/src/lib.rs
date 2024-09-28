pub mod config;
pub mod conversion;
#[cfg(test)]
pub mod tests;

use std::sync::Arc;

use appchain_core_contract_client::interfaces::core_contract::CoreContract;
use async_trait::async_trait;
use color_eyre::eyre::{eyre, Ok};
use color_eyre::Result;
use lazy_static::lazy_static;
use mockall::{automock, predicate::*};
use starknet::accounts::ConnectedAccount;
use starknet::core::types::{TransactionExecutionStatus, U256};
use starknet::providers::Provider;
use starknet::{
    accounts::{ExecutionEncoding, SingleOwnerAccount},
    core::{
        types::{BlockId, BlockTag, Felt, FunctionCall},
        utils::get_selector_from_name,
    },
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
    signers::{LocalWallet, SigningKey},
};
use tokio::time::{sleep, Duration};

use appchain_core_contract_client::clients::StarknetCoreContractClient;
use settlement_client_interface::{SettlementClient, SettlementVerificationStatus};
use utils::settings::Settings;

use crate::config::StarknetSettlementConfig;
use crate::conversion::{slice_slice_u8_to_vec_field, slice_u8_to_field};

pub struct StarknetSettlementClient {
    pub account: Arc<SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>>,
    pub starknet_core_contract_client: StarknetCoreContractClient,
    pub core_contract_address: Felt,
    pub tx_finality_retry_delay_in_seconds: u64,
}

pub const ENV_PUBLIC_KEY: &str = "STARKNET_PUBLIC_KEY";
pub const ENV_PRIVATE_KEY: &str = "STARKNET_PRIVATE_KEY";

const MAX_RETRIES_VERIFY_TX_FINALITY: usize = 10;

// Assumed the contract called for settlement l ooks like:
// https://github.com/keep-starknet-strange/piltover

impl StarknetSettlementClient {
    pub async fn new_with_settings(settings: &impl Settings) -> Self {
        let settlement_cfg = StarknetSettlementConfig::new_with_settings(settings);
        let provider: Arc<JsonRpcClient<HttpTransport>> =
            Arc::new(JsonRpcClient::new(HttpTransport::new(settlement_cfg.rpc_url.clone())));

        let public_key = settings.get_settings_or_panic(ENV_PUBLIC_KEY);
        let signer_address = Felt::from_hex(&public_key).expect("invalid signer address");

        // TODO: Very insecure way of building the signer. Needs to be adjusted.
        let private_key = settings.get_settings_or_panic(ENV_PRIVATE_KEY);
        let signer = Felt::from_hex(&private_key).expect("Invalid private key");
        let signer = LocalWallet::from(SigningKey::from_secret_scalar(signer));

        let core_contract_address =
            Felt::from_hex(&settlement_cfg.core_contract_address).expect("Invalid core contract address");

        let account: Arc<SingleOwnerAccount<Arc<JsonRpcClient<HttpTransport>>, LocalWallet>> =
            Arc::new(SingleOwnerAccount::new(
                provider.clone(),
                signer.clone(),
                signer_address,
                provider.chain_id().await.unwrap(),
                ExecutionEncoding::New,
            ));

        let starknet_core_contract_client: StarknetCoreContractClient =
            StarknetCoreContractClient::new(core_contract_address, account.clone());

        StarknetSettlementClient {
            account,
            core_contract_address,
            starknet_core_contract_client,
            tx_finality_retry_delay_in_seconds: settlement_cfg.tx_finality_retry_delay_in_seconds,
        }
    }
}

lazy_static! {
    pub static ref CONTRACT_WRITE_UPDATE_STATE_SELECTOR: Felt =
        get_selector_from_name("update_state").expect("Invalid update state selector");
    // TODO: `stateBlockNumber` does not exists yet in our implementation:
    // https://github.com/keep-starknet-strange/piltover
    // It should get added to match the solidity implementation of the core contract.
    pub static ref CONTRACT_READ_STATE_BLOCK_NUMBER: Felt =
        get_selector_from_name("stateBlockNumber").expect("Invalid update state selector");
}

// TODO: Note that we already have an implementation of the appchain core contract client available here:
// https://github.com/keep-starknet-strange/zaun/tree/main/crates/l3/appchain-core-contract-client
// However, this implementation uses different Felt types, and incorporating all of them
// into this repository would introduce unnecessary complexity.
// Therefore, we will wait for the update of starknet_rs in the Zaun repository before adapting
// the StarknetSettlementClient implementation.

#[automock]
#[async_trait]
impl SettlementClient for StarknetSettlementClient {
    /// Should register the proof on the base layer and return an external id
    /// which can be used to track the status.
    #[allow(unused)]
    async fn register_proof(&self, proof: [u8; 32]) -> Result<String> {
        !unimplemented!("register_proof not implemented yet")
    }

    /// Should be used to update state on core contract when DA is done in calldata
    async fn update_state_calldata(
        &self,
        program_output: Vec<[u8; 32]>,
        onchain_data_hash: [u8; 32],
        onchain_data_size: [u128; 2],
    ) -> Result<String> {
        let program_output = slice_slice_u8_to_vec_field(program_output.as_slice());
        let onchain_data_hash = slice_u8_to_field(&onchain_data_hash);
        let core_contract: &CoreContract = self.starknet_core_contract_client.as_ref();
        let onchain_data_size = U256::from_words(onchain_data_size[0], onchain_data_size[1]);
        let invoke_result = core_contract.update_state(program_output, onchain_data_hash, onchain_data_size).await?;

        Ok(invoke_result.transaction_hash.to_hex_string())
    }

    /// Should verify the inclusion of a tx in the settlement layer
    async fn verify_tx_inclusion(&self, tx_hash: &str) -> Result<SettlementVerificationStatus> {
        let tx_hash = Felt::from_hex(tx_hash)?;
        let tx_receipt = self.account.provider().get_transaction_receipt(tx_hash).await?;
        let execution_result = tx_receipt.receipt.execution_result();
        let status = execution_result.status();

        if tx_receipt.block.is_pending() {
            match status {
                TransactionExecutionStatus::Succeeded => Ok(SettlementVerificationStatus::Pending),
                TransactionExecutionStatus::Reverted => Ok(SettlementVerificationStatus::Rejected(format!(
                    "Pending tx {} has been reverted: {}",
                    tx_hash,
                    execution_result.revert_reason().unwrap()
                ))),
            }
        } else {
            match status {
                TransactionExecutionStatus::Succeeded => Ok(SettlementVerificationStatus::Verified),
                TransactionExecutionStatus::Reverted => Ok(SettlementVerificationStatus::Rejected(format!(
                    "Tx {} has been reverted: {}",
                    tx_hash,
                    execution_result.revert_reason().unwrap()
                ))),
            }
        }
    }

    /// Should be used to update state on core contract and publishing the blob simultaneously
    #[allow(unused)]
    async fn update_state_with_blobs(
        &self,
        program_output: Vec<[u8; 32]>,
        state_diff: Vec<Vec<u8>>,
        nonce: u64,
    ) -> Result<String> {
        !unimplemented!("not implemented yet.")
    }

    /// Wait for a pending tx to achieve finality
    async fn wait_for_tx_finality(&self, tx_hash: &str) -> Result<()> {
        let mut retries = 0;
        let duration_to_wait_between_polling = Duration::from_secs(self.tx_finality_retry_delay_in_seconds);
        sleep(duration_to_wait_between_polling).await;

        let tx_hash = Felt::from_hex(tx_hash)?;
        loop {
            let tx_receipt = self.account.provider().get_transaction_receipt(tx_hash).await?;
            if tx_receipt.block.is_pending() {
                retries += 1;
                if retries > MAX_RETRIES_VERIFY_TX_FINALITY {
                    return Err(eyre!("Max retries exceeeded while waiting for tx {tx_hash} finality."));
                }
                sleep(duration_to_wait_between_polling).await;
            } else {
                break;
            }
        }
        Ok(())
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

        Ok(u64::from_le_bytes(block_number[0].to_bytes_le()[0..8].try_into().unwrap()))
    }

    /// Returns the nonce for the wallet in use.
    async fn get_nonce(&self) -> Result<u64> {
        todo!("Yet to impl nonce call for Starknet.")
    }
}

#[cfg(test)]
mod test {

    use starknet::core::types::Felt;
    #[test]
    fn test_felt_conversion() {
        let number_in_felt = Felt::from_hex("0x8").unwrap();
        let number_final = u64::from_le_bytes(number_in_felt.to_bytes_le()[0..8].try_into().unwrap());
        println!("{number_in_felt} {number_final}");

        assert!(number_final == 8, "Should be 8");
    }
}
