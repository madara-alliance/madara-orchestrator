mod aws_s3;
mod types;

use crate::data_storage::types::StarknetOsOutput;
use async_trait::async_trait;
use aws_sdk_s3::Error;
use mockall::automock;

/// DataStorage trait contains the functions used to store and get the data from
/// the cloud provider storage.
/// The proposed storage format is :
/// ----s3
///     ----<block_number>
///         ----<snos_output.json>
///         ----<kzg.txt>
#[automock]
#[async_trait]
pub trait DataStorage: Send + Sync {
    async fn get_data_for_block(&self, key: &str) -> Result<StarknetOsOutput, Error>;
    async fn put_data_for_block(&self, data: StarknetOsOutput, key: &str) -> Result<usize, Error>;
}

pub trait DataStorageConfig {
    fn new_from_env() -> Self;
}
