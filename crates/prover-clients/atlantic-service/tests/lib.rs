use atlantic_service::AtlanticProverService;
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
use httpmock::MockServer;
use prover_client_interface::{ProverClient, Task};
use utils::settings::env::EnvSettingsProvider;

use crate::constants::CAIRO_PIE_PATH;

mod constants;

#[tokio::test]
async fn atlantic_client_submit_task_calls_correct_endpoint() {
    let _ = env_logger::try_init();
    color_eyre::install().expect("Unable to install color_eyre");
    dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

    // Start a mock server
    let mock_server = MockServer::start();

    // Create a mock for the submit endpoint
    let submit_mock = mock_server.mock(|when, then| {
        when.method("POST").path("/l1/atlantic-query/proof-generation-verification");
        then.status(200).header("content-type", "application/json").json_body(serde_json::json!({
            "sharpQueryId": "mock_query_id_123"
        }));
    });

    // Configure the service to use mock server
    let settings = EnvSettingsProvider {};
    let atlantic_service = AtlanticProverService::with_test_settings(&settings, mock_server.port());

    let cairo_pie_path = env!("CARGO_MANIFEST_DIR").to_string() + CAIRO_PIE_PATH;
    let cairo_pie = CairoPie::read_zip_file(cairo_pie_path.as_ref()).expect("failed to read cairo pie zip");

    let task_result = atlantic_service.submit_task(Task::CairoPie(Box::new(cairo_pie)), LayoutName::dynamic).await;

    assert!(task_result.is_ok());
    submit_mock.assert();
}
