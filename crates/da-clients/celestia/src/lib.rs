pub mod config;
pub mod error;

use anyhow::Result;
use async_trait::async_trait;
use error::CelestiaDaError;

use celestia_rpc::{BlobClient, Client, HeaderClient, ShareClient};
use celestia_types::blob::GasPrice;
use celestia_types::{nmt::Namespace, Blob, ExtendedDataSquare};

use da_client_interface::{DaClient, DaVerificationStatus};
use crate::{DaClient, DaMode};

#[derive(Clone, Debug)]
pub struct CelestiaDaClient {
    celestia_client: Client,
    nid: Namespace,
    mode: DaMode,
}

#[automock]
#[async_trait]
impl DaClient for CelestiaDaClient {
    async fn publish_state_diff(&self, state_diff: Vec<Vec<u8>>, to: &[u8; 32]) -> Result<String> {
        // Convert the state_diffs into Blobs
        let blobs = state_diff.iter().map(|&blob_data| Blob::new(self.nid, blob_data)).collect();
        // Submit the blobs to celestia
        let height = self.celestia_client.blob_submit(blobs,  GasPrice::default()).await.expect("Failed submitting blobs");
        // Return back the height of the block that will contain the blob.
        Ok(height.to_string())
    }

    async fn verify_inclusion(&self, external_id: &str) -> Result<DaVerificationStatus> {

        // TODO: check if feasible and needed ? we can send blob.commitment as a part of external id and use it to call get_blob rather than get_all for more precise answer.

        // External Id : Height of Block the blob is submitted to.
        // 2 ways to check : 
        // Is the current block ahead of the block that we have, if it is does my block contain the blob.
        // Call the blob_get_all function of the client

        let height: String = external_id.parse().unwrap();
        let height_id: u64 = match s.parse::<u64>() {
            Ok(n) => n,
            Err(e) => {
                // TODO: check ifthis returns 
                CelestiaDaError::Generic((format!("Failed to convert string to u64: {e}")));
            }
        };

        let retrieved_blobs = self.celestia_client.blob_get_all(height_id, self.nid).await
        .expect(CelestiaDaError::Generic((format!("could not init namespace"))));
        
        // TODO: Assumption: Given that we are sending only 1 nid, we'll get an array of 1 object back.

        match retrieved_blobs[0].validate() {
            Ok => Ok(DaVerificationStatus::Verified),
            Err(e) => Ok(DaVerificationStatus::Rejected(format!("Verification failed", e.to_string()))),
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

impl CelestiaClient {
    async fn publish_data(&self, blob: &Blob) -> Result<u64> {
        self.http_client
            .blob_submit(&[blob.clone()], SubmitOptions::default())
            .await
            .map_err(|e| CelestiaDaError::Client(format!("Celestia RPC error: {e}")))
    }

    fn get_blob_from_state_diff(&self, state_diff: Vec<U256>) -> CelestiaTypesResult<Blob> {
        let state_diff_bytes: Vec<u8> = state_diff
            .iter()
            .flat_map(|item| {
                let mut bytes = [0_u8; 32];
                item.to_big_endian(&mut bytes);
                bytes.to_vec()
            })
            .collect();

        Blob::new(self.nid, state_diff_bytes)
    }

    async fn verify_blob_was_included(&self, submitted_height: u64, blob: Blob) -> Result<()> {
        let received_blob = self.http_client.blob_get(submitted_height, self.nid, blob.commitment).await.unwrap();
        received_blob.validate()?;
        Ok(())
    }
}

impl TryFrom<config::CelestiaConfig> for CelestiaClient {
    type Error = anyhow::Error;

    fn try_from(conf: config::CelestiaConfig) -> Result<Self, Self::Error> {
        // Borrowed the below code from https://github.com/eigerco/lumina/blob/ccc5b9bfeac632cccd32d35ecb7b7d51d71fbb87/rpc/src/client.rs#L41.
        // Directly calling the function wasn't possible as the function is async. Since
        // we only need to initiate the http provider and not the ws provider, we don't need async

        let client = Client::new(&conf.http_provider, Some(&conf.auth_token))
        .await
        // TODO: user Celestia DA Errors
        .expect(CelestiaDaError::Client((format!("Failed creating rpc client"))));

        // Convert the input string to bytes
        let bytes = conf.nid.as_bytes();

        // Create a new Namespace from these bytes
        let nid = Namespace::new_v0(bytes).map_err(|e| CelestiaDaError::Generic(format!("could not init namespace: {e}")));

        Ok(Self { client, nid, mode: conf.mode })
    }
}