use clap::Args;
use url::Url;

/// Parameters used to config Ethereum.
#[derive(Debug, Clone, Args)]
#[group(requires_all = ["da_rpc_url"])]
pub struct EthereumDaCliArgs {
    /// Use the Ethereum DA layer.
    #[arg(long)]
    pub da_on_ethereum: bool,

    /// The RPC URL of the Ethereum node.
    #[arg(env = "MADARA_ORCHESTRATOR_ETHEREUM_DA_RPC_URL", long)]
    pub da_rpc_url: Option<Url>,
}
