use clap::Args;

/// Parameters used to config instrumentation.
#[derive(Debug, Clone, Args)]
pub struct InstrumentationParams {
    /// The name of the instrumentation service.
    #[arg(env = "OTEL_SERVICE_NAME", long)]
    pub service_name: String,

    /// The endpoint of the collector.
    #[arg(env = "OTEL_COLLECTOR_ENDPOINT", long)]
    pub collector_endpoint: String,
}
