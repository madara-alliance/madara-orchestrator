use once_cell::sync::Lazy;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::ExportConfig;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::reader::DefaultAggregationSelector;
use opentelemetry_sdk::metrics::reader::DefaultTemporalitySelector;
use opentelemetry_sdk::metrics::PeriodicReader;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::BatchConfigBuilder;
use opentelemetry_sdk::trace::Config;
use opentelemetry_sdk::trace::Tracer;
use opentelemetry_sdk::{runtime, Resource};
use std::sync::Arc;
use std::time::Duration;
use utils::env_utils::get_env_var_or_panic;

use lazy_static::lazy_static;

lazy_static! {
    #[derive(Debug)]
    pub static ref OTEL_SERVICE_NAME: String = get_env_var_or_panic("OTEL_SERVICE_NAME");
    #[derive(Debug)]
    pub static ref OTEL_COLLECTOR_ENDPOINT: String = get_env_var_or_panic("OTEL_COLLECTOR_ENDPOINT");
    #[derive(Debug)]
    pub static ref TRACING_LEVEL: String = get_env_var_or_panic("TRACING_LEVEL");
}

static METER_PROVIDER: Lazy<Arc<SdkMeterProvider>> = Lazy::new(|| {
    let meter_provider = init_metric_provider();
    Arc::new(meter_provider)
});

static TRACER: Lazy<Arc<Tracer>> = Lazy::new(|| {
    let tracer = init_tracer_provider();
    Arc::new(tracer)
});

pub fn init_analytics() {
    // Force initialization of METER_PROVIDER and TRACER
    // Meter provider should be accessed from global scope only
    Lazy::force(&METER_PROVIDER);
    Lazy::force(&TRACER);
}

pub fn global_tracer() -> &'static Tracer {
    &TRACER
}

pub fn global_meter() -> &'static SdkMeterProvider {
    &METER_PROVIDER
}

pub fn init_tracer_provider() -> Tracer {
    let batch_config = BatchConfigBuilder::default()
    // Increasing the queue size and batch size, only increase in queue size delays full channel error.
    .build();

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint((*OTEL_COLLECTOR_ENDPOINT).clone()))
        .with_trace_config(Config::default().with_resource(Resource::new(vec![KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            format!("{}{}", *OTEL_SERVICE_NAME, "_trace_service"),
        )])))
        .with_batch_config(batch_config)
        .install_batch(runtime::Tokio)
        .unwrap();

    global::set_tracer_provider(provider.clone());

    provider.tracer(format!("{}{}", *OTEL_SERVICE_NAME, "_subscriber"))
}

pub fn init_metric_provider() -> SdkMeterProvider {
    let export_config = ExportConfig { endpoint: (*OTEL_COLLECTOR_ENDPOINT).clone(), ..ExportConfig::default() };

    // Creates and builds the OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter().tonic().with_export_config(export_config).build_metrics_exporter(
        // TODO: highly likely that changing these configs will result in correct collection of traces, inhibiting full channel issue
        Box::new(DefaultAggregationSelector::new()),
        Box::new(DefaultTemporalitySelector::new()),
    );

    // Creates a periodic reader that exports every 5 seconds
    let reader =
        PeriodicReader::builder(exporter.unwrap(), runtime::Tokio).with_interval(Duration::from_secs(5)).build();

    // Builds a meter provider with the periodic reader
    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(Resource::new(vec![KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            format!("{}{}", *OTEL_SERVICE_NAME, "_meter_service"),
        )]))
        .build();
    global::set_meter_provider(provider.clone());
    provider
}

#[cfg(test)]
mod tests {
    use crate::metrics::OrchestratorMetrics;
    use utils::{metrics::lib::Metrics, register_metric};

    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_init_metric_provider() {
        // Set up necessary environment variables
        env::set_var("OTEL_COLLECTOR_ENDPOINT", "http://localhost:4317");
        env::set_var("OTEL_SERVICE_NAME", "test_service");

        // Call the function and check if it doesn't panic
        let result = std::panic::catch_unwind(|| {
            let _provider = init_metric_provider();
        });

        // Check if the global meter provider is set
        let _global_provider = global::meter_provider();
        assert!(result.is_ok(), "init_metric_provider() panicked");
    }

    #[tokio::test]
    async fn test_init_tracer_provider() {
        // Set up necessary environment variables
        env::set_var("OTEL_COLLECTOR_ENDPOINT", "http://localhost:4317");
        env::set_var("OTEL_SERVICE_NAME", "test_service");

        // Call the function and check if it doesn't panic
        let result = std::panic::catch_unwind(|| {
            let _tracer = init_tracer_provider();
        });

        assert!(result.is_ok(), "init_tracer_provider() panicked");
    }

    #[tokio::test]
    async fn test_init_analytics() {
        // This test just ensures that the function doesn't panic

        env::set_var("OTEL_COLLECTOR_ENDPOINT", "http://localhost:4317");
        env::set_var("OTEL_SERVICE_NAME", "test_service");

        init_analytics();

        let tracer = global_tracer();
        assert!(std::matches!(tracer, &Tracer { .. }));
    }

    #[tokio::test]
    async fn test_gauge_setter() {
        // This test just ensures that the function doesn't panic

        env::set_var("OTEL_COLLECTOR_ENDPOINT", "http://localhost:4317");
        env::set_var("OTEL_SERVICE_NAME", "test_service");

        init_analytics();

        register_metric!(ORCHESTRATOR_METRICS, OrchestratorMetrics);
    }
}
