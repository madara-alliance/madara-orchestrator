use std::sync::Arc;

use async_trait::async_trait;
use aws_sdk_sns::Client;

use crate::alerts::Alerts;
use crate::config::ProviderConfig;

#[derive(Debug, Clone)]
pub struct AWSSNSValidatedArgs {
    // TODO: convert to ARN type, and validate it
    // NOTE: aws is using str to represent ARN : https://docs.aws.amazon.com/sdk-for-rust/latest/dg/rust_sns_code_examples.html
    pub sns_arn: String,
}

impl AWSSNSValidatedArgs {
    pub fn get_topic_name(&self) -> String {
        self.sns_arn.split(":").last().unwrap().to_string()
    }
}

pub struct AWSSNS {
    client: Client,
    topic_arn: String,
}

impl AWSSNS {
    pub async fn new_with_params(aws_sns_params: &AWSSNSValidatedArgs, provider_config: Arc<ProviderConfig>) -> Self {
        let config = provider_config.get_aws_client_or_panic();
        Self { client: Client::new(config), topic_arn: aws_sns_params.sns_arn.clone() }
    }
}

#[async_trait]
impl Alerts for AWSSNS {
    async fn send_alert_message(&self, message_body: String) -> color_eyre::Result<()> {
        self.client.publish().topic_arn(self.topic_arn.clone()).message(message_body).send().await?;
        Ok(())
    }
}
