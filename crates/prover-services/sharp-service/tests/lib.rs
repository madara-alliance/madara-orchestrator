use cairo_vm::vm::runners::cairo_pie::CairoPie;
use httpmock::MockServer;
use prover_client_interface::{ProverClient, ProverClientError, Task};
use rstest::rstest;
use serde_json::json;
use sharp_service::SharpProverService;
use std::path::PathBuf;
use utils::settings::default::DefaultSettingsProvider;

#[rstest]
#[tokio::test]
async fn test_prover_client_submit_task_works() {
    let mut server = MockServer::connect("http://127.0.0.1:8080");
    let sharp_service = SharpProverService::with_settings(&DefaultSettingsProvider {});
    let cairo_pie_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "artifacts", "238996-SN.zip"].iter().collect();
    let cairo_pie = CairoPie::read_zip_file(&cairo_pie_path).unwrap();
    let encoded_pie =
        snos::sharp::pie::encode_pie_mem(cairo_pie.clone()).map_err(ProverClientError::PieEncoding).unwrap();

    let task_id = sharp_service.submit_task(Task::CairoPie(cairo_pie)).await.unwrap();

    // mocking sharp client call to add job
    let json_body = json!(
        {
            "action" : "add_job",
            "request" : { "cairo_pie": encoded_pie }
        }
    );
    let sharp_add_job_call = server.mock(|when, then| {
        when.path("/add-job").body_contains(serde_json::to_string(&json_body).unwrap());
        then.status(200).body(serde_json::to_vec("{\"code\": \"JOB_RECEIVED_SUCCESSFULLY\"}").unwrap());
    });
    sharp_add_job_call.assert();
}
