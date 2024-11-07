pub mod aws_s3;
pub mod types;

use async_trait::async_trait;
use bytes::Bytes;
use color_eyre::Result;
use mockall::automock;

/// Data Storage Trait
///
/// DataStorage trait contains the functions used to store and get the data from
/// the cloud provider storage.
/// The proposed storage format is :
///     ----<block_number>
///         ----<cairo_pie.json> (stored during the SNOS job)
///         ----<snos_output.json> (stored during the SNOS job)
///         ----<blob_data.txt> (stored during the DA job)
#[automock]
#[async_trait]
pub trait DataStorage: Send + Sync {
    async fn get_data(&self, key: &str) -> Result<Bytes>;
    async fn put_data(&self, data: Bytes, key: &str) -> Result<()>;
    async fn build_test_bucket(&self, bucket_name: &str) -> Result<()>;
}
