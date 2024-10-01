use once_cell::sync::Lazy;
use opentelemetry::global;
use opentelemetry::metrics::Meter;
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
    let meter_provider = init_metrics();
    global::set_meter_provider(meter_provider.clone());
    Arc::new(meter_provider)
});

static TRACER: Lazy<Arc<Tracer>> = Lazy::new(|| {
    let tracer = init_tracer_provider();
    Arc::new(tracer)
});

static GLOBAL_METER: Lazy<Meter> = Lazy::new(|| {
    let common_scope_attributes = vec![KeyValue::new("crate", "orchestrator")];

    global::meter_with_version(
        "orchestrator.opentelemetry",
        // TODO: Unsure of these settings, come back
        Some("0.17"),
        Some("https://opentelemetry.io/schemas/1.2.0"),
        Some(common_scope_attributes.clone()),
    )
});

pub fn init() {
    // Force initialization of METER_PROVIDER and GLOBAL_METER
    Lazy::force(&METER_PROVIDER);
    Lazy::force(&GLOBAL_METER);
    Lazy::force(&TRACER);
}

pub fn global_meter() -> &'static Meter {
    &GLOBAL_METER
}

pub fn global_meter_provider() -> &'static SdkMeterProvider {
    &METER_PROVIDER
}

pub fn global_tracer() -> &'static Tracer {
    &TRACER
}

pub fn init_tracer_provider() -> Tracer {
    let batch_config = BatchConfigBuilder::default()
    // Increasing the queue size and batch size, only increase in queue size delays full channel error.
    .with_max_queue_size(10000)
    .with_max_export_batch_size(512) // On default
    .with_scheduled_delay(Duration::from_secs(5)) // On default
    .build();

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(OTEL_COLLECTOR_ENDPOINT.to_string()))
        .with_trace_config(Config::default().with_resource(Resource::new(vec![KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            format!("{:?}{}", OTEL_SERVICE_NAME, "_trace_service"),
        )])))
        .with_batch_config(batch_config)
        .install_batch(runtime::Tokio)
        .unwrap();

    global::set_tracer_provider(provider.clone());

    provider.tracer(format!("{:?}{}", OTEL_SERVICE_NAME, "_subscriber"))
}

/// FOR STDOUT
// use opentelemetry_otlp::SpanExporter;
// use opentelemetry_stdout as stdout;
// use opentelemetry_sdk::metrics;

// pub fn init_tracer_provider() -> Tracer {
//      let exporter = opentelemetry_stdout::SpanExporter::default();
//     let provider = opentelemetry_sdk::trace::TracerProvider::builder()
//         .with_simple_exporter(exporter)
//         .with_config(Config::default()
//         .with_resource(Resource::new(vec![KeyValue::new(
//                         opentelemetry_semantic_conventions::resource::OTEL_SERVICE_NAME,
//                         format!("{}{}", OTEL_SERVICE_NAME, "_service"),
//                     )])))
//         .build();
//     global::set_tracer_provider(provider.clone());
//     provider.tracer(format!("{}{}", OTEL_SERVICE_NAME, "_subscriber"))
// }

pub fn init_metrics() -> SdkMeterProvider {
    let export_config = ExportConfig { endpoint: OTEL_COLLECTOR_ENDPOINT.to_string(), ..ExportConfig::default() };

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
            format!("{:?}{}", OTEL_SERVICE_NAME, "_meter_service"),
        )]))
        .build();
    global::set_meter_provider(provider.clone());
    provider
}
