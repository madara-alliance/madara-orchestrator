use aws_config::Region;
use orchestrator::data_storage::aws_s3::config::{AWSS3ConfigType, S3LocalStackConfig};
use orchestrator::data_storage::aws_s3::AWSS3;
use orchestrator::data_storage::{DataStorage, DataStorageConfig};
use orchestrator::queue::job_queue::{JOB_PROCESSING_QUEUE, JOB_VERIFICATION_QUEUE, JOB_HANDLE_FAILURE_QUEUE};

const PIE_FILE_URL: &str = "https://madara-orchestrator-sharp-pie.s3.amazonaws.com/238996-SN.zip";
const PIE_FILE_BLOCK_NUMBER: u64 = 238996;

/// LocalStack struct
pub struct LocalStack;

impl LocalStack {
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
        sqs_client.create_queue().queue_name(JOB_PROCESSING_QUEUE).send().await?;
        sqs_client.create_queue().queue_name(JOB_VERIFICATION_QUEUE).send().await?;
        sqs_client.create_queue().queue_name(JOB_HANDLE_FAILURE_QUEUE).send().await?;

        Ok(())
    }

    /// To set up s3 files needed for e2e testing
    pub async fn setup_s3(&self) -> color_eyre::Result<()> {
        let s3_client = self.s3_client().await;

        // getting the PIE file from s3 bucket using URL provided
        let file = reqwest::get(PIE_FILE_URL).await?;
        let file_bytes = file.bytes().await?;

        let s3_file_key = PIE_FILE_BLOCK_NUMBER.to_string() + "/pie.zip";
        s3_client.put_data(file_bytes, &s3_file_key).await?;

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
}
