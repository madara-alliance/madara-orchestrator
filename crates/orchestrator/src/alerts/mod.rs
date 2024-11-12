use async_trait::async_trait;
use mockall::automock;

use crate::cli::alert::AlertParams;

pub mod aws_sns;

#[automock]
#[async_trait]
pub trait Alerts: Send + Sync {
    /// To send an alert message to our alert service
    async fn send_alert_message(&self, message_body: String) -> color_eyre::Result<()>;
    async fn create_alert(&self, topic_name: &str) -> color_eyre::Result<()>;
    async fn setup(&self, params: AlertParams) -> color_eyre::Result<()> {
        match params {
            AlertParams::AWSSNS(aws_sns_params) => {
                let sns_topic_name = aws_sns_params.get_topic_name();
                self.create_alert(&sns_topic_name).await?;
            }
        }
        Ok(())
    }
}
