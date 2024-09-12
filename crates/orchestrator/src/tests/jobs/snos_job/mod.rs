use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use chrono::{SubsecRound, Utc};
use httpmock::MockServer;
use mockall::predicate::eq;
use rstest::*;
use serde_json::json;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use url::Url;
use uuid::Uuid;

use crate::data_storage::MockDataStorage;
use crate::jobs::snos_job::SnosJob;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;
use crate::tests::common::default_job_item;
use crate::tests::config::TestConfigBuilder;

#[rstest]
#[tokio::test]
async fn test_create_job() {
    let services = TestConfigBuilder::new().build().await;

    let job = SnosJob.create_job(services.config.clone(), String::from("0"), HashMap::new()).await;

    assert!(job.is_ok());
    let job = job.unwrap();

    let job_type = job.job_type;
    assert_eq!(job_type, JobType::SnosRun, "job_type should be SnosRun");
    assert!(!(job.id.is_nil()), "id should not be nil");
    assert_eq!(job.status, JobStatus::Created, "status should be Created");
    assert_eq!(job.version, 0_i32, "version should be 0");
    assert_eq!(job.external_id.unwrap_string().unwrap(), String::new(), "external_id should be empty string");
}

#[rstest]
#[tokio::test]
async fn test_verify_job(#[from(default_job_item)] mut job_item: JobItem) {
    let services = TestConfigBuilder::new().build().await;
    let job_status = SnosJob.verify_job(services.config.clone(), &mut job_item).await;

    // Should always be [Verified] for the moment.
    assert_eq!(job_status, Ok(JobVerificationStatus::Verified));
}

#[rstest]
#[tokio::test]
async fn test_process_job() {
    // Set up mock server
    let server = MockServer::start();

    // Set up mock storage
    let mut storage = MockDataStorage::new();
    let mock_cairo_pie = Bytes::from("mock cairo pie data");
    let mock_snos_output = Bytes::from("mock snos output data");
    storage.expect_get_data().with(eq("1/cairo_pie.zip")).times(1).return_once(move |_| Ok(mock_cairo_pie));
    storage.expect_get_data().with(eq("1/snos_output.json")).times(1).return_once(move |_| Ok(mock_snos_output));

    // Expect storage to be called twice for putting data (Cairo PIE and SNOS output)
    storage
        .expect_put_data()
        .with(eq(Bytes::from("mock cairo pie data")), eq("1/cairo_pie.zip"))
        .times(1)
        .return_once(|_, _| Ok(()));

    storage
        .expect_put_data()
        .with(eq(Bytes::from("mock snos output data")), eq("1/snos_output.json"))
        .times(1)
        .return_once(|_, _| Ok(()));

    // Set up mock Starknet provider
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse(format!("http://localhost:{}", server.port()).as_str()).expect("Failed to parse URL"),
    ));

    // Set up environment variables
    std::env::set_var("MADARA_RPC_URL", format!("http://localhost:{}", server.port()));

    // Build test configuration
    let services = TestConfigBuilder::new()
        .configure_starknet_client(provider.into())
        .configure_storage_client(storage.into())
        .build()
        .await;

    // Create job item
    let mut job_item = JobItem {
        id: Uuid::new_v4(),
        internal_id: "1".into(),
        job_type: JobType::SnosRun,
        status: JobStatus::Created,
        external_id: String::new().into(),
        metadata: HashMap::from([("block_number".to_string(), "1".to_string())]),
        version: 0,
        created_at: Utc::now().round_subsecs(0),
        updated_at: Utc::now().round_subsecs(0),
    };

    // Mock the Starknet RPC call
    server.mock(|when, then| {
        when.method("starknet_getBlockWithTxs").json_body(json!({"block_number": 1}));
        then.status(200).json_body(json!({
            "result": {
                "block_hash": "0x1234",
                "parent_hash": "0x5678",
                "block_number": 1,
                "state_root": "0xabcd",
                "transactions": []
            }
        }));
    });

    let result = SnosJob.process_job(Arc::clone(&services.config), &mut job_item).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "1");
}
