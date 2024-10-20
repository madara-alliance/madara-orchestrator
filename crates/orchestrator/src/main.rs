use axum::Router;
use dotenvy::dotenv;
use orchestrator::config::init_config;
use orchestrator::queue::init_consumers;
use orchestrator::routes::job_routes::job_routes;
use orchestrator::routes::routes::{app_router, handler_404};
use orchestrator::telemetry::{setup_analytics, shutdown_analytics};
use utils::env_utils::get_env_var_or_default;

/// Start the server
#[tokio::main]
// not sure why clippy gives this error on the latest rust
// version but have added it for now
#[allow(clippy::needless_return)]
async fn main() {
    dotenv().ok();
    // Analytics Setup
    let meter_provider = setup_analytics();
    tracing::info!(service = "orchestrator", "Starting orchestrator service");

    color_eyre::install().expect("Unable to install color_eyre");

    // initial config setup
    let config = init_config().await.expect("Config instantiation failed");
    tracing::debug!(service = "orchestrator", "Configuration initialized");

    let host = get_env_var_or_default("HOST", "127.0.0.1");
    let port = get_env_var_or_default("PORT", "3000").parse::<u16>().expect("PORT must be a u16");
    let address = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(address.clone()).await.expect("Failed to get listener");

    let job_routes = job_routes(config.clone());
    let app_routes = app_router();

    let app = Router::new().merge(app_routes).merge(job_routes).fallback(handler_404);

    tracing::debug!(service = "orchestrator", "Application router initialized");

    // init consumer
    match init_consumers(config).await {
        Ok(_) => tracing::info!(service = "orchestrator", "Consumers initialized successfully"),
        Err(e) => {
            tracing::error!(service = "orchestrator", error = %e, "Failed to initialize consumers");
            panic!("Failed to init consumers: {}", e);
        }
    }

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(service = "orchestrator", error = %e, "Server failed to start");
        panic!("Failed to start axum server: {}", e);
    }

    // Analytics Shutdown
    shutdown_analytics(meter_provider);
    tracing::info!(service = "orchestrator", "Orchestrator service shutting down");
}
