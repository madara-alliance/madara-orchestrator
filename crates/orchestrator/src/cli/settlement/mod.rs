use ethereum_settlement_client::EthereumSettlementParams;
use starknet_settlement_client::StarknetSettlementParams;

pub mod ethereum;
pub mod starknet;

#[derive(Clone, Debug)]
pub enum SettlementParams {
    Ethereum(EthereumSettlementParams),
    Starknet(StarknetSettlementParams),
}
