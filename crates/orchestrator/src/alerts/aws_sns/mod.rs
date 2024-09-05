mod config;

use crate::alerts::aws_sns::config::AWSSNSConfig;
use crate::alerts::Alerts;
use async_trait::async_trait;
use aws_sdk_sns::config::Region;
use aws_sdk_sns::Client;
use utils::settings::Settings;

pub const AWS_SNS_SETTINGS_NAME: &str = "sns";

pub struct AWSSNS {
    client: Client,
    topic_arn: String,
}

impl AWSSNS {
    pub async fn new_with_settings(settings: &impl Settings) -> Self {
        let sns_config =
            AWSSNSConfig::new_with_settings(settings).expect("Not able to get Aws sns config from provided settings");
        let config = aws_config::from_env().region(Region::new(sns_config.sns_arn_region)).load().await;
        Self { client: Client::new(&config), topic_arn: sns_config.sns_arn }
    }
}

#[async_trait]
impl Alerts for AWSSNS {
    async fn send_alert_message(&self, message_body: String) -> color_eyre::Result<()> {
        self.client.publish().topic_arn(self.topic_arn.clone()).message(message_body).send().await?;
        Ok(())
    }
}
