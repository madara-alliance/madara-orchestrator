use clap::Args;
use url::Url;

#[derive(Debug, Clone, Args)]
pub struct SNOSParams {
    /// The RPC URL for SNOS.
    #[arg(env = "RPC_FOR_SNOS", long, required = true)]
    pub rpc_for_snos: Url,

    /// The maximum block to process.
    #[arg(env = "MAX_BLOCK_TO_PROCESS", long)]
    pub max_block_to_process: u64,

    /// The minimum block to process.
    #[arg(env = "MIN_BLOCK_TO_PROCESS", long)]
    pub min_block_to_process: u64,
}
