use crate::jobs::da_job::DaJob;
use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
use crate::{
    config::{config, TestConfigBuilder},
    jobs::Job,
};
use color_eyre::{eyre::eyre, Result};
use da_client_interface::MockDaClient;
use rstest::*;
use serde_json::json;
use starknet_core::types::{FieldElement, MaybePendingStateUpdate, PendingStateUpdate, StateDiff, StateUpdate};
use std::collections::HashMap;
use utils::env_utils::get_env_var_or_panic;
use uuid::Uuid;

// TODO : How to know which DA client we have enabled ?
// feature flag the tests ?

#[rstest]
async fn test_da_job_process_job_failure_on_impossible_blob_length() -> Result<()> {
    // Mocking DA client calls
    let mut da_client = MockDaClient::new();
    da_client.expect_max_blob_per_txn().with().returning(|| 6);
    da_client.expect_max_bytes_per_blob().with().returning(|| 131072);

    let server = TestConfigBuilder::new().mock_da_client(Box::new(da_client)).build().await;
    let config = config().await;

    let internal_id = "1";

    let state_update = MaybePendingStateUpdate::Update(StateUpdate {
        block_hash: FieldElement::default(),
        new_root: FieldElement::default(),
        old_root: FieldElement::default(),
        state_diff: StateDiff {
            storage_diffs: vec![],
            deprecated_declared_classes: vec![],
            declared_classes: vec![],
            deployed_contracts: vec![],
            replaced_classes: vec![],
            nonces: vec![],
        },
    });
    let state_update = serde_json::to_value(&state_update).unwrap();
    let response = json!({ "id": 1,"jsonrpc":"2.0","result": state_update });

    let state_update_mock = server.mock(|when, then| {
        when.path(get_env_var_or_panic("MADARA_RPC_URL").as_str()).body_contains("starknet_getStateUpdate");
        then.status(200).body(serde_json::to_vec(&response).unwrap());
    });

    state_update_mock.assert();

    let max_blob_per_txn = config.da_client().max_blob_per_txn().await;
    let current_blob_length: u64 = 100;

    assert_eq!(
        DaJob
            .process_job(
                config.as_ref(),
                &mut JobItem {
                    id: Uuid::default(),
                    internal_id: internal_id.to_string(),
                    job_type: JobType::DataSubmission,
                    status: JobStatus::Created,
                    external_id: ExternalId::String("1".to_string().into_boxed_str()),
                    metadata: HashMap::default(),
                    version: 0,
                }
            )
            .await
            .unwrap(),
        eyre!(
            "Exceeded the maximum number of blobs per transaction: allowed {}, found {} for block {} and job id {}",
            max_blob_per_txn,
            current_blob_length,
            internal_id.to_string(),
            Uuid::default()
        )
        .to_string()
    );

    Ok(())
}

#[rstest]
async fn test_da_job_process_job_failure_on_pending_block() -> Result<()> {
    // Mocking DA client calls
    let mut da_client = MockDaClient::new();
    da_client.expect_max_blob_per_txn().with().returning(|| 6);
    da_client.expect_max_bytes_per_blob().with().returning(|| 131072);

    let server = TestConfigBuilder::new().mock_da_client(Box::new(da_client)).build().await;
    let config = config().await;
    let internal_id = "1";

    let pending_state_update = MaybePendingStateUpdate::PendingUpdate(PendingStateUpdate {
        old_root: FieldElement::default(),
        state_diff: StateDiff {
            storage_diffs: vec![],
            deprecated_declared_classes: vec![],
            declared_classes: vec![],
            deployed_contracts: vec![],
            replaced_classes: vec![],
            nonces: vec![],
        },
    });

    let pending_state_update = serde_json::to_value(&pending_state_update).unwrap();
    let response = json!({ "id": 1,"jsonrpc":"2.0","result": pending_state_update });

    let state_update_mock = server.mock(|when, then| {
        when.path(get_env_var_or_panic("MADARA_RPC_URL").as_str()).body_contains("starknet_getStateUpdate");
        then.status(200).body(serde_json::to_vec(&response).unwrap());
    });

    state_update_mock.assert();

    assert_eq!(
        DaJob
            .process_job(
                config.as_ref(),
                &mut JobItem {
                    id: Uuid::default(),
                    internal_id: internal_id.to_string(),
                    job_type: JobType::DataSubmission,
                    status: JobStatus::Created,
                    external_id: ExternalId::String("1".to_string().into_boxed_str()),
                    metadata: HashMap::default(),
                    version: 0,
                }
            )
            .await
            .unwrap(),
            eyre!(
                "Cannot process block {} for job id {} as it's still in pending state",
                internal_id.to_string(),
                Uuid::default()
            ).to_string()
    );

    Ok(())
}

#[rstest]
async fn test_da_job_process_job_success() -> Result<()> {
    // Mocking DA client calls
    let mut da_client = MockDaClient::new();
    da_client.expect_max_blob_per_txn().with().returning(|| 6);
    da_client.expect_max_bytes_per_blob().with().returning(|| 131072);

    let server = TestConfigBuilder::new().mock_da_client(Box::new(da_client)).build().await;
    let config = config().await;
    let internal_id = "1";

    let state_update = MaybePendingStateUpdate::Update(StateUpdate {
        block_hash: FieldElement::default(),
        new_root: FieldElement::default(),
        old_root: FieldElement::default(),
        state_diff: StateDiff {
            storage_diffs: vec![],
            deprecated_declared_classes: vec![],
            declared_classes: vec![],
            deployed_contracts: vec![],
            replaced_classes: vec![],
            nonces: vec![],
        },
    });
    let state_update = serde_json::to_value(&state_update).unwrap();
    let response = json!({ "id": 1,"jsonrpc":"2.0","result": state_update });

    let state_update_mock = server.mock(|when, then| {
        when.path(get_env_var_or_panic("MADARA_RPC_URL").as_str()).body_contains("starknet_getStateUpdate");
        then.status(200).body(serde_json::to_vec(&response).unwrap());
    });

    state_update_mock.assert();

    assert_eq!(
        DaJob
            .process_job(
                config.as_ref(),
                &mut JobItem {
                    id: Uuid::default(),
                    internal_id: internal_id.to_string(),
                    job_type: JobType::DataSubmission,
                    status: JobStatus::Created,
                    external_id: ExternalId::String("1".to_string().into_boxed_str()),
                    metadata: HashMap::default(),
                    version: 0,
                }
            )
            .await
            .unwrap(),
        "0xbeef"
    );

    Ok(())
}
