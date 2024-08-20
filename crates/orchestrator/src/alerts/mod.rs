use async_trait::async_trait;

pub mod aws_sns;

#[async_trait]
pub trait Alerts: Send + Sync {
    /// To send an alert message to our alert service
    async fn send_alert_message(&self, message_body: String) -> color_eyre::Result<()>;
}
