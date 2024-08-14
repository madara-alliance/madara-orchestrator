use cairo_vm::vm::runners::cairo_pie::CairoPie;
use httpmock::MockServer;
use prover_client_interface::{ProverClient, ProverClientError, Task};
use rstest::rstest;
use sharp_service::SharpProverService;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread::{JoinHandle, spawn};
use httpmock::standalone::start_standalone_server;
use lazy_static::lazy_static;
use tokio::task::LocalSet;
use utils::settings::default::DefaultSettingsProvider;

// To start a standalone server to be used with sharp client mocking
lazy_static! {
    static ref STANDALONE_SERVER: Mutex<JoinHandle<Result<(), String>>> = Mutex::new(spawn(|| {
        let srv =
            start_standalone_server(5000, false, None, false, usize::MAX, std::future::pending());
        let mut runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        LocalSet::new().block_on(&mut runtime, srv)
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
    println!("mock server connected...");
    let sharp_service = SharpProverService::with_settings(&DefaultSettingsProvider {});
    println!("sharp service initiated...");
    let cairo_pie_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "artifacts", "238996-SN.zip"].iter().collect();
    let cairo_pie = CairoPie::read_zip_file(&cairo_pie_path).unwrap();
    println!("PIE reading completed...");
    let encoded_pie =
        snos::sharp::pie::encode_pie_mem(cairo_pie.clone()).map_err(ProverClientError::PieEncoding).unwrap();
    println!("encoded PIE : {:?}", encoded_pie.len());

    let sharp_add_job_call = server.mock(|when, then| {
        when.path("/add-job").query_param("encoded_pie", encoded_pie);
        then.status(200).body(serde_json::to_vec("{\"code\": \"JOB_RECEIVED_SUCCESSFULLY\"}").unwrap());
    });

    let task_id = sharp_service.submit_task(Task::CairoPie(cairo_pie)).await.unwrap();
    println!("task ID : {:?}", task_id);
    assert_ne!(task_id.len(), 0, "Task ID should be there.");
    sharp_add_job_call.assert();
}
