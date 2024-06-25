use crate::data_storage::aws_s3::config::AWSS3Config;
use crate::data_storage::types::StarknetOsOutput;
use crate::data_storage::DataStorage;
use async_trait::async_trait;
use aws_sdk_s3::config::{Builder, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{Client, Error};

pub mod config;

pub struct AWSS3 {
    client: Client,
    config: AWSS3Config,
}

impl AWSS3 {
    #[allow(dead_code)]
    pub async fn new(config: AWSS3Config) -> Self {
        // AWS cred building
        let credentials = Credentials::new(
            config.s3_key_id.clone(),
            config.s3_key_secret.clone(),
            None,
            None,
            "loaded_from_custom_env",
        );
        let region = Region::new(config.s3_bucket_region.clone().to_string());
        let conf_builder = Builder::new()
            .region(region)
            .credentials_provider(credentials);
        let conf = conf_builder.build();

        // Building AWS S3 config
        let client = Client::from_conf(conf);

        Self { client, config }
    }
}

#[async_trait]
impl DataStorage for AWSS3 {
    async fn get_data_for_block(&self, key: &str) -> Result<StarknetOsOutput, Error> {
        let response = self
            .client
            .get_object()
            .bucket(self.config.s3_bucket_name.clone())
            .key(key)
            .send()
            .await?;
        let data_stream = response
            .body
            .collect()
            .await
            .expect("Failed to convert body into AggregatedBytes.");
        let data_bytes = data_stream.into_bytes();
        let json: StarknetOsOutput =
            serde_json::from_slice(&data_bytes).expect("Failed to convert data_bytes into JSON.");

        Ok(json)
    }

    async fn put_data_for_block(&self, data: StarknetOsOutput, key: &str) -> Result<usize, Error> {
        let json_data =
            serde_json::to_vec(&data).expect("Failed to convert StarknetOsOutput into JSON.");
        let byte_stream = ByteStream::from(json_data.clone());
        self.client
            .put_object()
            .bucket(self.config.s3_bucket_name.clone())
            .key(key)
            .body(byte_stream)
            .content_type("application/json")
            .send()
            .await?;

        Ok(json_data.len())
    }
}
