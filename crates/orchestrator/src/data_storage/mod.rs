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
    async fn get_snos_data_for_block(&self, block_number: u128) -> Result<StarknetOsOutput, Error>;
    async fn store_snos_data_for_block(
        &self,
        block_number: u128,
        data: StarknetOsOutput,
    ) -> Result<usize, Error>;
    async fn get_kzg_data_for_block(&self, block_number: u128) -> Result<String, Error>;
    async fn set_kzg_data_for_block(
        &self,
        block_number: u128,
        kzg_proof: &str,
    ) -> Result<usize, Error>;
}

pub trait DataStorageConfig {
    fn new_from_env() -> Self;
}
