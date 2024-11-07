use clap::Args;

#[derive(Debug, Clone, Args)]
#[group(requires_all = ["rpc_for_snos"])]
pub struct ServiceCliArgs {
    /// The maximum block to process.
    #[arg(env = "MAX_BLOCK_TO_PROCESS", long)]
    pub max_block_to_process: Option<u64>,

    /// The minimum block to process.
    #[arg(env = "MIN_BLOCK_TO_PROCESS", long)]
    pub min_block_to_process: Option<u64>,
}
