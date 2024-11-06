use clap::Args;
use url::Url;

/// Parameters used to config Ethereum.
#[derive(Debug, Clone, Args)]
pub struct EthereumParams {
    /// The RPC URL of the Ethereum node.
    #[arg(env = "DA_RPC_URL", long)]
    pub rpc_url: Url,
}
