use settlement_client_interface::SettlementConfig;

pub struct StarknetSettlementConfig {}

impl SettlementConfig for StarknetSettlementConfig {
    /// Should create a new instance of the DaConfig from the environment variables
    fn new_from_env() -> Self {
        StarknetSettlementConfig {}
    }
}
