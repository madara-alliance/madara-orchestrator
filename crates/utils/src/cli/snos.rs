use clap::Args;
use url::Url;

#[derive(Debug, Clone, Args)]
#[group(requires_all = ["rpc_for_snos"])]
pub struct SNOSCliArgs {
    /// The RPC URL for SNOS.
    #[arg(env = "RPC_FOR_SNOS", long)]
    pub rpc_for_snos: Url,

    /// The maximum block to process.
    #[arg(env = "MAX_BLOCK_TO_PROCESS", long)]
    pub max_block_to_process: Option<u64>,

    /// The minimum block to process.
    #[arg(env = "MIN_BLOCK_TO_PROCESS", long)]
    pub min_block_to_process: Option<u64>,
}
#[derive(Debug, Clone)]
pub struct SNOSParams {
    pub rpc_for_snos: Url,
    pub max_block_to_process: Option<u64>,
    pub min_block_to_process: Option<u64>,
}
