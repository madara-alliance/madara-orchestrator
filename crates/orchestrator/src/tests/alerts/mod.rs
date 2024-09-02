use crate::tests::config::{TestConfigBuilder, SNS_ALERT_TEST_QUEUE_NAME};
use rstest::rstest;
use std::time::Duration;
use tokio::time::sleep;

#[rstest]
#[tokio::test]
async fn sns_alert_subscribe_to_topic_receive_alert_works() {
    let services = TestConfigBuilder::new().testcontainer_sns_sqs_alert().await.build().await;

    let sqs_client = services.queue_client.unwrap();
    let queue = sqs_client.create_queue().queue_name(SNS_ALERT_TEST_QUEUE_NAME).send().await.unwrap();
    let queue_url = queue.queue_url().unwrap();

    let message_to_send = "Hello World :)";

    // Getting sns client from the module
    let alerts_client = services.config.alerts();
    // Sending the alert message
    alerts_client.send_alert_message(message_to_send.to_string()).await.unwrap();

    sleep(Duration::from_secs(5)).await;

    // Checking the queue for message
    let consumed_messages =
        services.config.queue().consume_message_from_queue(SNS_ALERT_TEST_QUEUE_NAME.to_string()).await;

    assert!(consumed_messages.unwrap().take_payload().is_some());
}
