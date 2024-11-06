pub mod ethereum;
pub mod starknet;


#[derive(Clone, Debug)]
pub enum SettlementParams {
    Ethereum(ethereum::EthereumSettlementParams),
    Starknet(starknet::StarknetSettlementParams),
}