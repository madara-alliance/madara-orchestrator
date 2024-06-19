use std::str::FromStr;
use aptos_sdk::crypto::_once_cell::sync::Lazy;
use url::Url;
use da_client_interface::DaConfig;
use utils::env_utils::get_env_var_or_panic;

#[derive(Clone, Debug)]
pub struct AptosDaConfig {
    pub node_url: String,
    pub private_key: String,
    pub account_address: String,
}

impl DaConfig for AptosDaConfig {
    fn new_from_env() -> Self {
        static NODE_URL: Lazy<Url> = Lazy::new(|| {
            Url::from_str(
                std::env::var("APTOS_NODE_URL")
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("https://fullnode.devnet.aptoslabs.com"),
            )
                .unwrap()
        });

        static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
            Url::from_str(
                std::env::var("APTOS_FAUCET_URL")
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("https://faucet.devnet.aptoslabs.com"),
            )
                .unwrap()
        });
        Self {
            node_url: get_env_var_or_panic("APTOS_NODE_URL"),
            private_key: get_env_var_or_panic("PRIVATE_KEY"),
            account_address: get_env_var_or_panic("ADDRESS")
        }
    }
}