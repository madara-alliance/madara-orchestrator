use crate::get_env_var_or_panic;
use aws_config::Region;
use aws_sdk_eventbridge::types::{InputTransformer, RuleState, Target};
use aws_sdk_sqs::types::QueueAttributeName;
use aws_sdk_sqs::types::QueueAttributeName::VisibilityTimeout;
use bytes::Bytes;
use orchestrator::data_storage::aws_s3::config::{AWSS3ConfigType, S3LocalStackConfig};
use orchestrator::data_storage::aws_s3::AWSS3;
use orchestrator::data_storage::{DataStorage, DataStorageConfig};
use orchestrator::queue::job_queue::{
    WorkerTriggerMessage, WorkerTriggerType, JOB_HANDLE_FAILURE_QUEUE, JOB_PROCESSING_QUEUE, JOB_VERIFICATION_QUEUE,
    WORKER_TRIGGER_QUEUE,
};
use std::collections::HashMap;
use std::fs::read;

/// LocalStack struct
pub struct LocalStack {
    l2_block_number: String,
}

impl LocalStack {
    pub fn new() -> Self {
        Self { l2_block_number: get_env_var_or_panic("L2_BLOCK_NUMBER_FOR_TEST") }
    }

    pub fn l2_block_number(&self) -> String {
        self.l2_block_number.clone()
    }

    /// To set up SQS on localstack instance
    pub async fn setup_sqs(&self) -> color_eyre::Result<()> {
        let sqs_client = self.sqs_client().await;

        let list_queues_output = sqs_client.list_queues().send().await?;
        let queue_urls = list_queues_output.queue_urls();
        log::debug!("Found {} queues", queue_urls.len());
        for queue_url in queue_urls {
            match sqs_client.delete_queue().queue_url(queue_url).send().await {
                Ok(_) => log::debug!("Successfully deleted queue: {}", queue_url),
                Err(e) => eprintln!("Error deleting queue {}: {:?}", queue_url, e),
            }
        }

        // Creating SQS queues
        let mut queue_attributes = HashMap::new();
        queue_attributes.insert(VisibilityTimeout, "1".into());
        sqs_client
            .create_queue()
            .queue_name(JOB_PROCESSING_QUEUE)
            .set_attributes(Some(queue_attributes.clone()))
            .send()
            .await?;
        sqs_client
            .create_queue()
            .queue_name(JOB_VERIFICATION_QUEUE)
            .set_attributes(Some(queue_attributes.clone()))
            .send()
            .await?;
        sqs_client
            .create_queue()
            .queue_name(JOB_HANDLE_FAILURE_QUEUE)
            .set_attributes(Some(queue_attributes.clone()))
            .send()
            .await?;
        sqs_client
            .create_queue()
            .queue_name(WORKER_TRIGGER_QUEUE)
            .set_attributes(Some(queue_attributes.clone()))
            .send()
            .await?;
        println!("🌊 SQS queues creation completed.");

        Ok(())
    }

    /// To set up s3 files needed for e2e testing
    pub async fn setup_s3(&self) -> color_eyre::Result<()> {
        let s3_client = self.s3_client().await;

        s3_client.build_test_bucket(&get_env_var_or_panic("AWS_S3_BUCKET_NAME")).await.unwrap();

        // putting the snos output and program output for the given block into localstack s3
        let snos_output_key = self.l2_block_number.to_string() + "/snos_output.json";
        let snos_output_json = read("artifacts/snos_output.json").unwrap();
        s3_client.put_data(Bytes::from(snos_output_json), &snos_output_key).await?;
        println!("✔️ snos output file uploaded to localstack s3.");

        let program_output_key = self.l2_block_number.to_string() + "/program_output.json";
        let program_output = read(format!("artifacts/program_output_{}.txt", self.l2_block_number)).unwrap();
        s3_client.put_data(Bytes::from(program_output), &program_output_key).await?;
        println!("✔️ program output file uploaded to localstack s3.");

        // getting the PIE file from s3 bucket using URL provided
        let file = reqwest::get(format!(
            "https://madara-orchestrator-sharp-pie.s3.amazonaws.com/{}-SN.zip",
            self.l2_block_number
        ))
        .await?;
        let file_bytes = file.bytes().await?;

        // putting the pie file into localstack s3
        let s3_file_key = self.l2_block_number.to_string() + "/pie.zip";
        s3_client.put_data(file_bytes, &s3_file_key).await?;
        println!("✔️ PIE file uploaded to localstack s3");

        Ok(())
    }

    /// Event Bridge setup
    pub async fn setup_event_bridge(&self, worker_trigger_type: WorkerTriggerType) -> color_eyre::Result<()> {
        let event_bridge_client = self.event_bridge_client().await;
        let sqs_client = self.sqs_client().await;

        let rule_name = "worker_trigger_scheduled";

        event_bridge_client
            .put_rule()
            .name(rule_name)
            .schedule_expression("rate(1 minute)")
            .state(RuleState::Enabled)
            .send()
            .await?;
        let queue_url = sqs_client.get_queue_url().queue_name(WORKER_TRIGGER_QUEUE).send().await?;

        let queue_attributes = sqs_client
            .get_queue_attributes()
            .queue_url(&queue_url.queue_url.unwrap())
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
            .input_template(format!("{}", event_detail))
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

        println!("🌉 Event bridge setup completed. Trigger Type : {:?}", worker_trigger_type);

        Ok(())
    }

    /// Generic function to send message to any of the queues
    pub async fn send_message_to_queue(&self, queue_url: &str, message_body: &str) -> color_eyre::Result<()> {
        let sqs_client = self.sqs_client().await;
        sqs_client.send_message().queue_url(queue_url).message_body(message_body).send().await?;
        Ok(())
    }

    async fn sqs_client(&self) -> aws_sdk_sqs::Client {
        let region_provider = Region::new("us-east-1");
        let config = aws_config::from_env().region(region_provider).load().await;
        aws_sdk_sqs::Client::new(&config)
    }

    async fn s3_client(&self) -> Box<dyn DataStorage + Send + Sync> {
        Box::new(AWSS3::new(AWSS3ConfigType::WithEndpoint(S3LocalStackConfig::new_from_env())).await)
    }

    async fn event_bridge_client(&self) -> aws_sdk_eventbridge::Client {
        let region_provider = Region::new("us-east-1");
        let config = aws_config::from_env().region(region_provider).load().await;
        aws_sdk_eventbridge::Client::new(&config)
    }

    pub async fn delete_event_bridge_rule(&self, rule_name: &str) -> color_eyre::Result<()> {
        let event_bridge_client = self.event_bridge_client().await;

        let list_targets_output = event_bridge_client.list_targets_by_rule().rule(rule_name).send().await;
        
        match list_targets_output { 
            Ok(output) => {
                let targets = output.targets();
                if !targets.is_empty() {
                    let target_ids: Vec<String> = targets.iter().map(|t| t.id().to_string()).collect();

                    event_bridge_client.remove_targets().rule(rule_name).set_ids(Some(target_ids)).send().await?;

                    println!("🧹 Removed targets from rule: {}", rule_name);
                }

                // Step 2: Delete the rule
                event_bridge_client.delete_rule().name(rule_name).send().await?;

                println!("🧹 Deleted EventBridge rule: {}", rule_name);
                println!("🧹 Rule deleted successfully.");

                Ok(())
            },
            Err(_) => {
                return Ok(());
            }
        }
    }
}
