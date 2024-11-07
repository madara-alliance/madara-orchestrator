use url::Url;
#[derive(Clone, Debug)]
pub struct StarknetSettlementParams {
    pub starknet_rpc_url: Url,

    pub starknet_private_key: String,

    pub starknet_account_address: String,

    pub starknet_cairo_core_contract_address: String,

    pub starknet_finality_retry_wait_in_secs: u64,

    pub madara_binary_path: String,
}
