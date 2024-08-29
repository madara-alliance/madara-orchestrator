use crate::get_env_var_or_panic;
use alloy::network::EthereumWallet;
use alloy::node_bindings::AnvilInstance;
use alloy::primitives::Address;
use alloy::providers::ext::AnvilApi;
use alloy::providers::ProviderBuilder;
use alloy::signers::local::LocalSigner;
use std::str::FromStr;

#[allow(dead_code)]
const BLOCK_TO_FORK: u64 = 20607627;

pub struct EthereumClient {
    anvil_endpoint: String,
    pub anvil_instance: Option<AnvilInstance>,
}

impl EthereumClient {
    /// Run : anvil --fork-url https://mainnet.infura.io/v3/bf9e41563a6a45e28eb60382d85ef3c9@20607627
    pub fn new() -> Self {
        // let eth_mainnet_rpc_url = get_env_var_or_panic("ETHEREUM_MAINNET_RPC_URL");
        //
        // let forked_anvil = Anvil::new()
        //     .fork(eth_mainnet_rpc_url)
        //     .fork_block_number(BLOCK_TO_FORK)
        //     .try_spawn()
        //     .expect("Unable to fork eth mainnet and run anvil.");

        println!("♢ Ethereum Client setup completed.");

        Self { anvil_endpoint: "http://localhost:8545".parse().unwrap(), anvil_instance: None }
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
