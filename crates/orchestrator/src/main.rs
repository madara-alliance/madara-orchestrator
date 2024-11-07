use clap::Parser as _;
use dotenvy::dotenv;
use orchestrator::config::init_config;
use orchestrator::queue::init_consumers;
use orchestrator::routes::setup_server;
use orchestrator::telemetry::{setup_analytics, shutdown_analytics};
use utils::cli::RunCmd;

/// Start the server
#[tokio::main]
// not sure why clippy gives this error on the latest rust
// version but have added it for now
#[allow(clippy::needless_return)]
async fn main() {
    dotenv().ok();
    // TODO: could this be an ARC ?
    let run_cmd: RunCmd = RunCmd::parse();

    // print the run cmd
    println!("{:?}", run_cmd);

    // Analytics Setup
    let instrumentation_params = run_cmd.validate_instrumentation_params().expect("Invalid instrumentation params");
    let meter_provider = setup_analytics(&instrumentation_params);
    tracing::info!(service = "orchestrator", "Starting orchestrator service");

    color_eyre::install().expect("Unable to install color_eyre");

    // initial config setup
    let config = init_config(&run_cmd).await.expect("Config instantiation failed");
    tracing::debug!(service = "orchestrator", "Configuration initialized");

    // initialize the server
    let _ = setup_server(config.clone()).await;

    tracing::debug!(service = "orchestrator", "Application router initialized");

    // init consumer
    match init_consumers(config).await {
        Ok(_) => tracing::info!(service = "orchestrator", "Consumers initialized successfully"),
        Err(e) => {
            tracing::error!(service = "orchestrator", error = %e, "Failed to initialize consumers");
            panic!("Failed to init consumers: {}", e);
        }
    }

    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");

    // Analytics Shutdown
    shutdown_analytics(meter_provider, &instrumentation_params);
    tracing::info!(service = "orchestrator", "Orchestrator service shutting down");
}
