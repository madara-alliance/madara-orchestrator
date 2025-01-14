use clap::Args;
use url::Url;

#[derive(Debug, Clone, Args)]
#[group(requires_all = ["rpc_for_snos"])]
pub struct SNOSCliArgs {
    #[arg(env = "MADARA_ORCHESTRATOR_SNOS_FULL_OUTPUT", long)]
    pub snos_full_output: bool,

    /// The RPC URL for SNOS.
    #[arg(env = "MADARA_ORCHESTRATOR_RPC_FOR_SNOS", long)]
    pub rpc_for_snos: Url,
}

#[derive(Debug, Clone)]
pub struct SNOSParams {
    pub snos_full_output: bool,
    pub rpc_for_snos: Url,
}
