use aws_sdk_eventbridge::types::{InputTransformer, RuleState, Target};
use aws_sdk_sqs::types::QueueAttributeName;

use crate::queue::job_queue::{WorkerTriggerMessage, WorkerTriggerType};
use crate::setup::SetupConfig;

#[allow(unreachable_patterns)]
pub async fn setup_event_bridge_for_trigger_type(
    worker_trigger_type: WorkerTriggerType,
    config: &SetupConfig,
    rule_name: &str,
    worker_trigger_queue_name: &str,
) -> color_eyre::Result<()> {
    let config = match config {
        SetupConfig::AWS(config) => config,
        _ => panic!("Unsupported Event Bridge configuration"),
    };
    let event_bridge_client = aws_sdk_eventbridge::Client::new(config);
    let sqs_client = aws_sdk_sqs::Client::new(config);

    event_bridge_client
        .put_rule()
        .name(rule_name)
        .schedule_expression("rate(1 minute)")
        .state(RuleState::Enabled)
        .send()
        .await?;
    let queue_url = sqs_client.get_queue_url().queue_name(worker_trigger_queue_name).send().await?;

    let queue_attributes = sqs_client
        .get_queue_attributes()
        .queue_url(queue_url.queue_url.unwrap())
        .attribute_names(QueueAttributeName::QueueArn)
        .send()
        .await?;
    let queue_arn = queue_attributes.attributes().unwrap().get(&QueueAttributeName::QueueArn).unwrap();

    // Create a sample WorkerTriggerMessage
    let message = WorkerTriggerMessage { worker: worker_trigger_type.clone() };
    let event_detail = serde_json::to_string(&message)?;

    // Create the EventBridge target with the input transformer
    let input_transformer = InputTransformer::builder()
        .input_paths_map("$.time", "time")
        .input_template(event_detail.to_string())
        .build()?;

    event_bridge_client
        .put_targets()
        .rule(rule_name)
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
