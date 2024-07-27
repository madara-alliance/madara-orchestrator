use crate::jobs::da_job::da_word;
use crate::jobs::da_job::DaJob;
use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
use crate::tests::common::drop_database;
use crate::{
    config::{config, TestConfigBuilder},
    jobs::Job,
};
use color_eyre::{eyre::eyre, Result};
use da_client_interface::MockDaClient;
use mockall::predicate::always;
use serde_json::json;
use starknet_core::types::{FieldElement, MaybePendingStateUpdate, PendingStateUpdate, StateDiff, StateUpdate};
use std::collections::HashMap;
use uuid::Uuid;

use std::fs;
use std::fs::File;
use std::io::Read;

use ::serde::{Deserialize, Serialize};
use httpmock::prelude::*;
use majin_blob_core::blob;
use majin_blob_types::serde;
use majin_blob_types::state_diffs::UnorderedEq;
// use majin_blob_types::serde;
use crate::data_storage::MockDataStorage;
use rstest::rstest;

use crate::tests::common::init_config;

#[rstest]
#[case(
    "src/tests/jobs/da_job/test_data/state_update/640641.txt",
    "src/tests/jobs/da_job/test_data/nonces/640641.txt",
    "640641",
    110
)]
#[case(
    "src/tests/jobs/da_job/test_data/state_update/631861.txt",
    "src/tests/jobs/da_job/test_data/nonces/631861.txt",
    "631861",
    110
)]
#[case(
    "src/tests/jobs/da_job/test_data/state_update/638353.txt",
    "src/tests/jobs/da_job/test_data/nonces/638353.txt",
    "63853",
    110
)]
#[tokio::test]
async fn test_da_job_process_job_failure_on_impossible_blob_length(
    #[case] state_update_file: String,
    #[case] noances_file: String,
    #[case] internal_id: String,
    #[case] current_blob_length: u64,
) -> Result<()> {
    dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

    // Mocking DA client calls
    let mut da_client = MockDaClient::new();
    // dummy state will have more than 1200 bytes
    da_client.expect_max_blob_per_txn().with().returning(|| 1);
    da_client.expect_max_bytes_per_blob().with().returning(|| 1200);

    let server = TestConfigBuilder::new().mock_da_client(Box::new(da_client)).build().await;
    let config = config().await;

    let state_update = read_state_update_from_file(state_update_file.as_str()).expect("issue while reading");

    let state_update = MaybePendingStateUpdate::Update(state_update);
    let state_update = serde_json::to_value(&state_update).unwrap();
    let response = json!({ "id": 640641,"jsonrpc":"2.0","result": state_update });

    get_nonce_attached(&server, noances_file.as_str());

    let state_update_mock = server.mock(|when, then| {
        when.path("/").body_contains("starknet_getStateUpdate");
        then.status(200).body(serde_json::to_vec(&response).unwrap());
    });

    let max_blob_per_txn = config.da_client().max_blob_per_txn().await;

    let response = DaJob
        .process_job(
            config.as_ref(),
            &mut JobItem {
                id: Uuid::default(),
                internal_id: internal_id.to_string(),
                job_type: JobType::DataSubmission,
                status: JobStatus::Created,
                external_id: ExternalId::String(internal_id.to_string().into_boxed_str()),
                metadata: HashMap::default(),
                version: 0,
            },
        )
        .await;

    let expected = eyre!(
        "Exceeded the maximum number of blobs per transaction: allowed {}, found {} for block {} and job id {}",
        max_blob_per_txn,
        current_blob_length,
        internal_id.to_string(),
        Uuid::default()
    )
    .to_string();

    match response {
        Ok(_) => return Err(eyre!("This testcase should not have processed the job correctly.")),
        Err(e) => {
            println!("answer : {}", e.to_string());
            println!("expected : {}", expected);
            assert_eq!(e.to_string(), expected);
        }
    }
    state_update_mock.assert();
    let _ = drop_database().await;

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_da_job_process_job_failure_on_pending_block() -> Result<()> {
    dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

    let server = TestConfigBuilder::new().build().await;
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
        when.path("/").body_contains("starknet_getStateUpdate");
        then.status(200).body(serde_json::to_vec(&response).unwrap());
    });

    let response = DaJob
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
            },
        )
        .await;

    let expected = eyre!(
        "Cannot process block {} for job id {} as it's still in pending state",
        internal_id.to_string(),
        Uuid::default()
    )
    .to_string();

    match response {
        Ok(_) => return Err(eyre!("This testcase should not have processed the job correctly.")),
        Err(e) => {
            assert_eq!(e.to_string(), expected);
        }
    }
    state_update_mock.assert();
    Ok(())
}

#[rstest]
#[case(
    "src/tests/jobs/da_job/test_data/state_update/638353.txt",
    "src/tests/jobs/da_job/test_data/nonces/638353.txt",
    "638353"
)]
#[tokio::test]
async fn test_da_job_process_job_success(
    #[case] state_update_file: String,
    #[case] noances_file: String,
    #[case] internal_id: String,
) -> Result<()> {
    dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

    // Mocking DA client calls
    let mut da_client = MockDaClient::new();
    da_client.expect_publish_state_diff().with(always(), always()).returning(|_, _| Ok("Done".to_string()));
    da_client.expect_max_blob_per_txn().with().returning(|| 6);
    da_client.expect_max_bytes_per_blob().with().returning(|| 131072);

    let server = TestConfigBuilder::new().mock_da_client(Box::new(da_client)).build().await;
    let config = config().await;

    let state_update = read_state_update_from_file(state_update_file.as_str()).expect("issue while reading");

    let state_update = serde_json::to_value(&state_update).unwrap();
    let response = json!({ "id": 1,"jsonrpc":"2.0","result": state_update });

    get_nonce_attached(&server, noances_file.as_str());

    let state_update_mock = server.mock(|when, then| {
        when.path("/").body_contains("starknet_getStateUpdate");
        then.status(200).body(serde_json::to_vec(&response).unwrap());
    });

    let response = DaJob
        .process_job(
            config.as_ref(),
            &mut JobItem {
                id: Uuid::default(),
                internal_id: internal_id.to_string(),
                job_type: JobType::DataSubmission,
                status: JobStatus::Created,
                external_id: ExternalId::String(internal_id.to_string().into_boxed_str()),
                metadata: HashMap::default(),
                version: 0,
            },
        )
        .await;

    match response {
        Ok(message) => {
            assert_eq!(message, eyre!("Done").to_string());
        }
        Err(_) => {}
    }

    state_update_mock.assert();
    let _ = drop_database().await;

    Ok(())
}

#[rstest]
#[case(false, 1, 1, "18446744073709551617")]
#[case(false, 1, 0, "18446744073709551616")]
#[case(false, 0, 6, "6")]
#[case(true, 1, 0, "340282366920938463481821351505477763072")]
fn da_word_works(#[case] class_flag: bool, #[case] new_nonce: u64, #[case] num_changes: u64, #[case] expected: String) {
    let new_nonce = if new_nonce > 0 { Some(FieldElement::from(new_nonce)) } else { None };
    let da_word = da_word(class_flag, new_nonce, num_changes);
    let expected = FieldElement::from_dec_str(expected.as_str()).unwrap();
    assert_eq!(da_word, expected);
}

#[rstest]
#[case(
    631861,
    "src/jobs/da_job/test_data/state_update_from_block_631861.txt",
    "src/jobs/da_job/test_data/test_blob_631861.txt",
    "src/jobs/da_job/test_data/nonces_from_block_631861.txt"
)]
#[case(
    638353,
    "src/jobs/da_job/test_data/state_update_from_block_638353.txt",
    "src/jobs/da_job/test_data/test_blob_638353.txt",
    "src/jobs/da_job/test_data/nonces_from_block_638353.txt"
)]
#[case(
    640641,
    "src/jobs/da_job/test_data/state_update_from_block_640641.txt",
    "src/jobs/da_job/test_data/test_blob_640641.txt",
    "src/jobs/da_job/test_data/nonces_from_block_640641.txt"
)]
#[tokio::test]
async fn test_state_update_to_blob_data(
    #[case] block_no: u64,
    #[case] state_update_file_path: &str,
    #[case] file_path: &str,
    #[case] nonce_file_path: &str,
) {
    use crate::jobs::da_job::{convert_to_biguint, state_update_to_blob_data};

    let server = MockServer::start();
    let mut da_client = MockDaClient::new();
    let mut storage_client = MockDataStorage::new();

    // Mocking DA client calls
    da_client.expect_max_blob_per_txn().with().returning(|| 6);
    da_client.expect_max_bytes_per_blob().with().returning(|| 131072);

    // Mocking storage client
    storage_client.expect_put_data().returning(|_, _| Result::Ok(())).times(1);

    let config = init_config(
        Some(format!("http://localhost:{}", server.port())),
        None,
        None,
        Some(da_client),
        None,
        None,
        Some(storage_client),
    )
    .await;

    get_nonce_attached(&server, nonce_file_path);

    let state_update = read_state_update_from_file(state_update_file_path).expect("issue while reading");
    let blob_data = state_update_to_blob_data(block_no, state_update, &config)
        .await
        .expect("issue while converting state update to blob data");

    let blob_data_biguint = convert_to_biguint(blob_data);

    let block_data_state_diffs = serde::parse_state_diffs(blob_data_biguint.as_slice());

    let original_blob_data = serde::parse_file_to_blob_data(file_path);
    // converting the data to it's original format
    let recovered_blob_data = blob::recover(original_blob_data.clone());
    let blob_data_state_diffs = serde::parse_state_diffs(recovered_blob_data.as_slice());

    assert!(block_data_state_diffs.unordered_eq(&blob_data_state_diffs), "value of data json should be identical");
}

#[rstest]
#[case("src/jobs/da_job/test_data/test_blob_631861.txt")]
#[case("src/jobs/da_job/test_data/test_blob_638353.txt")]
#[case("src/jobs/da_job/test_data/test_blob_639404.txt")]
#[case("src/jobs/da_job/test_data/test_blob_640641.txt")]
#[case("src/jobs/da_job/test_data/test_blob_640644.txt")]
#[case("src/jobs/da_job/test_data/test_blob_640646.txt")]
#[case("src/jobs/da_job/test_data/test_blob_640647.txt")]
fn test_fft_transformation(#[case] file_to_check: &str) {
    // parsing the blob hex to the bigUints

    use crate::jobs::da_job::fft_transformation;
    let original_blob_data = serde::parse_file_to_blob_data(file_to_check);
    // converting the data to its original format
    let ifft_blob_data = blob::recover(original_blob_data.clone());
    // applying the fft function again on the original format
    let fft_blob_data = fft_transformation(ifft_blob_data);

    // ideally the data after fft transformation and the data before ifft should be same.
    assert_eq!(fft_blob_data, original_blob_data);
}

#[rstest]
fn test_bincode() {
    let data = vec![vec![1, 2], vec![3, 4]];

    let serialize_data = bincode::serialize(&data).unwrap();
    let deserialize_data: Vec<Vec<u8>> = bincode::deserialize(&serialize_data).unwrap();

    assert_eq!(data, deserialize_data);
}

pub fn read_state_update_from_file(file_path: &str) -> Result<StateUpdate> {
    // let file_path = format!("state_update_block_no_{}.txt", block_no);
    let mut file = File::open(file_path)?;
    let mut json = String::new();
    file.read_to_string(&mut json)?;
    let state_update: StateUpdate = serde_json::from_str(&json)?;
    Ok(state_update)
}

#[derive(Serialize, Deserialize, Debug)]
struct NonceAddress {
    nonce: String,
    address: String,
}

pub fn get_nonce_attached(server: &MockServer, file_path: &str) {
    // Read the file
    let file_content = fs::read_to_string(file_path).expect("Unable to read file");

    // Parse the JSON content into a vector of NonceAddress
    let nonce_addresses: Vec<NonceAddress> = serde_json::from_str(&file_content).expect("JSON was not well-formatted");

    // Set up mocks for each entry
    for entry in nonce_addresses {
        let address = entry.address.clone();
        let nonce = entry.nonce.clone();
        let response = json!({ "id": 1,"jsonrpc":"2.0","result": nonce });
        let field_element =
            FieldElement::from_dec_str(&address).expect("issue while converting the hex to field").to_bytes_be();
        let hex_field_element = vec_u8_to_hex_string(&field_element);

        server.mock(|when, then| {
            when.path("/").body_contains("starknet_getNonce").body_contains(hex_field_element);
            then.status(200).body(serde_json::to_vec(&response).unwrap());
        });
    }
}

fn vec_u8_to_hex_string(data: &[u8]) -> String {
    let hex_chars: Vec<String> = data.iter().map(|byte| format!("{:02x}", byte)).collect();

    let mut new_hex_chars = hex_chars.join("");
    new_hex_chars = new_hex_chars.trim_start_matches('0').to_string();

    // Handle the case where the trimmed string is empty (e.g., data was all zeros)
    if new_hex_chars.is_empty() {
        "0x0".to_string()
    } else {
        format!("0x{}", new_hex_chars)
    }
}
