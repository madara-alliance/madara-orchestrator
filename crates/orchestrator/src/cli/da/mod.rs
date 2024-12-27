use ethereum_da_client::EthereumDaValidatedArgs;
use starknet_da_client::StarknetDaValidatedArgs;

pub mod ethereum;
pub mod starknet;

#[derive(Debug, Clone)]
pub enum DaValidatedArgs {
    Ethereum(EthereumDaValidatedArgs),
    Starknet(StarknetDaValidatedArgs),
}
