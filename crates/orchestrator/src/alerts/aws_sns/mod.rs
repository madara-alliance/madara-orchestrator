use crate::alerts::Alerts;
use async_trait::async_trait;
use aws_config::SdkConfig;
use aws_sdk_sns::config::Region;
use aws_sdk_sns::Client;
use utils::env_utils::get_env_var_or_panic;

pub struct AWSSNS {
    client: Client,
    topic_arn: String,
}

impl AWSSNS {
    /// To create a new SNS client from passed config
    pub async fn new(config: SdkConfig, topic_arn: String) -> Self {
        AWSSNS { client: Client::new(&config), topic_arn }
    }

    /// To create a new SNS client from env
    pub async fn new_from_env() -> Self {
        let sns_region = get_env_var_or_panic("AWS_SNS_REGION");
        let topic_arn = get_env_var_or_panic("AWS_SNS_ARN");
        let config = aws_config::from_env().region(Region::new(sns_region)).load().await;
        AWSSNS { client: Client::new(&config), topic_arn }
    }

    /// To get the service's ARN (Amazon Resource Name)
    fn get_topic_arn(&self) -> String {
        self.topic_arn.clone()
    }
}

#[async_trait]
impl Alerts for AWSSNS {
    async fn send_alert_message(&self, message_body: String) -> color_eyre::Result<Option<String>> {
        let topic_arn = self.get_topic_arn();
        let published = self.client.publish().topic_arn(topic_arn).message(message_body).send().await?;
        Ok(published.message_id)
    }
}
