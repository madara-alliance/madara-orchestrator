use crate::alerts::aws_sns::AWSSNSParams;

pub mod aws_sns;

#[derive(Clone, Debug)]
pub enum AlertParams {
    AWSSNS(AWSSNSParams),
}
