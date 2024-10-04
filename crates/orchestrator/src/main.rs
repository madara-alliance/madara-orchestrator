use dotenvy::dotenv;
use opentelemetry::global;
use orchestrator::queue::init_consumers;
use orchestrator::routes::app_router;
use orchestrator::telemetry::{self, TRACING_LEVEL};
use orchestrator::{config::init_config, telemetry::OTEL_COLLECTOR_ENDPOINT};
use std::str::FromStr;
use tracing::Level;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use utils::env_utils::get_env_var_or_default;

/// Start the server
#[tokio::main]
async fn main() {
    dotenv().ok();

    let otel_enabled = !OTEL_COLLECTOR_ENDPOINT.to_string().is_empty();
    let tracing_level =
        Level::from_str(TRACING_LEVEL.as_str()).expect("Could not obtain tracing level from environment variable.");

    if otel_enabled {
        telemetry::init_analytics();
        let tracer = telemetry::global_tracer().clone();
        tracing_subscriber::registry()
            .with(tracing_subscriber::filter::LevelFilter::from_level(tracing_level))
            .with(tracing_subscriber::fmt::layer())
            .with(OpenTelemetryLayer::new(tracer))
            .init();
    }

    // initial config setup
    let config = init_config().await;

    let host = get_env_var_or_default("HOST", "127.0.0.1");
    let port = get_env_var_or_default("PORT", "3000").parse::<u16>().expect("PORT must be a u16");
    let address = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(address.clone()).await.expect("Failed to get listener");
    let app = app_router();

    // init consumer
    init_consumers(config).await.expect("Failed to init consumers");

    tracing::info!("Listening on http://{}", address);
    axum::serve(listener, app).await.expect("Failed to start axum server");

    if otel_enabled {
        global::shutdown_tracer_provider();
        // TODO: how do we shutdown the meter provider, and why do we have to shut it down ?
        // let _ = global::meter_provider().shutdown();
    }
}
