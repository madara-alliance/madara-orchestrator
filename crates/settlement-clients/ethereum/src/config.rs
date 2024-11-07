
use url::Url;

#[derive(Clone, Debug)]
pub struct EthereumSettlementParams {
    pub ethereum_rpc_url: Url,

    pub ethereum_private_key: String,

    pub l1_core_contract_address: String,

    pub starknet_operator_address: String,
}
