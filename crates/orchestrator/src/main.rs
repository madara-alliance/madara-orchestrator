use dotenvy::dotenv;
use orchestrator::config::init_config;
use orchestrator::queue::init_consumers;
use orchestrator::routes::app_router;
use utils::env_utils::get_env_var_or_default;

// Instrumentation
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;

use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::Config;
use opentelemetry_sdk::trace::Tracer;
use opentelemetry_sdk::{runtime, Resource};
use tracing::Level;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::prelude::*;

static TEST_NAME: &str = "madara-orchestrator";
static ENDPOINT: &str = "http://localhost:4317";

fn init_tracer_provider() -> Tracer {
    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(ENDPOINT))
        .with_trace_config(Config::default().with_resource(Resource::new(vec![KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            format!("{}{}", TEST_NAME, "_service"),
        )])))
        .install_batch(runtime::Tokio)
        .unwrap();

    global::set_tracer_provider(provider.clone());

    provider.tracer(format!("{}{}", TEST_NAME, "_subscriber"))
}

/// Start the server
#[tokio::main]
async fn main() {
    dotenv().ok();

    let tracer = init_tracer_provider();

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(Level::TRACE))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();

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
}
