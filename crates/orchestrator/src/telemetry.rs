use opentelemetry::global;
use opentelemetry::trace::TracerProvider;

use once_cell::sync::Lazy;
use opentelemetry::metrics::Meter;
use opentelemetry::KeyValue;
use opentelemetry_otlp::ExportConfig;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::BatchConfigBuilder;
use opentelemetry_sdk::metrics;
use opentelemetry_sdk::metrics::reader::DefaultAggregationSelector;
use opentelemetry_sdk::metrics::reader::DefaultTemporalitySelector;
use opentelemetry_sdk::metrics::PeriodicReader;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::Config;
use opentelemetry_sdk::trace::Tracer;
use opentelemetry_sdk::{runtime, Resource};
use std::sync::Arc;
use std::time::Duration;

pub static SERVICE_NAME: &str = "service_1";
pub static ENDPOINT: &str = "http://localhost:4317";

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
    let meter = global::meter_with_version(
        "response-time-meter",
        Some("v1.0"),
        Some("schema_url"),
        Some(common_scope_attributes.clone()),
    );
    meter
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
   .with_max_queue_size(10000) // Increase from the default (2048)
   .with_scheduled_delay(Duration::from_secs(5))
   .with_max_export_batch_size(512).build();

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint(ENDPOINT))
        .with_trace_config(Config::default().with_resource(Resource::new(vec![KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            format!("{}{}", SERVICE_NAME, "_service"),
        )])))
        .with_batch_config(batch_config)
        .install_batch(runtime::Tokio)
        .unwrap();

    global::set_tracer_provider(provider.clone());

    provider.tracer(format!("{}{}", SERVICE_NAME, "_subscriber"))
}

pub fn init_metrics() -> SdkMeterProvider {
    let export_config = ExportConfig { endpoint: ENDPOINT.to_string(), ..ExportConfig::default() };

    // Create and build the OTLP exporter
    let exporter = opentelemetry_otlp::new_exporter().tonic().with_export_config(export_config).build_metrics_exporter(
        Box::new(DefaultAggregationSelector::new()),
        Box::new(DefaultTemporalitySelector::new()),
    );

    // Create a periodic reader that exports every 5 seconds
    let reader =
        PeriodicReader::builder(exporter.unwrap(), runtime::Tokio).with_interval(Duration::from_secs(5)).build();

    // Build a meter provider with the periodic reader
    let provider = SdkMeterProvider::builder()
      .with_reader(reader)
      .with_resource(Resource::new(vec![KeyValue::new(
          opentelemetry_semantic_conventions::resource::SERVICE_NAME,
          format!("{}{}", SERVICE_NAME, "_service"),
      )]))
      .with_view(metrics::new_view(
          metrics::Instrument::new().name(format!("{}{}", SERVICE_NAME, "_response_time_create_job_histogram")),
          metrics::Stream::new().aggregation(
              metrics::Aggregation::ExplicitBucketHistogram {
                  boundaries: vec![100.0, 200.0, 500.0, 1000.0, 2000.0, 10000.0],
                  record_min_max: true,
              },
          ),
      ).unwrap())
      .build();
    global::set_meter_provider(provider.clone());

    provider
}
