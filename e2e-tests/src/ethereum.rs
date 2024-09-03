use alloy::network::EthereumWallet;
use alloy::node_bindings::{Anvil, AnvilInstance};
use alloy::primitives::Address;
use alloy::providers::ext::AnvilApi;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::LocalSigner;
use std::str::FromStr;
use utils::env_utils::get_env_var_or_panic;

// L1 block to fork
const L1_BLOCK_TO_FORK: u64 = 20607627;

pub struct EthereumClient {
    anvil_endpoint: String,
    /// We are defining the anvil instance here, but it may not be used
    /// anywhere. We are keeping it to keep the things in scope.
    _anvil_instance: Option<AnvilInstance>,
}

impl EthereumClient {
    pub fn new() -> Self {
        let eth_mainnet_rpc_url = get_env_var_or_panic("SETTLEMENT_RPC_URL");

        let forked_anvil = Anvil::new()
            .fork(eth_mainnet_rpc_url)
            .fork_block_number(L1_BLOCK_TO_FORK)
            .try_spawn()
            .expect("Unable to fork eth mainnet and run anvil.");

        println!("✅ Ethereum Client setup completed.");

        Self { anvil_endpoint: forked_anvil.endpoint(), _anvil_instance: Some(forked_anvil) }
    }

    /// Impersonate Account on anvil as starknet operator
    pub async fn impersonate_account_as_address(&self, address: Address) {
        let provider = ProviderBuilder::new().on_http(self.anvil_endpoint.parse().unwrap());

        // Impersonate account as starknet operator
        provider.anvil_impersonate_account(address).await.expect("Unable to impersonate account.");

        println!("✅ Impersonate Account setup completed.");
    }

    /// To get the anvil endpoint
    pub fn endpoint(&self) -> String {
        self.anvil_endpoint.clone()
    }

    /// To get the signer (this will make the signer from the given private key in env vars)
    pub fn get_signer(&self) -> EthereumWallet {
        let signer = LocalSigner::from_str(get_env_var_or_panic("ETHEREUM_PRIVATE_KEY").as_str()).unwrap();
        EthereumWallet::from(signer)
    }
}

impl Default for EthereumClient {
    fn default() -> Self {
        Self::new()
    }
}
