use url::Url;

#[derive(Debug, Clone)]
pub struct SharpParams {
    pub sharp_customer_id: String,
    pub sharp_url: Url,
    pub sharp_user_crt: String,
    pub sharp_user_key: String,
    pub sharp_rpc_node_url: Url,
    pub sharp_server_crt: String,
    pub sharp_proof_layout: String,
    pub gps_verifier_contract_address: String,
}
