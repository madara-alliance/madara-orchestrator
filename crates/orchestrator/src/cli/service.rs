use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct ServiceCliArgs {
    /// The maximum block to process.
    #[arg(env = "MADARA_ORCHESTRATOR_MAX_BLOCK_NO_TO_PROCESS", long)]
    pub max_block_to_process: Option<String>,

    /// The minimum block to process.
    #[arg(env = "MADARA_ORCHESTRATOR_MIN_BLOCK_NO_TO_PROCESS", long)]
    pub min_block_to_process: Option<String>,

    /// The Unique Id to describe the runner.
    #[arg(env = "MADARA_ORCHESTRATOR_SERVICE_ID", long)]
    pub service_id: Option<String>,
}
