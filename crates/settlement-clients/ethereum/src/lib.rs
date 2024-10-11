use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use alloy::consensus::{
    BlobTransactionSidecar, SignableTransaction, TxEip4844, TxEip4844Variant, TxEip4844WithSidecar, TxEnvelope,
};
use alloy::eips::eip2930::AccessList;
use alloy::eips::eip4844::BYTES_PER_BLOB;
use alloy::hex;
use alloy::network::EthereumWallet;
use alloy::primitives::{Address, B256, U256};
use alloy::providers::{PendingTransactionConfig, Provider, ProviderBuilder};
use alloy::rpc::types::{TransactionReceipt, TransactionRequest};
use alloy::signers::local::PrivateKeySigner;
use alloy_primitives::Bytes;
use async_trait::async_trait;
use c_kzg::{Blob, Bytes32, KzgCommitment, KzgProof, KzgSettings};
use color_eyre::eyre::{eyre, Ok};
use color_eyre::Result;
use conversion::{get_input_data_for_eip_4844, prepare_sidecar};
use settlement_client_interface::{SettlementClient, SettlementVerificationStatus};
#[cfg(feature = "testing")]
use url::Url;
use utils::env_utils::get_env_var_or_panic;

use crate::clients::interfaces::validity_interface::StarknetValidityContractTrait;
use crate::clients::StarknetValidityContractClient;
use crate::config::EthereumSettlementConfig;
use crate::conversion::{slice_u8_to_u256, vec_u8_32_to_vec_u256};
pub mod clients;
pub mod config;
pub mod conversion;
pub mod tests;
pub mod types;
use alloy::providers::RootProvider;
use alloy::transports::http::Http;
use lazy_static::lazy_static;
use mockall::automock;
use reqwest::Client;
use utils::settings::Settings;

pub const ENV_PRIVATE_KEY: &str = "ETHEREUM_PRIVATE_KEY";

lazy_static! {
    pub static ref PROJECT_ROOT: PathBuf = PathBuf::from(format!("{}/../../../", env!("CARGO_MANIFEST_DIR")));
    pub static ref KZG_SETTINGS: KzgSettings = KzgSettings::load_trusted_setup_file(
        &PROJECT_ROOT.join("crates/settlement-clients/ethereum/src/trusted_setup.txt")
    )
    .expect("Error loading trusted setup file");
}

#[allow(dead_code)]
pub struct EthereumSettlementClient {
    core_contract_client: StarknetValidityContractClient,
    wallet: EthereumWallet,
    wallet_address: Address,
    provider: Arc<RootProvider<Http<Client>>>,
    impersonate_account: Option<Address>,
}

impl EthereumSettlementClient {
    pub fn new_with_settings(settings: &impl Settings) -> Self {
        let settlement_cfg = EthereumSettlementConfig::new_with_settings(settings);
        let private_key = get_env_var_or_panic(ENV_PRIVATE_KEY);
        let signer: PrivateKeySigner = private_key.parse().expect("Failed to parse private key");
        let wallet_address = signer.address();
        let wallet = EthereumWallet::from(signer);

        // provider without wallet
        let provider = Arc::new(ProviderBuilder::new().on_http(settlement_cfg.rpc_url.clone()));

        // provider with wallet
        let filler_provider = Arc::new(
            ProviderBuilder::new().with_recommended_fillers().wallet(wallet.clone()).on_http(settlement_cfg.rpc_url),
        );

        let core_contract_client = StarknetValidityContractClient::new(
            Address::from_str(&settlement_cfg.core_contract_address)
                .expect("Failed to convert the validity contract address.")
                .0
                .into(),
            filler_provider,
        );

        EthereumSettlementClient { provider, core_contract_client, wallet, wallet_address, impersonate_account: None }
    }

    #[cfg(feature = "testing")]
    pub fn with_test_settings(
        provider: RootProvider<Http<Client>>,
        core_contract_address: Address,
        rpc_url: Url,
        impersonate_account: Option<Address>,
    ) -> Self {
        let private_key = get_env_var_or_panic(ENV_PRIVATE_KEY);
        let signer: PrivateKeySigner = private_key.parse().expect("Failed to parse private key");
        let wallet_address = signer.address();
        let wallet = EthereumWallet::from(signer);

        let fill_provider =
            Arc::new(ProviderBuilder::new().with_recommended_fillers().wallet(wallet.clone()).on_http(rpc_url));

        let core_contract_client = StarknetValidityContractClient::new(core_contract_address, fill_provider);

        EthereumSettlementClient {
            provider: Arc::new(provider),
            core_contract_client,
            wallet,
            wallet_address,
            impersonate_account,
        }
    }

    /// Build kzg proof for the x_0 point evaluation
    pub fn build_proof(blob_data: Vec<Vec<u8>>, x_0_value: Bytes32) -> Result<KzgProof> {
        // Assuming that there is only one blob in the whole Vec<Vec<u8>> array for now.
        // Later we will add the support for multiple blob in single blob_data vec.
        assert_eq!(blob_data.len(), 1);

        let fixed_size_blob: [u8; BYTES_PER_BLOB] = blob_data[0].as_slice().try_into()?;

        let blob = Blob::new(fixed_size_blob);
        let commitment = KzgCommitment::blob_to_kzg_commitment(&blob, &KZG_SETTINGS)?;
        let (kzg_proof, y_0_value) = KzgProof::compute_kzg_proof(&blob, &x_0_value, &KZG_SETTINGS)?;

        // Verifying the proof for double check
        let eval = KzgProof::verify_kzg_proof(
            &commitment.to_bytes(),
            &x_0_value,
            &y_0_value,
            &kzg_proof.to_bytes(),
            &KZG_SETTINGS,
        )?;

        if !eval { Err(eyre!("ERROR : Assertion failed, not able to verify the proof.")) } else { Ok(kzg_proof) }
    }
}

#[automock]
#[async_trait]
impl SettlementClient for EthereumSettlementClient {
    /// Should register the proof on the base layer and return an external id
    /// which can be used to track the status.
    #[allow(unused)]
    async fn register_proof(&self, proof: [u8; 32]) -> Result<String> {
        todo!("register_proof is not implemented yet")
    }

    /// Should be used to update state on core contract when DA is done in calldata
    async fn update_state_calldata(
        &self,
        program_output: Vec<[u8; 32]>,
        onchain_data_hash: [u8; 32],
        onchain_data_size: [u8; 32],
    ) -> Result<String> {
        let program_output: Vec<U256> = vec_u8_32_to_vec_u256(program_output.as_slice())?;
        let onchain_data_hash: U256 = slice_u8_to_u256(&onchain_data_hash)?;
        let onchain_data_size = U256::from_be_bytes(onchain_data_size);
        let tx_receipt =
            self.core_contract_client.update_state(program_output, onchain_data_hash, onchain_data_size).await?;
        Ok(format!("0x{:x}", tx_receipt.transaction_hash))
    }

    /// Should be used to update state on core contract when DA is in blobs/alt DA
    async fn update_state_with_blobs(
        &self,
        program_output: Vec<[u8; 32]>,
        state_diff: Vec<Vec<u8>>,
        nonce: u64,
    ) -> Result<String> {
        let (sidecar_blobs, sidecar_commitments, sidecar_proofs) = prepare_sidecar(&state_diff, &KZG_SETTINGS).await?;
        let sidecar = BlobTransactionSidecar::new(sidecar_blobs, sidecar_commitments, sidecar_proofs);

        println!(">>>> sidecar done");
        
        let eip1559_est = self.provider.estimate_eip1559_fees(None).await?;
        let chain_id: u64 = self.provider.get_chain_id().await?.to_string().parse()?;

        println!(">>>> chain_id: {:?}", chain_id);
        
        let mut max_fee_per_blob_gas: u128 = self.provider.get_blob_base_fee().await?.to_string().parse()?;
        // TODO: need to send more than current gas price.
        max_fee_per_blob_gas += 12;
        let max_priority_fee_per_gas: u128 = self.provider.get_max_priority_fee_per_gas().await?.to_string().parse()?;

        println!(">>>> max_priority_fee_per_gas: {:?}", max_priority_fee_per_gas);
        
        // x_0_value : program_output[10]
        // Updated with starknet 0.13.2 spec
        let kzg_proof = Self::build_proof(
            state_diff,
            Bytes32::from_bytes(program_output[10].as_slice()).expect("Not able to get x_0 point params."),
        )
        .expect("Unable to build KZG proof for given params.")
        .to_owned();

        println!(">>>> kzg_proof: {:?}", kzg_proof);

        let input_bytes = get_input_data_for_eip_4844(program_output, kzg_proof)?;

        println!(">>>> input_bytes: {:?}", input_bytes);

        let tx: TxEip4844 = TxEip4844 {
            chain_id,
            nonce,
            gas_limit: 30_000_000,
            max_fee_per_gas: eip1559_est.max_fee_per_gas.to_string().parse()?,
            max_priority_fee_per_gas,
            to: self.core_contract_client.contract_address(),
            value: U256::from(0),
            access_list: AccessList(vec![]),
            blob_versioned_hashes: sidecar.versioned_hashes().collect(),
            max_fee_per_blob_gas,
            input: Bytes::from(hex::decode(input_bytes)?),
        };

        let tx_sidecar = TxEip4844WithSidecar { tx: tx.clone(), sidecar: sidecar.clone() };

        println!(">>>> tx sidecar built");

        let mut variant = TxEip4844Variant::from(tx_sidecar);
        let signature = self.wallet.default_signer().sign_transaction(&mut variant).await?;
        let tx_signed = variant.into_signed(signature);
        println!(">>>> tx signed");
        let tx_envelope: TxEnvelope = tx_signed.into();
        println!(">>>> tx envelope");

        #[cfg(not(feature = "testing"))]
        let txn_request = {
            let txn_request: TransactionRequest = tx_envelope.clone().into();
            txn_request
        };

        #[cfg(feature = "testing")]
        let txn_request =
            { test_config::configure_transaction(self.provider.clone(), tx_envelope, self.impersonate_account).await };

        println!(">>>> tx request : {:?}", txn_request);

        let pending_transaction = self.provider.send_transaction(txn_request).await?;
        return Ok(pending_transaction.tx_hash().to_string());
    }

    /// Should verify the inclusion of a tx in the settlement layer
    async fn verify_tx_inclusion(&self, tx_hash: &str) -> Result<SettlementVerificationStatus> {
        let tx_hash = B256::from_str(tx_hash)?;
        let maybe_tx_status: Option<TransactionReceipt> = self.provider.get_transaction_receipt(tx_hash).await?;
        match maybe_tx_status {
            Some(tx_status) => {
                if tx_status.status() {
                    Ok(SettlementVerificationStatus::Verified)
                } else {
                    Ok(SettlementVerificationStatus::Pending)
                }
            }
            None => Ok(SettlementVerificationStatus::Rejected(format!("Could not find status of tx: {}", tx_hash))),
        }
    }

    /// Wait for a pending tx to achieve finality
    async fn wait_for_tx_finality(&self, tx_hash: &str) -> Result<()> {
        let tx_hash = B256::from_str(tx_hash)?;
        self.provider.watch_pending_transaction(PendingTransactionConfig::new(tx_hash)).await?;
        Ok(())
    }

    /// Get the last block settled through the core contract
    async fn get_last_settled_block(&self) -> Result<u64> {
        let block_number = self.core_contract_client.state_block_number().await?;
        Ok(block_number.try_into()?)
    }

    async fn get_nonce(&self) -> Result<u64> {
        let nonce = self.provider.get_transaction_count(self.wallet_address).await?.to_string().parse()?;
        Ok(nonce)
    }
}

#[cfg(feature = "testing")]
mod test_config {
    use alloy::network::TransactionBuilder;

    use super::*;

    pub async fn configure_transaction(
        provider: Arc<RootProvider<Http<Client>>>,
        tx_envelope: TxEnvelope,
        impersonate_account: Option<Address>,
    ) -> TransactionRequest {
        let mut txn_request: TransactionRequest = tx_envelope.into();

        // IMPORTANT to understand #[cfg(test)], #[cfg(not(test))] and SHOULD_IMPERSONATE_ACCOUNT
        // Two tests :  `update_state_blob_with_dummy_contract_works` &
        // `update_state_blob_with_impersonation_works` use a env var `SHOULD_IMPERSONATE_ACCOUNT` to inform
        // the function `update_state_with_blobs` about the kind of testing,
        // `SHOULD_IMPERSONATE_ACCOUNT` can have any of "0" or "1" value :
        //      - if "0" then : Testing via default Anvil address.
        //      - if "1" then : Testing via impersonating `Starknet Operator Address`.
        // Note : changing between "0" and "1" is handled automatically by each test function, `no` manual
        // change in `env.test` is needed.
        if let Some(impersonate_account) = impersonate_account {
            let nonce =
                provider.get_transaction_count(impersonate_account).await.unwrap().to_string().parse::<u64>().unwrap();
            txn_request.set_nonce(nonce);
            txn_request = txn_request.with_from(impersonate_account);
        }

        txn_request
    }
}
