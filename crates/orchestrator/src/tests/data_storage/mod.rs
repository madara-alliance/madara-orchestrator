use crate::data_storage::aws_s3::config::AWSS3Config;
use crate::data_storage::aws_s3::AWSS3;
use crate::data_storage::{DataStorage, DataStorageConfig};
use bytes::Bytes;
use dotenvy::dotenv;
use rstest::rstest;
use serde_json::json;

#[rstest]
#[tokio::test]
async fn test_put_and_get_data_s3() -> color_eyre::Result<()> {
    dotenv().ok();
    let config = AWSS3Config::new_from_env();
    let s3_client = AWSS3::new(config).await;

    let mock_data = json!(
        {
            "body" : "hello world. hello world."
        }
    );
    let json_bytes = serde_json::to_vec(&mock_data)?;
    let key = "test_data.txt";

    // putting test data on key : "test_data.txt"
    s3_client.put_data(Bytes::from(json_bytes), key).await.expect("Unable to put data into the bucket.");

    // getting the data from key : "test_data.txt"
    let data = s3_client.get_data(key).await.expect("Unable to get the data from the bucket.");
    let received_json: serde_json::Value = serde_json::from_slice(&data)?;

    assert_eq!(received_json, mock_data);

    Ok(())
}
