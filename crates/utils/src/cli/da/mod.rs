pub mod ethereum;

#[derive(Debug, Clone)]
pub enum DaParams {
    Ethereum(ethereum::EthereumDAParams),
}
