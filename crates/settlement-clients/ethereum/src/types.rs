use alloy::{
    network::{Ethereum, EthereumWallet},
    providers::{
        fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller},
        RootProvider,
    },
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
