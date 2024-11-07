use async_trait::async_trait;
use mockall::automock;
use utils::settings::env::EnvSettingsProvider;
use utils::settings::Settings;

pub mod aws_sns;

#[automock]
#[async_trait]
pub trait Alerts: Send + Sync {
    /// To send an alert message to our alert service
    async fn send_alert_message(&self, message_body: String) -> color_eyre::Result<()>;
    async fn create_alert(&self, topic_name: &str) -> color_eyre::Result<()>;
    async fn setup(&self) -> color_eyre::Result<()> {
        let settings_provider = EnvSettingsProvider {};
        let sns_topic_name = settings_provider.get_settings_or_panic("SNS_TOPIC_NAME");
        self.create_alert(&sns_topic_name).await?;
        Ok(())
    }
}
