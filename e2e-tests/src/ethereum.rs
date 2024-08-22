use alloy::node_bindings::Anvil;

const BLOCK_TO_FORK: u64 = 18169622;
const ETH_MAINNET_URL: &str = "";

pub struct EthereumClient {
    anvil_endpoint: String,
}

impl EthereumClient {
    /// To create a new Ethereum Client (spawns a new anvil instance)
    pub fn new() -> Self {
        let forked_anvil = Anvil::new()
            .fork(ETH_MAINNET_URL)
            .fork_block_number(BLOCK_TO_FORK)
            .try_spawn()
            .expect("Unable to fork eth mainnet and run anvil.");

        Self { anvil_endpoint: forked_anvil.endpoint() }
    }

    /// To get the anvil endpoint
    pub fn endpoint(&self) -> String {
        self.anvil_endpoint.clone()
    }
}

impl Default for EthereumClient {
    fn default() -> Self {
        Self::new()
    }
}
