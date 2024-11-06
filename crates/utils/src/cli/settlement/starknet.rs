use clap::Args;
use url::Url;


// STARKNET_RPC_URL="https://starknet-sepolia.public.blastapi.io"
// STARKNET_PRIVATE_KEY=0x76f2ccdb23f29bc7b69278e947c01c6160a31cf02c19d06d0f6e5ab1d768b86
// STARKNET_ACCOUNT_ADDRESS=0x3bb306a004034dba19e6cf7b161e7a4fef64bc1078419e8ad1876192f0b8cd1
// STARKNET_CAIRO_CORE_CONTRACT_ADDRESS=""
// STARKNET_FINALITY_RETRY_WAIT_IN_SECS=""
// MADARA_BINARY_PATH="/path/to/madara"


#[derive(Debug, Clone, Args)]
pub struct StarknetSettlementParams {
    /// The URL of the Ethereum RPC node.
    #[arg(env = "STARKNET_RPC_URL", long)]
    pub starknet_rpc_url: Url,
  
    /// The private key of the Ethereum account.
    #[arg(env = "STARKNET_PRIVATE_KEY", long)]
    pub starknet_private_key: String,

    /// The address of the Starknet account.
    #[arg(env = "STARKNET_ACCOUNT_ADDRESS", long)]
    pub starknet_account_address: String,

    /// The address of the Cairo core contract.
    #[arg(env = "STARKNET_CAIRO_CORE_CONTRACT_ADDRESS", long)]
    pub starknet_cairo_core_contract_address: String,

    /// The number of seconds to wait for finality.
    #[arg(env = "STARKNET_FINALITY_RETRY_WAIT_IN_SECS", long)]
    pub starknet_finality_retry_wait_in_secs: u64,

    /// The path to the Madara binary.
    #[arg(env = "MADARA_BINARY_PATH", long)]
    pub madara_binary_path: String,
    
}
