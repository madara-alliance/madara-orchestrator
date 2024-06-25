use crate::data_storage::aws_s3::config::AWSS3Config;
use crate::data_storage::types::StarknetOsOutput;
use crate::data_storage::DataStorage;
use async_trait::async_trait;
use aws_sdk_s3::config::{Builder, Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::{Client, Error};

pub mod config;

const SNOS_OUTPUT_FILE_NAME: &str = "snos_output.json";
const KZG_FILE_NAME: &str = "kzg.txt";

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
        let conf_builder = Builder::new().region(region).credentials_provider(credentials);
        let conf = conf_builder.build();

        // Building AWS S3 config
        let client = Client::from_conf(conf);

        Self { client, config }
    }
}

#[async_trait]
impl DataStorage for AWSS3 {
    async fn get_snos_data_for_block(&self, block_number: u128) -> Result<StarknetOsOutput, Error> {
        let key = format!("{}/{}", block_number, SNOS_OUTPUT_FILE_NAME);
        let response = self.client.get_object().bucket(self.config.s3_bucket_name.clone()).key(key).send().await?;
        let data_stream = response.body.collect().await.expect("Failed to convert body into AggregatedBytes.");
        let data_bytes = data_stream.into_bytes();
        let json: StarknetOsOutput =
            serde_json::from_slice(&data_bytes).expect("Failed to convert data_bytes into JSON.");

        Ok(json)
    }

    async fn store_snos_data_for_block(&self, block_number: u128, data: StarknetOsOutput) -> Result<usize, Error> {
        let json_data = serde_json::to_vec(&data).expect("Failed to convert StarknetOsOutput into JSON.");
        let key = format!("{}/{}", block_number, SNOS_OUTPUT_FILE_NAME);
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

    async fn get_kzg_data_for_block(&self, block_number: u128) -> Result<String, Error> {
        let key = format!("{}/{}", block_number, KZG_FILE_NAME);
        let response = self.client.get_object().bucket(self.config.s3_bucket_name.clone()).key(key).send().await?;
        let data_stream = response.body.collect().await.expect("Failed to convert body into AggregatedBytes.");
        let data_bytes = data_stream.into_bytes();

        // Convert the bytes to a UTF-8 string
        let text = String::from_utf8(data_bytes.to_vec()).expect("Failed to convert data_bytes into String.");

        Ok(text)
    }

    async fn set_kzg_data_for_block(&self, block_number: u128, kzg_proof: &str) -> Result<usize, Error> {
        let byte_stream = ByteStream::from(kzg_proof.as_bytes().to_vec());
        let key = format!("{}/{}", block_number, SNOS_OUTPUT_FILE_NAME);

        // Uploading the text file
        self.client
            .put_object()
            .bucket(self.config.s3_bucket_name.clone())
            .key(key)
            .body(byte_stream)
            .content_type("application/json")
            .send()
            .await?;

        Ok(kzg_proof.len())
    }
}
