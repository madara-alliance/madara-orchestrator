use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct ServiceCliArgs {
    /// The maximum block to process.
    /// The default value is u64::MAX.
    #[arg(env = "MADARA_ORCHESTRATOR_MAX_BLOCK_NO_TO_PROCESS", long, default_value = Some("18446744073709551615"))]
    pub max_block_to_process: Option<u64>,

    /// The minimum block to process.
    #[arg(env = "MADARA_ORCHESTRATOR_MIN_BLOCK_NO_TO_PROCESS", long, default_value = Some("0"))]
    pub min_block_to_process: Option<u64>,

    /// The maximum number of SNOS jobs to process concurrently.
    #[arg(env = "MADARA_ORCHESTRATOR_MAX_CONCURRENT_SNOS_JOBS", long, default_value = Some("1"))]
    pub max_concurrent_snos_jobs: Option<usize>,
}
