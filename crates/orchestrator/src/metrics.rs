use crate::telemetry::OTEL_SERVICE_NAME;
use once_cell::sync::Lazy;
use opentelemetry::{
    global,
    metrics::{Gauge, Meter},
    KeyValue,
};
use utils::{
    metrics::lib::{register_gauge_metric_instrument, Metrics},
    register_metric,
};

pub static ORCHESTRATOR_METRICS: Lazy<OrchestratorMetrics> = register_metric!(OrchestratorMetrics);

pub struct OrchestratorMetrics {
    pub meter: Meter,
    pub block_gauge: Gauge<f64>,
}

impl Metrics for OrchestratorMetrics {
    fn register() -> Self {
        // Register meter
        let common_scope_attributes = vec![KeyValue::new("crate", "orchestrator")];
        let orchestrator_meter = global::meter_with_version(
            "crates.orchestrator.opentelemetry",
            // TODO: Unsure of these settings, come back
            Some("0.17"),
            Some("https://opentelemetry.io/schemas/1.2.0"),
            Some(common_scope_attributes.clone()),
        );

        // Register all instruments
        let block_gauge = register_gauge_metric_instrument(
            &orchestrator_meter,
            format!("{:?}{}", OTEL_SERVICE_NAME, "_block_state"),
            "A gauge to show block state at given time".to_string(),
            "block".to_string(),
        );

        Self { meter: orchestrator_meter, block_gauge }
    }
}
