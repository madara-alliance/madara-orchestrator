use ethereum_da_client::EthereumDaParams;

pub mod ethereum;

#[derive(Debug, Clone)]
pub enum DaParams {
    Ethereum(EthereumDaParams),
}
