use ethereum_da_client::EthereumDaValidatedArgs;

pub mod ethereum;

#[derive(Debug, Clone)]
pub enum DaParams {
    Ethereum(EthereumDaValidatedArgs),
}
