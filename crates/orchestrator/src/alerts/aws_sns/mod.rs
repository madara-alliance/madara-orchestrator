use std::sync::Arc;

use async_trait::async_trait;
use aws_sdk_sns::Client;
use utils::cli::alert::aws_sns::AWSSNSParams;

use crate::alerts::Alerts;
use crate::config::ProviderConfig;

pub struct AWSSNS {
    client: Client,
    topic_arn: String,
}

impl AWSSNS {
    pub async fn new_with_settings(aws_sns_params: &AWSSNSParams, provider_config: Arc<ProviderConfig>) -> Self {
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
