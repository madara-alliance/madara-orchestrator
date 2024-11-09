
use serde::{Deserialize, Serialize};
use url::Url;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EthereumDaParams {
    pub ethereum_da_rpc_url: Url,
}
