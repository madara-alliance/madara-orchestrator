pub mod config;
pub mod error;

use color_eyre::Result;
use async_trait::async_trait;
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
        let blobs : Result<Vec<Blob>, _> = state_diff.into_iter().map(|blob_data| Blob::new(self.nid, blob_data)).collect();
        
        // Submit the blobs to celestia
        let height = self.celestia_client.blob_submit(blobs?.as_slice(),  GasPrice::default()).await.expect("Failed submitting blobs");
        
        // // Return back the height of the block that will contain the blob.
        Ok(height.to_string())
    }

    async fn verify_inclusion(&self, external_id: &str) -> Result<DaVerificationStatus> {
        // TODO: check if feasible and needed ? we can send blob.commitment as a part of external id and use it to call get_blob rather than get_all for more precise answer.

        // External Id : Height of Block the blob is submitted to.
        // 2 ways to check : 
        // Is the current block ahead of the block that we have, if it is does my block contain the blob.
        // Call the blob_get_all function of the client

        let height_id = external_id.parse()?;
        let retrieved_blobs = self.celestia_client.blob_get_all(height_id, &[self.nid]).await;

        //TODO: Assumption: Given that we are sending only 1 nid, we'll get an array of 1 object back.

        match retrieved_blobs {
            Ok(blobs) => {
                if blobs.len() == 1 {
                    Ok(DaVerificationStatus::Verified)
                } else {
                    Ok(DaVerificationStatus::Rejected(format!("Expected 1 blob, but got {}", blobs.len())))
                }
            }
            Err(e) => Ok(DaVerificationStatus::Rejected(format!("Error occurred: {}", e))),
        }

    }

    async fn max_blob_per_txn(&self) -> u64 {
        //Info: No docs suggest a number. 
        1
    }

    async fn max_bytes_per_blob(&self) -> u64 {
        //Info: https://github.com/celestiaorg/celestia-node/issues/3356
        1974272
    }
}

impl TryFrom<config::CelestiaConfig> for CelestiaDaClient {
    type Error = anyhow::Error;
    fn try_from(conf: config::CelestiaConfig) -> Result<Self, Self::Error> {
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
        let nid = Namespace::new_v0(bytes).map_err(|e| CelestiaDaError::Generic(format!("could not init namespace: {e}"))).unwrap();

        Ok(Self { celestia_client: http_client, nid })
    }
}




#[cfg(test)]
mod tests {

    use config::{CelestiaConfig,  DEFAULT_CELESTIA_NODE, DEFAULT_NID};

    use super::*;

    // async fn test_celestia_da_client(){
    //     let config = CelestiaConfig {
    //         http_provider: DEFAULT_CELESTIA_NODE.to_string(),
    //         auth_token: None,
    //         nid: DEFAULT_NID.to_string(),
    //     };
    //     // Instantiate CelestiaDaClient
    //     let celestia_da_client = CelestiaDaClient::try_from(config).unwrap();

    //     celestia_da_client.publish_state_diff(state_diff, to);


    // }

    // async fn test_verify_inclusion(){


    // }


    #[tokio::test]
    async fn test_max_blob_per_txn(){
        let expected_value:u64 = 1;

        println!("{}",DEFAULT_CELESTIA_NODE);

        let config = CelestiaConfig {
            http_provider: DEFAULT_CELESTIA_NODE.to_string(),
            auth_token: None,
            nid: DEFAULT_NID.to_string(),
        };
        // Instantiate CelestiaDaClient
        let celestia_da_client = CelestiaDaClient::try_from(config).unwrap();

        let max_blobs_per_txn = celestia_da_client.max_blob_per_txn().await;
        assert_eq!(max_blobs_per_txn, expected_value);
    }

    #[tokio::test]
    async fn test_max_bytes_per_blob(){
        let expected_value:u64 = 1974272;

        let config = CelestiaConfig {
            http_provider: DEFAULT_CELESTIA_NODE.to_string(),
            auth_token: None,
            nid: DEFAULT_NID.to_string(),
        };
        // Instantiate CelestiaDaClient
        let celestia_da_client = CelestiaDaClient::try_from(config).unwrap();

        let max_bytes_per_blob = celestia_da_client.max_bytes_per_blob().await;
        assert_eq!(max_bytes_per_blob,expected_value);
    }


}