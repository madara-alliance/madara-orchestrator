use atlantic_service::AtlanticProverService;
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
use prover_client_interface::{ProverClient, Task};
use rstest::rstest;
use utils::settings::env::EnvSettingsProvider;

use crate::constants::CAIRO_PIE_PATH;

mod constants;

#[tokio::test]
#[rstest]
async fn atlantic_client_submit_task_works() {
    let _ = env_logger::try_init();
    color_eyre::install().expect("Unable to install color_eyre");
    dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");
    let atlantic_service = AtlanticProverService::new_with_settings(&EnvSettingsProvider {});

    println!("Atlantic service: ");

    let cairo_pie_path = env!("CARGO_MANIFEST_DIR").to_string() + CAIRO_PIE_PATH;
    println!("Cairo pie path: {}", cairo_pie_path);

    let cairo_pie = CairoPie::read_zip_file(cairo_pie_path.as_ref()).expect("failed to read cairo pie zip");
    println!("cairo pie read successfully");
    let task_result = atlantic_service.submit_task(Task::CairoPie(Box::new(cairo_pie)), LayoutName::dynamic).await;
    log::info!("Task result from atlantic service: {:?}", task_result);
    assert!(task_result.is_ok());

    let query_id = task_result.expect("Failed to submit task");
    // let query_id = "01JA7X1R3HH2BXJ6B7NC814ERP";
    log::info!("Task submitted with query id: {:?}", query_id);
    let status = atlantic_service
        .atlantic_client
        .get_job_status(query_id.as_ref())
        .await
        .expect("Failed to get status from atlantic");
    log::info!("Got status from atlantic {:?}", status);
}
