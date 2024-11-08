use std::time::Duration;

use async_trait::async_trait;
use aws_sdk_eventbridge::types::{InputTransformer, RuleState, Target};
use aws_sdk_sqs::types::QueueAttributeName;

use crate::cron::Cron;
use crate::queue::job_queue::WorkerTriggerType;
use crate::setup::SetupConfig;

pub struct AWSEventBridge {}

const WORKER_TRIGGER_RULE_NAME: &str = "worker_trigger_scheduled";

#[async_trait]
impl Cron for AWSEventBridge {
    #[allow(unreachable_patterns)]
    async fn setup_cron(
        &self,
        config: &SetupConfig,
        cron_time: Duration,
        target_queue_name: String,
        message: String,
        worker_trigger_type: WorkerTriggerType,
    ) -> color_eyre::Result<()> {
        let config = match config {
            SetupConfig::AWS(config) => config,
            _ => panic!("Unsupported Event Bridge configuration"),
        };
        let event_bridge_client = aws_sdk_eventbridge::Client::new(config);
        let sqs_client = aws_sdk_sqs::Client::new(config);

        event_bridge_client
            .put_rule()
            .name(WORKER_TRIGGER_RULE_NAME)
            .schedule_expression(duration_to_rate_string(cron_time))
            .state(RuleState::Enabled)
            .send()
            .await?;
        let queue_url = sqs_client.get_queue_url().queue_name(target_queue_name).send().await?;

        let queue_attributes = sqs_client
            .get_queue_attributes()
            .queue_url(queue_url.queue_url.unwrap())
            .attribute_names(QueueAttributeName::QueueArn)
            .send()
            .await?;
        let queue_arn = queue_attributes.attributes().unwrap().get(&QueueAttributeName::QueueArn).unwrap();

        // Create the EventBridge target with the input transformer
        let input_transformer =
            InputTransformer::builder().input_paths_map("$.time", "time").input_template(message).build()?;

        event_bridge_client
            .put_targets()
            .rule(WORKER_TRIGGER_RULE_NAME)
            .targets(
                Target::builder()
                    .id(format!("worker-trigger-target-{:?}", worker_trigger_type))
                    .arn(queue_arn)
                    .input_transformer(input_transformer)
                    .build()?,
            )
            .send()
            .await?;

        Ok(())
    }
}

fn duration_to_rate_string(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let total_mins = duration.as_secs() / 60;
    let total_hours = duration.as_secs() / 3600;
    let total_days = duration.as_secs() / 86400;

    if total_days > 0 {
        format!("rate({} day{})", total_days, if total_days == 1 { "" } else { "s" })
    } else if total_hours > 0 {
        format!("rate({} hour{})", total_hours, if total_hours == 1 { "" } else { "s" })
    } else if total_mins > 0 {
        format!("rate({} minute{})", total_mins, if total_mins == 1 { "" } else { "s" })
    } else {
        format!("rate({} second{})", total_secs, if total_secs == 1 { "" } else { "s" })
    }
}

#[cfg(test)]
mod event_bridge_utils_test {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_duration_to_rate_string() {
        assert_eq!(duration_to_rate_string(Duration::from_secs(60)), "rate(1 minute)");
        assert_eq!(duration_to_rate_string(Duration::from_secs(120)), "rate(2 minutes)");
        assert_eq!(duration_to_rate_string(Duration::from_secs(30)), "rate(30 seconds)");
        assert_eq!(duration_to_rate_string(Duration::from_secs(3600)), "rate(1 hour)");
        assert_eq!(duration_to_rate_string(Duration::from_secs(86400)), "rate(1 day)");
    }
}
