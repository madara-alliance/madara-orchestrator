use cairo_vm::types::layout_name::LayoutName;
use clap::Parser as _;
use color_eyre::eyre::Ok as ColorOk;
use dotenvy::dotenv;
use orchestrator::cli::{Cli, Commands, RunCmd, SetupCmd};
use orchestrator::config::init_config;
use orchestrator::jobs::snos_job::SnosError;
use orchestrator::jobs::JobError;
use orchestrator::queue::init_consumers;
use orchestrator::routes::setup_server;
use orchestrator::setup::setup_cloud;
use orchestrator::telemetry::{setup_analytics, shutdown_analytics};
use prove_block::prove_block;
use utils::env_utils::get_env_var_or_default;

/// Start the server
#[tokio::main]
// not sure why clippy gives this error on the latest rust
// version but have added it for now
#[allow(clippy::needless_return)]
async fn main() {
    dotenv().ok();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Run { run_command } => {
            run_orchestrator(run_command).await.expect("Failed to run orchestrator");
        }
        Commands::Setup { setup_command } => {
            setup_orchestrator(setup_command).await.expect("Failed to setup orchestrator");
        }
        Commands::Test {} => {
            test_prove().await.expect("Failed to run test_prove");
        }
    }
}

async fn run_orchestrator(run_cmd: &RunCmd) -> color_eyre::Result<()> {
    // Analytics Setup
    let instrumentation_params = run_cmd.validate_instrumentation_params().expect("Invalid instrumentation params");
    let meter_provider = setup_analytics(&instrumentation_params);
    tracing::info!(service = "orchestrator", "Starting orchestrator service");

    color_eyre::install().expect("Unable to install color_eyre");

    // initial config setup
    let config = init_config(run_cmd).await.expect("Config instantiation failed");
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

    Ok(())
}

async fn setup_orchestrator(setup_cmd: &SetupCmd) -> color_eyre::Result<()> {
    setup_cloud(setup_cmd).await.expect("Failed to setup cloud");
    Ok(())
}

pub const COMPILED_OS: &[u8] = include_bytes!("../../../build/os_latest.json");

async fn test_prove() -> color_eyre::Result<()> {
    dotenv().ok();
    println!("Running prove block test");
    let endpoint = get_env_var_or_default("MADARA_ORCHESTRATOR_RPC_FOR_SNOS", "http://localhost:9545");
    let blocks_to_run_on = get_env_var_or_default("MADARA_ORCHESTRATOR_BLOCKS_TO_RUN_ON", "48,49")
        .split(',')
        .map(|x| x.parse::<u64>().expect("Failed to parse block number"))
        .collect::<Vec<u64>>();

    println!("Running on blocks: {:?}", blocks_to_run_on);
    println!("Using endpoint: {}", endpoint);

    for block_number in blocks_to_run_on {
        println!("Running on block: {}", block_number);
        let result = process_job_helper(block_number, endpoint.as_str()).await;
        assert!(result.is_ok());
        println!("Finished block: {}", block_number);
    }

    ColorOk(())
}

async fn process_job_helper(block_number: u64, snos_url: &str) -> color_eyre::Result<String> {
    let (cairo_pie, snos_output) = prove_block(COMPILED_OS, block_number, snos_url, LayoutName::all_cairo, false)
        .await
        .map_err(|e| SnosError::SnosExecutionError { internal_id: block_number.to_string(), message: e.to_string() })?;
    cairo_pie.run_validity_checks().expect("Valid SNOS PIE");
    println!("SNOS Output Came for block: {}", block_number);
    ColorOk(block_number.to_string())
}
