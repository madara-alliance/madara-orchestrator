use crate::alerts::aws_sns::AWSSNSValidatedArgs;

pub mod event_bridge;

#[derive(Clone, Debug)]
pub enum CronParams {
    AWSSNS(AWSSNSValidatedArgs),
}
