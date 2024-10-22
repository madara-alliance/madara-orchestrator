use alloy::network::{Ethereum, EthereumWallet};
use alloy::providers::fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller};
use alloy::providers::{Identity, RootProvider};
use alloy::transports::http::{Client, Http};
use alloy_primitives::U256;

pub type LocalWalletSignerMiddleware = FillProvider<
    JoinFill<
        JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<Http<Client>>,
    Http<Client>,
    Ethereum,
>;

pub type EthHttpProvider = FillProvider<
    JoinFill<
        JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<Http<Client>>,
    Http<Client>,
    Ethereum,
>;

pub fn convert_stark_bigint_to_u256(y_low: u128, y_high: u128) -> U256 {
    let y_high_u256 = U256::from(y_high);
    let y_low_u256 = U256::from(y_low);
    let shifted = y_high_u256 << 128;
    shifted + y_low_u256
}

pub fn bytes_to_u128(bytes: &[u8; 32]) -> u128 {
    let mut result: u128 = 0;

    // Since u128 is 16 bytes, we'll use the last 16 bytes of the input array
    // Starting from index 16 to get the least significant bytes
    for &byte in bytes[16..32].iter() {
        result = (result << 8) | byte as u128;
    }

    result
}
