use clap::Args;


// SETTLEMENT_RPC_URL="https://eth-sepolia.public.blastapi.io"
// ETHEREUM_PRIVATE_KEY="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
// L1_CORE_CONTRACT_ADDRESS="0xE2Bb56ee936fd6433DC0F6e7e3b8365C906AA057"
// STARKNET_OPERATOR_ADDRESS="0x5b98B836969A60FEC50Fa925905Dd1D382a7db43"


#[derive(Debug, Clone, Args)]
pub struct EthereumSettlementParams {
    /// The URL of the Ethereum RPC node.
    #[arg(env = "ETHEREUM_SETTLEMENT_RPC_URL", long)]
    pub ethereum_rpc_url: String,

    /// The private key of the Ethereum account.
    #[arg(env = "ETHEREUM_PRIVATE_KEY", long)]
    pub ethereum_private_key: String,

    /// The address of the L1 core contract.
    #[arg(env = "L1_CORE_CONTRACT_ADDRESS", long)]
    pub l1_core_contract_address: String,

    /// The address of the Starknet operator.
    #[arg(env = "STARKNET_OPERATOR_ADDRESS", long)]
    pub starknet_operator_address: String,
}
