use crate::config::config_force_init;
use crate::database::MockDatabase;
use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
use crate::queue::job_queue::JobQueueMessage;
use crate::queue::MockQueueProvider;
use crate::tests::common::init_config;
use crate::workers::snos::SnosWorker;
use crate::workers::Worker;
use da_client_interface::MockDaClient;
use httpmock::MockServer;
use rstest::rstest;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

#[rstest]
#[tokio::test]
async fn test_create_job() {
    let server = MockServer::start();
    let da_client = MockDaClient::new();
    let mut db = MockDatabase::new();
    let mut queue = MockQueueProvider::new();

    // Mocking db functions
    db.expect_get_latest_job_by_type().returning(|_| Ok(None)).call(JobType::SnosRun).expect("Failed to call.");
    // Getting jobs for check expectations
    for i in 1..6 {
        db.expect_get_job_by_internal_id_and_type()
            .returning(|_, _| Ok(None))
            .call(&i.to_string(), &JobType::SnosRun)
            .expect("Failed to call.");
    }

    // Creating jobs expectations
    for i in 1..6 {
        db.expect_create_job()
            .returning(|_| Ok(get_job_item_mock_by_id("1".to_string())))
            .call(get_job_item_mock_by_id(i.to_string()))
            .expect("Failed to call");
    }

    // Queue function call simulations
    queue
        .expect_send_message_to_queue()
        .returning(|_, _, _| Ok(()))
        .call(
            "madara_orchestrator_job_processing_queue".to_string(),
            serde_json::to_string(&JobQueueMessage { id: Uuid::new_v4() }).unwrap(),
            None,
        )
        .expect("Failed to call");

    // mock block number (madara) : 5
    let rpc_response_block_number = 5;
    let response = json!({ "id": 1,"jsonrpc":"2.0","result": rpc_response_block_number });

    let config =
        init_config(Some(format!("http://localhost:{}", server.port())), Some(db), Some(queue), Some(da_client)).await;
    config_force_init(config).await;

    // mocking block call
    let rpc_block_call_mock = server.mock(|when, then| {
        when.path("/").body_contains("starknet_blockNumber");
        then.status(200).body(serde_json::to_vec(&response).unwrap());
    });

    let snos_worker = SnosWorker {};
    snos_worker.run_worker().await;

    rpc_block_call_mock.assert();
}

fn get_job_item_mock_by_id(id: String) -> JobItem {
    let uuid = Uuid::new_v4();

    JobItem {
        id: uuid,
        internal_id: id.clone(),
        job_type: JobType::SnosRun,
        status: JobStatus::Created,
        external_id: ExternalId::Number(0),
        metadata: HashMap::new(),
        version: 0,
    }
}
