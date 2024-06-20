use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::Address,
    providers::{
        fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller},
        RootProvider,
    },
    signers::local::PrivateKeySigner,
    transports::http::{Client, Http},
};

pub type LocalWalletSignerMiddleware = FillProvider<
    JoinFill<
        JoinFill<JoinFill<JoinFill<alloy::providers::Identity, GasFiller>, NonceFiller>, ChainIdFiller>,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<Http<Client>>,
    Http<Client>,
    Ethereum,
>;
