use opentelemetry::metrics::{Gauge, Meter};

pub trait Metrics {
    fn register() -> Self;
}

#[macro_export]
macro_rules! register_metric {
    ($type:ty) => {
        Lazy::new(|| <$type>::register())
    };
}
pub fn register_gauge_metric_instrument(
    crate_meter: &Meter,
    instrument_name: String,
    desc: String,
    unit: String,
) -> Gauge<f64> {
    crate_meter.f64_gauge(instrument_name).with_description(desc).with_unit(unit).init()
}
