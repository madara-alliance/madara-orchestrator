use crate::data_storage::aws_s3::config::{AWSS3Config, AWSS3ConfigType};
use crate::data_storage::DataStorage;
use async_trait::async_trait;
use aws_config::SdkConfig;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use bytes::Bytes;
use color_eyre::Result;

/// Module for AWS S3 config structs and implementations
pub mod config;

/// AWSS3 represents AWS S3 client object containing the client and the config itself.
pub struct AWSS3 {
    client: Client,
    bucket: String,
}

/// Implementation for AWS S3 client. Contains the function for :
///
/// - initializing a new AWS S3 client
impl AWSS3 {
    /// Initializes a new AWS S3 client by passing the config
    /// and returning it.
    pub fn new(s3_config: AWSS3Config, aws_config: &SdkConfig) -> Self {
        // Building AWS S3 config
        let client = Client::new(aws_config);
        Self { client, bucket: s3_config.bucket_name }
    }
}

/// Return the constructed `Credentials` and `Region`
fn get_credentials_and_region_from_config(
    s3_key_id: String,
    s3_key_secret: String,
    s3_bucket_region: String,
) -> (Credentials, Region) {
    let credentials = Credentials::new(s3_key_id, s3_key_secret, None, None, "loaded_from_custom_env");
    let region = Region::new(s3_bucket_region);
    (credentials, region)
}

/// Implementation of `DataStorage` for `AWSS3`
/// contains the function for getting the data and putting the data
/// by taking the key as an argument.
#[async_trait]
impl DataStorage for AWSS3 {
    /// Function to get the data from S3 bucket by Key.
    async fn get_data(&self, key: &str) -> Result<Bytes> {
        let response = self.client.get_object().bucket(&self.bucket).key(key).send().await?;
        let data_stream = response.body.collect().await.expect("Failed to convert body into AggregatedBytes.");
        let data_bytes = data_stream.into_bytes();
        Ok(data_bytes)
    }

    /// Function to put the data to S3 bucket by Key.
    async fn put_data(&self, data: Bytes, key: &str) -> Result<()> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data))
            .content_type("application/json")
            .send()
            .await?;

        Ok(())
    }

    #[cfg(test)]
    async fn build_test_bucket(&self, bucket_name: &str) -> Result<()> {
        self.client.create_bucket().bucket(bucket_name).send().await?;
        Ok(())
    }
}
