pub mod aws_sns;

#[derive(Clone, Debug)]
pub enum AlertParams {
    AWSSNS(aws_sns::AWSSNSParams),
}