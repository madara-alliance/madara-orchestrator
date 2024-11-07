use ethereum_settlement_client::config::EthereumSettlementParams;
use starknet_settlement_client::config::StarknetSettlementParams;

pub mod ethereum;
pub mod starknet;

#[derive(Clone, Debug)]
pub enum SettlementParams {
    Ethereum(EthereumSettlementParams),
    Starknet(StarknetSettlementParams),
}
