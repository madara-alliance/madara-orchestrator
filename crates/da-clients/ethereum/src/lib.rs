#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]

use crate::config::EthereumDaConfig;
use alloy::network::Ethereum;
use alloy::providers::{ProviderBuilder, RootProvider};
use alloy::rpc::client::RpcClient;
use alloy::transports::http::Http;
use async_trait::async_trait;
use color_eyre::Result;
use da_client_interface::{DaClient, DaVerificationStatus};
use mockall::automock;
use mockall::predicate::*;
use reqwest::Client;
use std::str::FromStr;
use url::Url;
use utils::settings::SettingsProvider;

pub const DA_SETTINGS_NAME: &str = "ethereum";

pub mod config;
pub struct EthereumDaClient {
    #[allow(dead_code)]
    provider: RootProvider<Ethereum, Http<Client>>,
}

#[automock]
#[async_trait]
impl DaClient for EthereumDaClient {
    async fn publish_state_diff(&self, _state_diff: Vec<Vec<u8>>, _to: &[u8; 32]) -> Result<String> {
        // Here in case of ethereum we are not publishing the state diff because we are doing it all together in update_state job.
        // So we don't need to send the blob here.
        Ok("NA".to_string())
    }

    async fn verify_inclusion(&self, _external_id: &str) -> Result<DaVerificationStatus> {
        Ok(DaVerificationStatus::Verified)
    }

    async fn max_blob_per_txn(&self) -> u64 {
        6
    }

    async fn max_bytes_per_blob(&self) -> u64 {
        131072
    }
}

impl EthereumDaClient {
    pub fn with_settings(settings: &impl SettingsProvider) -> Self {
        let config: EthereumDaConfig = settings.get_settings(DA_SETTINGS_NAME).unwrap();
        let client =
            RpcClient::new_http(Url::from_str(config.rpc_url.as_str()).expect("Failed to parse SETTLEMENT_RPC_URL"));
        let provider = ProviderBuilder::<_, Ethereum>::new().on_client(client);
        EthereumDaClient { provider }
    }
}
