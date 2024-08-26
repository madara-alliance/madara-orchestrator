use crate::get_env_var_or_panic;
use alloy::network::EthereumWallet;
use alloy::node_bindings::{Anvil, AnvilInstance};
use alloy::signers::local::LocalSigner;
use std::str::FromStr;

// This is the transaction for updateState :
// https://etherscan.io/tx/0xacc442468b2297ea3fe7aee63f3dac0816625f3f0fd7ba074217316a25658355
const BLOCK_TO_FORK: u64 = 18169622;

pub struct EthereumClient {
    anvil_endpoint: String,
    anvil_instance: AnvilInstance,
}

impl EthereumClient {
    /// To create a new Ethereum Client (spawns a new anvil instance)
    pub fn new() -> Self {
        let eth_mainnet_rpc_url = get_env_var_or_panic("ETHEREUM_MAINNET_RPC_URL");

        let forked_anvil = Anvil::new()
            .fork(eth_mainnet_rpc_url).port(8545u16)
            .fork_block_number(BLOCK_TO_FORK)
            .try_spawn()
            .expect("Unable to fork eth mainnet and run anvil.");

        Self { anvil_endpoint: forked_anvil.endpoint(), anvil_instance: forked_anvil }
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
