pub mod config;
pub mod error;

use async_trait::async_trait;
use color_eyre::Result;
use error::CelestiaDaError;
use jsonrpsee::http_client::{HeaderMap, HeaderValue, HttpClient, HttpClientBuilder};
use reqwest::header;

use celestia_rpc::BlobClient;
use celestia_types::blob::GasPrice;
use celestia_types::{nmt::Namespace, Blob};

use da_client_interface::{DaClient, DaVerificationStatus};

#[derive(Clone, Debug)]
pub struct CelestiaDaClient {
    celestia_client: HttpClient,
    nid: Namespace,
}

#[async_trait]
impl DaClient for CelestiaDaClient {
    async fn publish_state_diff(&self, state_diff: Vec<Vec<u8>>, to: &[u8; 32]) -> Result<String> {
        // Convert the state_diffs into Blobs
        let blobs: Result<Vec<Blob>, _> =
            state_diff.into_iter().map(|blob_data| Blob::new(self.nid, blob_data)).collect();

        // Submit the blobs to celestia
        let height = self
            .celestia_client
            .blob_submit(blobs?.as_slice(), GasPrice::default())
            .await?;

        // // Return back the height of the block that will contain the blob.
        Ok(height.to_string())
    }

    async fn verify_inclusion(&self, external_id: &str) -> Result<DaVerificationStatus> {
        // https://node-rpc-docs.celestia.org/?version=v0.13.7#blob.Submit
        // Our Oberservation: 
            // 1) Submit sends Blobs and reports the height in which they were included.
            // 2) It takes submit 1-15 seconds (under right network conditions) depending on the nearest block.
        // Assumption :
            // blob.Submit is a blocking call that returns only when the BLOCK HAS BEEN INCLUDED.

        Ok(DaVerificationStatus::Verified)
    }

    async fn max_blob_per_txn(&self) -> u64 {
        //Info: No docs suggest a number, default to 1.
        1
    }

    async fn max_bytes_per_blob(&self) -> u64 {
        //Info: https://docs.celestia.org/nodes/mainnet#maximum-bytes 
        1973786
    }
}

impl TryFrom<config::CelestiaDaConfig> for CelestiaDaClient {
    type Error = anyhow::Error;
    fn try_from(conf: config::CelestiaDaConfig) -> Result<Self, Self::Error> {
        // Borrowed the below code from https://github.com/eigerco/lumina/blob/ccc5b9bfeac632cccd32d35ecb7b7d51d71fbb87/rpc/src/client.rs#L41.
        // Directly calling the function wasn't possible as the function is async. Since
        // we only need to initiate the http provider and not the ws provider, we don't need async

        let mut headers = HeaderMap::new();

        // checking if Auth is available
        if let Some(auth_token) = conf.auth_token {
            let val = HeaderValue::from_str(&format!("Bearer {}", auth_token))?;
            headers.insert(header::AUTHORIZATION, val);
        }

        let http_client = HttpClientBuilder::default()
            .set_headers(headers)
            .build(conf.http_provider.as_str())
            .map_err(|e| CelestiaDaError::Client(format!("could not init http client: {e}")))?;

        // Convert the input string to bytes
        let bytes = conf.nid.as_bytes();

        // Create a new Namespace from these bytes
        let nid = Namespace::new_v0(bytes)
            .map_err(|e| CelestiaDaError::Generic(format!("could not init namespace: {e}")))
            .unwrap();

        Ok(Self { celestia_client: http_client, nid })
    }
}

/*
celestia-node - Steps : 
1. Run celestia-node, preferred impl https://docs.celestia.org/nodes/docker-images.
2. Ensure to safely note down the account information provided to use later on.
3. Ensure to manually fund the account, see https://docs.celestia.org/nodes/arabica-devnet#arabica-devnet-faucet.
4. Ensure that the account is detected by celestia-node, see https://docs.celestia.org/developers/celestia-node-key#docker-and-cel-key.
5. Remove the #ignores to run the tests.

Shortcut method to run Celestia as DA : 
 - define $NETWORK, $RPC_URL, $NODE_TYPE, see https://docs.celestia.org/nodes/docker-images#quick-start.
 - skips Auth, setup from https://node-rpc-docs.celestia.org/?version=v0.13.7#node.AuthNew.
 - exposes 26658 for RPC communication: https://docs.celestia.org/nodes/celestia-node-troubleshooting#ports, binds it to 8000 of host.
    ```bash 
    docker run --expose 26658 -p 8000:26658 -e NODE_TYPE=$NODE_TYPE -e P2P_NETWORK=$NETWORK -v $HOME/<path-to-folder>:/home/celestia ghcr.io/celestiaorg/celestia-node:v0.14.0 celestia light start --core.ip $RPC_URL --p2p.network $NETWORK --rpc.port 26658 --rpc.addr 0.0.0.0 --rpc.skip-auth
    ```
 - [only for testnet/devnet] Then copy paste all files from `.celestia-light-<network_type>/keys` to `.celestia-light/keys`, check if account is getting detected, see https://docs.celestia.org/developers/celestia-node-key#using-the-cel-key-utility.
 */

#[cfg(test)]
mod tests {

    use config::CelestiaDaConfig;
    use da_client_interface::DaConfig;

    use super::*;

    #[tokio::test]
    #[ignore = "Can't run without manual intervention, setup celestia-node and fund address."]
    async fn test_celestia_publish_state_diff_and_verify_inclusion() {
        let config: CelestiaDaConfig = CelestiaDaConfig::new_from_env();
        let celestia_da_client = CelestiaDaClient::try_from(config).unwrap();

        let s = "Hello World!";
        let bytes: Vec<u8> = s.bytes().collect();
        let state_diff = vec![bytes];

        let to: [u8; 32] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32, 0x10, 0x00, 0x11,
            0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];

        let height_id = celestia_da_client.publish_state_diff(state_diff, &to).await.expect("Problem reading:");

        let inclusion_response = celestia_da_client.verify_inclusion(&height_id).await.expect("Problem reading:");

        assert_eq!(inclusion_response, DaVerificationStatus::Verified);

    }
}
