use crate::get_env_var_or_panic;
use alloy::network::EthereumWallet;
use alloy::node_bindings::{Anvil, AnvilInstance};
use alloy::primitives::Address;
use alloy::providers::ext::AnvilApi;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::LocalSigner;
use std::str::FromStr;

const BLOCK_TO_FORK: u64 = 20607627;

pub struct EthereumClient {
    anvil_endpoint: String,
    anvil_instance: AnvilInstance,
}

impl EthereumClient {
    /// To create a new Ethereum Client (spawns a new anvil instance)
    pub fn new() -> Self {
        let eth_mainnet_rpc_url = get_env_var_or_panic("ETHEREUM_MAINNET_RPC_URL");

        let forked_anvil = Anvil::new()
            .fork(eth_mainnet_rpc_url)
            .port(8545u16)
            .fork_block_number(BLOCK_TO_FORK)
            .try_spawn()
            .expect("Unable to fork eth mainnet and run anvil.");

        println!("♢ Ethereum Client setup completed.");

        Self { anvil_endpoint: forked_anvil.endpoint(), anvil_instance: forked_anvil }
    }

    /// Impersonate Account on anvil as starknet operator
    pub async fn impersonate_account_as_starknet_operator(&self) {
        let provider = ProviderBuilder::new().on_http(self.anvil_endpoint.parse().unwrap());

        // Impersonate account as starknet operator
        provider
            .anvil_impersonate_account(
                Address::from_str("0x2C169DFe5fBbA12957Bdd0Ba47d9CEDbFE260CA7").expect("Unable to parse address"),
            )
            .await
            .expect("Unable to impersonate account.");

        println!("♢ Impersonate Account setup completed.");
    }

    /// To get the anvil endpoint
    pub fn endpoint(&self) -> String {
        self.anvil_endpoint.clone()
    }

    /// To get anvil instance
    pub fn anvil_instance(&self) -> &AnvilInstance {
        &self.anvil_instance
    }

    /// To get the signer
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
