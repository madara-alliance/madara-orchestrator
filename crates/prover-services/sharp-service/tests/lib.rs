use alloy::primitives::B256;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
use httpmock::standalone::start_standalone_server;
use httpmock::MockServer;
use lazy_static::lazy_static;
use prover_client_interface::{ProverClient, Task, TaskId, TaskStatus};
use rstest::rstest;
use serde_json::json;
use sharp_service::{split_task_id, SharpProverService};
use snos::sharp::CairoJobStatus;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;
use std::thread::{spawn, JoinHandle};
use tokio::task::LocalSet;
use utils::env_utils::get_env_var_or_panic;
use utils::settings::default::DefaultSettingsProvider;

// To start a standalone server to be used with sharp client mocking
lazy_static! {
    static ref STANDALONE_SERVER: Mutex<JoinHandle<Result<(), String>>> = Mutex::new(spawn(|| {
        let srv = start_standalone_server(5000, false, None, false, usize::MAX, std::future::pending());
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        LocalSet::new().block_on(&runtime, srv)
    }));
}

/// To simulate a standalone server wherever needed
pub fn simulate_standalone_server() {
    let _unused = STANDALONE_SERVER.lock().unwrap_or_else(|e| e.into_inner());
}

#[rstest]
#[tokio::test]
async fn prover_client_submit_task_works() {
    dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");
    simulate_standalone_server();

    let server = MockServer::connect("localhost:5000");
    let sharp_service = SharpProverService::with_settings(&DefaultSettingsProvider {});
    let cairo_pie_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "artifacts", "238996-SN.zip"].iter().collect();
    let cairo_pie = CairoPie::read_zip_file(&cairo_pie_path).unwrap();

    let sharp_response = json!(
            {
                "code" : "JOB_RECEIVED_SUCCESSFULLY"
            }
    );
    let customer_id = get_env_var_or_panic("SHARP_CUSTOMER_ID");
    let sharp_add_job_call = server.mock(|when, then| {
        when.path_contains("/add_job").query_param("customer_id", customer_id.as_str());
        then.status(200).body(serde_json::to_vec(&sharp_response).unwrap());
    });

    let task_id = sharp_service.submit_task(Task::CairoPie(cairo_pie)).await.unwrap();
    println!("TASK_ID : {:?}", task_id);
    let (_, fact) = split_task_id(&task_id).unwrap();

    // Comparing the calculated fact with on chain verified fact.
    assert_eq!(fact, B256::from_str("0xec8fa9cdfe069ed59b8f17aeecfd95c6abd616379269d2fa16a80955b6e0f068").unwrap());

    sharp_add_job_call.assert();
}

#[rstest]
#[case(CairoJobStatus::FAILED)]
#[case(CairoJobStatus::INVALID)]
#[case(CairoJobStatus::UNKNOWN)]
#[case(CairoJobStatus::IN_PROGRESS)]
#[case(CairoJobStatus::NOT_CREATED)]
#[case(CairoJobStatus::PROCESSED)]
#[case(CairoJobStatus::ONCHAIN)]
#[tokio::test]
async fn prover_client_get_task_status_works(#[case] cairo_job_status: CairoJobStatus) {
    dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");
    simulate_standalone_server();

    let server = MockServer::connect("localhost:5000");
    let sharp_service = SharpProverService::with_settings(&DefaultSettingsProvider {});
    let customer_id = get_env_var_or_panic("SHARP_CUSTOMER_ID");

    let sharp_add_job_call = server.mock(|when, then| {
        when.path_contains("/get_status").query_param("customer_id", customer_id.as_str());
        then.status(200).body(serde_json::to_vec(&get_task_status_sharp_response(&cairo_job_status)).unwrap());
    });

    let task_status = sharp_service
        .get_task_status(&TaskId::from(
            "c31381bf-4739-4667-b5b8-b08af1c6b1c7:0x924cf8d0b955a889fd254b355bb7b29aa9582a370f26943acbe85b2c1a0b201b",
        ))
        .await
        .unwrap();
    assert_eq!(task_status, get_task_status_expectation(&cairo_job_status), "Cairo Job Status assertion failed");

    sharp_add_job_call.assert();
}

fn get_task_status_expectation(cairo_job_status: &CairoJobStatus) -> TaskStatus {
    match cairo_job_status {
        CairoJobStatus::FAILED => TaskStatus::Failed("Sharp task failed".to_string()),
        CairoJobStatus::INVALID => TaskStatus::Failed("Task is invalid: INVALID_CAIRO_PIE_FILE_FORMAT".to_string()),
        CairoJobStatus::UNKNOWN => TaskStatus::Failed("".to_string()),
        CairoJobStatus::IN_PROGRESS | CairoJobStatus::NOT_CREATED | CairoJobStatus::PROCESSED => TaskStatus::Processing,
        CairoJobStatus::ONCHAIN => TaskStatus::Failed(
            "Fact 924cf8d0b955a889fd254b355bb7b29aa9582a370f26943acbe85b2c1a0b201b is not valid or not registered"
                .to_string(),
        ),
    }
}

fn get_task_status_sharp_response(cairo_job_status: &CairoJobStatus) -> serde_json::Value {
    match cairo_job_status {
        CairoJobStatus::FAILED => json!(
            {
                "status" : "FAILED",
                "error_log" : "Sharp task failed"
            }
        ),
        CairoJobStatus::INVALID => json!(
            {
                "status": "INVALID",
                "invalid_reason": "INVALID_CAIRO_PIE_FILE_FORMAT",
                "error_log": "The Cairo PIE file has a wrong format. Deserialization ended with exception: Invalid prefix for zip file.."}
        ),
        CairoJobStatus::UNKNOWN => json!(
            {
                "status" : "FAILED"
            }
        ),
        CairoJobStatus::IN_PROGRESS => json!(
            {
                "status": "IN_PROGRESS",
                "validation_done": false
            }
        ),
        CairoJobStatus::NOT_CREATED => json!(
            {
                "status": "NOT_CREATED",
                "validation_done": false
            }
        ),
        CairoJobStatus::PROCESSED => json!(
            {
                "status": "PROCESSED",
                "validation_done": false
            }
        ),
        CairoJobStatus::ONCHAIN => json!(
            {
                "status": "ONCHAIN",
                "validation_done": true
            }
        ),
    }
}
