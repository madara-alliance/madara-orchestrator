use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use url::Url;
use utils::settings::Settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementLayer {
    Ethereum,
    Starknet,
}

/// SHARP proving service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlanticConfig {
    /// SHARP service url
    pub service_url: Url,
    /// EVM RPC node url
    pub rpc_node_url: Url,
    /// GPS verifier contract address (implements FactRegistry)
    pub verifier_address: Address,
    pub settlement_layer: SettlementLayer,
}

impl AtlanticConfig {
    pub fn new_with_settings(settings: &impl Settings) -> color_eyre::Result<Self> {
        let settlement_layer = settings.get_settings_or_panic("SETTLEMENT_LAYER");
        match settlement_layer.as_str() {
            "ethereum" => Ok(Self {
                service_url: settings.get_settings_or_panic("ATLANTIC_URL").parse().unwrap(),
                rpc_node_url: settings.get_settings_or_panic("SETTLEMENT_RPC_URL").parse().unwrap(),
                verifier_address: settings.get_settings_or_panic("GPS_VERIFIER_CONTRACT_ADDRESS").parse().unwrap(),
                settlement_layer: SettlementLayer::Ethereum,
            }),
            "starknet" => Ok(Self {
                service_url: settings.get_settings_or_panic("ATLANTIC_URL").parse().unwrap(),
                rpc_node_url: settings.get_settings_or_panic("SETTLEMENT_RPC_URL").parse().unwrap(),
                verifier_address: settings.get_settings_or_panic("GPS_VERIFIER_CONTRACT_ADDRESS").parse().unwrap(),
                settlement_layer: SettlementLayer::Starknet,
            }),
            _ => panic!("Unsupported Settlement layer"),
        }
    }
}
