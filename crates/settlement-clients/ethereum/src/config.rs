use settlement_client_interface::SettlementConfig;

pub struct EthereumSettlementConfig {}

impl SettlementConfig for EthereumSettlementConfig {
    /// Should create a new instance of the DaConfig from the environment variables
    fn new_from_env() -> Self {
        EthereumSettlementConfig {}
    }
}
