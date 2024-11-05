use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct SNOSParams {
    /// The RPC URL for SNOS.
    #[arg(env = "RPC_FOR_SNOS", long)]
    pub rpc_for_snos: String,

    /// The maximum block to process.
    #[arg(env = "MAX_BLOCK_TO_PROCESS", long)]
    pub max_block_to_process: String,

    /// The minimum block to process.
    #[arg(env = "MIN_BLOCK_TO_PROCESS", long)]
    pub min_block_to_process: String,
}
