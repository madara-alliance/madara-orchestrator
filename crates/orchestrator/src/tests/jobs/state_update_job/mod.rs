use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use assert_matches::assert_matches;
use bytes::Bytes;
use httpmock::prelude::*;
use mockall::predicate::{always, eq};
use rstest::*;
use settlement_client_interface::MockSettlementClient;

use color_eyre::eyre::eyre;
use utils::env_utils::get_env_var_or_panic;

use crate::config::config;
use crate::constants::{BLOB_DATA_FILE_NAME, SNOS_OUTPUT_FILE_NAME};
use crate::data_storage::MockDataStorage;
use crate::jobs::constants::JOB_METADATA_STATE_UPDATE_LAST_FAILED_BLOCK_NO;
use crate::jobs::constants::{
    JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY, JOB_METADATA_STATE_UPDATE_FETCH_FROM_TESTS,
    JOB_PROCESS_ATTEMPT_METADATA_KEY,
};
use crate::jobs::state_update_job::utils::hex_string_to_u8_vec;
use crate::jobs::state_update_job::{StateUpdateError, StateUpdateJob};
use crate::jobs::types::{JobStatus, JobType};
use crate::jobs::{Job, JobError};
use crate::tests::common::{default_job_item, get_storage_client};
use crate::tests::config::TestConfigBuilder;
use lazy_static::lazy_static;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use url::Url;

lazy_static! {
    pub static ref CURRENT_PATH: PathBuf = std::env::current_dir().unwrap();
}

pub const X_0_FILE_NAME: &str = "x_0.txt";

// ================= Exhaustive tests (with minimum mock) =================

#[rstest]
#[tokio::test]
async fn test_process_job_attempt_not_present_fails() {
    TestConfigBuilder::new().build().await;

    let mut job = default_job_item();
    let config = config().await;
    let state_update_job = StateUpdateJob {};
    let res = state_update_job.process_job(&config, &mut job).await.unwrap_err();
    assert_eq!(res, JobError::StateUpdateJobError(StateUpdateError::AttemptNumberNotFound));
}

#[rstest]
#[case(None, String::from("651053,651054,651055"), 0)]
#[case(Some(651054), String::from("651053,651054,651055"), 1)]
#[tokio::test]
async fn test_process_job_works(
    #[case] failed_block_number: Option<u64>,
    #[case] blocks_to_process: String,
    #[case] processing_start_index: u8,
) {
    // Will be used by storage client which we call while storing the data.

    use num::ToPrimitive;

    use crate::jobs::state_update_job::utils::fetch_blob_data_for_block;
    dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

    // Mocking the settlement client.
    let mut settlement_client = MockSettlementClient::new();

    let block_numbers: Vec<u64> = parse_block_numbers(&blocks_to_process).unwrap();

    // This must be the last block number and should be returned as an output from the process job.
    let last_block_number = block_numbers[block_numbers.len() - 1];

    // Storing `blob_data` and `snos_output` in storage client
    store_data_in_storage_client_for_s3(block_numbers.clone()).await;

    // Building a temp config that will be used by `fetch_blob_data_for_block` and `fetch_snos_for_block`
    // functions while fetching the blob data from storage client.
    TestConfigBuilder::new().build().await;

    // test_process_job_works uses nonce just to write expect_update_state_with_blobs for a mocked settlement client,
    // which means that nonce ideally is never checked against, hence supplying any `u64` `nonce` works.
    let nonce: u64 = 3;
    settlement_client.expect_get_nonce().with().returning(move || Ok(nonce));

    // Adding expectations for each block number to be called by settlement client.
    for block in block_numbers.iter().skip(processing_start_index as usize) {
        let blob_data = fetch_blob_data_for_block(block.to_u64().unwrap()).await.unwrap();
        settlement_client
            .expect_update_state_with_blobs()
            .with(eq(vec![]), eq(blob_data), always())
            .times(1)
            .returning(|_, _, _| Ok("0xbeef".to_string()));
    }
    settlement_client.expect_get_last_settled_block().with().returning(move || Ok(651052));

    // Building new config with mocked settlement client
    TestConfigBuilder::new().mock_settlement_client(Box::new(settlement_client)).build().await;

    // setting last failed block number as 651053.
    // setting blocks yet to process as 651054 and 651055.
    // here total blocks to process will be 3.
    let mut metadata: HashMap<String, String> = HashMap::new();
    metadata.insert(JOB_PROCESS_ATTEMPT_METADATA_KEY.to_string(), "0".to_string());
    if let Some(block_number) = failed_block_number {
        metadata.insert(JOB_METADATA_STATE_UPDATE_LAST_FAILED_BLOCK_NO.to_string(), block_number.to_string());
    }
    metadata.insert(JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY.to_string(), blocks_to_process);

    // creating a `JobItem`
    let mut job = default_job_item();
    job.job_type = JobType::StateTransition;
    job.metadata = metadata;

    let config = config().await;
    let state_update_job = StateUpdateJob {};
    let res = state_update_job.process_job(&config, &mut job).await.unwrap();
    assert_eq!(res, last_block_number.to_string());
}

// ==================== Mock Tests (Unit tests) ===========================

#[rstest]
#[tokio::test]
async fn create_job_works() {
    TestConfigBuilder::new().build().await;

    let config = config().await;

    let job = StateUpdateJob.create_job(&config, String::from("0"), HashMap::default()).await;
    assert!(job.is_ok());

    let job = job.unwrap();
    let job_type = job.job_type;

    assert_eq!(job_type, JobType::StateTransition, "job_type should be StateTransition");
    assert!(!(job.id.is_nil()), "id should not be nil");
    assert_eq!(job.status, JobStatus::Created, "status should be Created");
    assert_eq!(job.version, 0_i32, "version should be 0");
    assert_eq!(job.external_id.unwrap_string().unwrap(), String::new(), "external_id should be empty string");
}

#[rstest]
#[tokio::test]
async fn process_job_works() {
    let mut settlement_client = MockSettlementClient::new();
    let mut storage_client = MockDataStorage::new();

    // Mock the latest block settled
    settlement_client.expect_get_last_settled_block().returning(|| Ok(651052_u64));

    // TODO: have tests for update_state_calldata, only kzg for now
    let block_numbers = ["651053", "651054", "651055", "651056"];
    for block_no in block_numbers {
        let program_output: Vec<[u8; 32]> = vec![];
        let state_diff: Vec<Vec<u8>> = load_state_diff_file(block_no.parse::<u64>().unwrap()).await;

        let snos_output_key = block_no.to_owned() + "/" + SNOS_OUTPUT_FILE_NAME;
        let snos_output_data = fs::read_to_string(
            CURRENT_PATH
                .join(format!("src/tests/jobs/state_update_job/test_data/{}/{}", block_no, SNOS_OUTPUT_FILE_NAME)),
        )
        .expect("Failed to read the snos output data json file");
        storage_client
            .expect_get_data()
            .with(eq(snos_output_key))
            .returning(move |_| Ok(Bytes::from(snos_output_data.clone())));

        let blob_data_key = block_no.to_owned() + "/" + BLOB_DATA_FILE_NAME;
        let blob_data = fs::read_to_string(
            CURRENT_PATH
                .join(format!("src/tests/jobs/state_update_job/test_data/{}/{}", block_no, BLOB_DATA_FILE_NAME)),
        )
        .expect("Failed to read the blob data txt file");
        let blob_data_vec = vec![hex_string_to_u8_vec(&blob_data).unwrap()];
        let blob_serialized = bincode::serialize(&blob_data_vec).unwrap();
        storage_client
            .expect_get_data()
            .with(eq(blob_data_key))
            .returning(move |_| Ok(Bytes::from(blob_serialized.clone())));

        let x_0_key = block_no.to_owned() + "/" + X_0_FILE_NAME;
        let x_0 = fs::read_to_string(
            CURRENT_PATH.join(format!("src/tests/jobs/state_update_job/test_data/{}/{}", block_no, X_0_FILE_NAME)),
        )
        .expect("Failed to read the blob data txt file");
        storage_client.expect_get_data().with(eq(x_0_key)).returning(move |_| Ok(Bytes::from(x_0.clone())));

        // let nonce = settlement_client.get_nonce().await.expect("Unable to fetch nonce for settlement client.");
        settlement_client.expect_get_nonce().returning(|| Ok(1));

        settlement_client
            .expect_update_state_with_blobs()
            // TODO: vec![] is program_output
            .with(eq(program_output), eq(state_diff), always())
            .returning(|_, _, _| Ok(String::from("0x5d17fac98d9454030426606019364f6e68d915b91f6210ef1e2628cd6987442")));
    }

    TestConfigBuilder::new()
        .mock_settlement_client(Box::new(settlement_client))
        .mock_storage_client(Box::new(storage_client))
        .build()
        .await;

    let mut metadata: HashMap<String, String> = HashMap::new();
    metadata.insert(String::from(JOB_METADATA_STATE_UPDATE_FETCH_FROM_TESTS), String::from("TRUE"));
    metadata.insert(String::from(JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY), block_numbers.join(","));
    metadata.insert(String::from(JOB_PROCESS_ATTEMPT_METADATA_KEY), String::from("0"));

    let mut job =
        StateUpdateJob.create_job(config().await.as_ref(), String::from("internal_id"), metadata).await.unwrap();
    assert_eq!(StateUpdateJob.process_job(config().await.as_ref(), &mut job).await.unwrap(), "651056".to_string())
}

#[rstest]
#[case(String::from("651052, 651054, 651051, 651056"), "numbers aren't sorted in increasing order")]
#[case(String::from("651052, 651052, 651052, 651052"), "Duplicated block numbers")]
#[case(String::from("a, 651054, b, 651056"), "settle list is not correctly formatted")]
#[case(String::from("651052, 651052, 651053, 651053"), "Duplicated block numbers")]
#[case(String::from(""), "settle list is not correctly formatted")]
#[tokio::test]
async fn process_job_invalid_inputs_errors(#[case] block_numbers_to_settle: String, #[case] expected_error: &str) {
    let server = MockServer::start();
    let settlement_client = MockSettlementClient::new();

    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse(format!("http://localhost:{}", server.port()).as_str()).expect("Failed to parse URL"),
    ));

    TestConfigBuilder::new()
        .mock_starknet_client(Arc::new(provider))
        .mock_settlement_client(Box::new(settlement_client))
        .build()
        .await;
    let config = config().await;

    let mut metadata: HashMap<String, String> = HashMap::new();
    metadata.insert(String::from(JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY), block_numbers_to_settle);
    metadata.insert(String::from(JOB_PROCESS_ATTEMPT_METADATA_KEY), String::from("0"));

    let mut job = StateUpdateJob.create_job(&config, String::from("internal_id"), metadata).await.unwrap();
    let status = StateUpdateJob.process_job(&config, &mut job).await;
    assert!(status.is_err());

    if let Err(error) = status {
        let error_message = format!("{}", error);
        assert!(
            error_message.contains(expected_error),
            "Error message did not contain expected substring: {}",
            expected_error
        );
    }
}

#[rstest]
#[tokio::test]
async fn process_job_invalid_input_gap_panics() {
    let server = MockServer::start();
    let mut settlement_client = MockSettlementClient::new();

    settlement_client.expect_get_last_settled_block().returning(|| Ok(4_u64));

    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse(format!("http://localhost:{}", server.port()).as_str()).expect("Failed to parse URL"),
    ));

    TestConfigBuilder::new()
        .mock_starknet_client(Arc::new(provider))
        .mock_settlement_client(Box::new(settlement_client))
        .build()
        .await;

    let mut metadata: HashMap<String, String> = HashMap::new();
    metadata.insert(String::from(JOB_METADATA_STATE_UPDATE_BLOCKS_TO_SETTLE_KEY), String::from("6, 7, 8"));
    metadata.insert(String::from(JOB_PROCESS_ATTEMPT_METADATA_KEY), String::from("0"));

    let mut job =
        StateUpdateJob.create_job(config().await.as_ref(), String::from("internal_id"), metadata).await.unwrap();
    let response = StateUpdateJob.process_job(config().await.as_ref(), &mut job).await;

    assert_matches!(response,
        Err(e) => {
            let err = StateUpdateError::GapBetweenFirstAndLastBlock;
            let expected_error = JobError::StateUpdateJobError(err);
            assert_eq!(e.to_string(), expected_error.to_string());
        }
    );
}

// ==================== Utility functions ===========================

async fn load_state_diff_file(block_no: u64) -> Vec<Vec<u8>> {
    let mut state_diff_vec: Vec<Vec<u8>> = Vec::new();
    let file_path = format!("src/tests/jobs/state_update_job/test_data/{}/{}", block_no, BLOB_DATA_FILE_NAME);
    let file_data = fs::read_to_string(file_path).expect("Unable to read kzg_proof.txt").replace("0x", "");
    let blob_data = hex_string_to_u8_vec(&file_data).unwrap();
    state_diff_vec.push(blob_data);
    state_diff_vec
}

async fn store_data_in_storage_client_for_s3(block_numbers: Vec<u64>) {
    let storage_client = get_storage_client().await;
    storage_client.build_test_bucket(&get_env_var_or_panic("AWS_S3_BUCKET_NAME")).await.unwrap();

    for block in block_numbers {
        // Getting the blob data from file.
        let blob_data_key = block.to_owned().to_string() + "/" + BLOB_DATA_FILE_NAME;
        let blob_data = fs::read_to_string(
            CURRENT_PATH.join(format!("src/tests/jobs/state_update_job/test_data/{}/{}", block, BLOB_DATA_FILE_NAME)),
        )
        .unwrap();
        let blob_data_vec = vec![hex_string_to_u8_vec(&blob_data).unwrap()];
        let blob_serialized = bincode::serialize(&blob_data_vec).unwrap();

        // Getting the snos data from file.
        let snos_output_key = block.to_owned().to_string() + "/" + SNOS_OUTPUT_FILE_NAME;
        let snos_output_data = fs::read_to_string(
            CURRENT_PATH.join(format!("src/tests/jobs/state_update_job/test_data/{}/{}", block, SNOS_OUTPUT_FILE_NAME)),
        )
        .unwrap();

        storage_client.put_data(Bytes::from(snos_output_data), &snos_output_key).await.unwrap();
        storage_client.put_data(Bytes::from(blob_serialized), &blob_data_key).await.unwrap();
    }
}

fn parse_block_numbers(blocks_to_settle: &str) -> color_eyre::Result<Vec<u64>> {
    let sanitized_blocks = blocks_to_settle.replace(' ', "");
    let block_numbers: Vec<u64> = sanitized_blocks
        .split(',')
        .map(|block_no| block_no.parse::<u64>())
        .collect::<color_eyre::Result<Vec<u64>, _>>()
        .map_err(|e| eyre!("Block numbers to settle list is not correctly formatted: {e}"))?;
    Ok(block_numbers)
}
