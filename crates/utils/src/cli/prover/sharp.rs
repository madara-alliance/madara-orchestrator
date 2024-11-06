use clap::Args;
use url::Url;

/// Parameters used to config Sharp.
#[derive(Debug, Clone, Args)]
pub struct SharpParams {
    /// The customer id for Sharp.
    #[arg(env = "SHARP_CUSTOMER_ID", long)]
    pub sharp_customer_id: String,

    /// The URL of the Sharp server.
    #[arg(env = "SHARP_URL", long)]
    pub sharp_url: Url,

    /// The user certificate for Sharp.
    #[arg(env = "SHARP_USER_CRT", long)]
    pub sharp_user_crt: String,

    /// The user key for Sharp.
    #[arg(env = "SHARP_USER_KEY", long)]
    pub sharp_user_key: String,

    /// The RPC node URL for Sharp.
    #[arg(env = "SHARP_RPC_NODE_URL", long)]
    pub sharp_rpc_node_url: Url,

    /// The server certificate for Sharp.
    #[arg(env = "SHARP_SERVER_CRT", long)]
    pub sharp_server_crt: String,

    /// The proof layout for Sharp.
    #[arg(env = "SHARP_PROOF_LAYOUT", long, default_value = "small")]
    pub sharp_proof_layout: String,

    /// The GPS verifier contract address.
    #[arg(env = "GPS_VERIFIER_CONTRACT_ADDRESS", long)]
    pub gps_verifier_contract_address: String,
}
