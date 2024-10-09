use atlantic_service::AtlanticProverService;
use prover_client_interface::{ProverClient, Task};
use rstest::rstest;
use utils::settings::env::EnvSettingsProvider;

use crate::constants::CAIRO_PIE_PATH;

mod constants;

#[tokio::test]
#[rstest]
async fn atlantic_client_submit_task_works() {
    let _ = env_logger::try_init();
    dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");
    let atlantic_service = AtlanticProverService::new_with_settings(&EnvSettingsProvider {});

    let cairo_pie_path = env!("CARGO_MANIFEST_DIR").to_string() + CAIRO_PIE_PATH;
    assert!(atlantic_service.submit_task(Task::CairoPieFilePath(cairo_pie_path)).await.is_ok());
}
