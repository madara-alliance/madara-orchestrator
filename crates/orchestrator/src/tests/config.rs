use crate::alerts::aws_sns::AWSSNS;
use crate::config::Config;
use crate::data_storage::{DataStorage, DataStorageConfig, MockDataStorage};
use crate::queue::job_queue::{JOB_PROCESSING_QUEUE, JOB_VERIFICATION_QUEUE};
use aws_sdk_s3::config::Credentials;
use aws_sdk_sqs::types::QueueAttributeName::QueueArn;
use da_client_interface::{DaClient, MockDaClient};
use httpmock::MockServer;
use std::sync::Arc;
use testcontainers::core::IntoContainerPort;

use crate::alerts::{Alerts, MockAlerts};
use prover_client_interface::{MockProverClient, ProverClient};
use settlement_client_interface::{MockSettlementClient, SettlementClient};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Url};
use testcontainers::core::logs::consumer::logging_consumer::LoggingConsumer;
use url::Host;
use utils::env_utils::get_env_var_or_panic;

use crate::database::mongodb::config::MongoDbConfig;
use crate::database::mongodb::MongoDb;
use crate::database::{Database, MockDatabase};
use crate::queue::sqs::SqsQueue;
use crate::queue::{MockQueueProvider, QueueProvider};

// Inspiration : https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
// TestConfigBuilder allows to heavily customise the global configs based on the test's requirement.
// Eg: We want to mock only the da client and leave rest to be as it is, use add_da_client.

const DEFAULT_RETRY_ATTEMPTS: u16 = 10;
pub const SNS_ALERT_TEST_TOPIC_NAME: &str = "madara-orchestrator-alert";
pub const SNS_ALERT_TEST_QUEUE_NAME: &str = "madara-orchestrator-queue";

// TestBuilder for Config
pub struct TestConfigBuilder {
    /// The starknet client to get data from the node
    starknet_client: Option<Arc<JsonRpcClient<HttpTransport>>>,
    /// The DA client to interact with the DA layer
    da_client: Option<Box<dyn DaClient>>,
    /// The service that produces proof and registers it onchain
    prover_client: Option<Box<dyn ProverClient>>,
    /// Settlement client
    settlement_client: Option<Box<dyn SettlementClient>>,
    /// The database client
    database: Option<Box<dyn Database>>,
    /// Queue client
    queue: Option<Box<dyn QueueProvider>>,
    /// Storage client
    storage: Option<Box<dyn DataStorage>>,
    /// Alerts client
    alerts: Option<Box<dyn Alerts>>,

    // Storing for Data Storage client
    // These are need to be kept in scope to keep the Server alive
    database_node: Option<ContainerAsync<Mongo>>,
    data_storage_node: Option<ContainerAsync<LocalStack>>,
    data_storage_client: Option<aws_sdk_s3::Client>,
    queue_node: Option<ContainerAsync<LocalStack>>,
    queue_client: Option<sqs::Client>,
    pub alert_node: Option<ContainerAsync<LocalStack>>,
    pub alert_client: Option<sns::Client>,
}

pub struct TestConfigBuildReturn {
    pub mock_server: MockServer,
    pub data_storage_node: Option<ContainerAsync<LocalStack>>,
    pub data_storage_client: Option<aws_sdk_s3::Client>,
    pub database_node: Option<ContainerAsync<Mongo>>,
    pub queue_client: Option<sqs::Client>,
    pub queue_node: Option<ContainerAsync<LocalStack>>,
    pub alert_client: Option<sns::Client>,
    pub alert_node: Option<ContainerAsync<LocalStack>>,

    pub config: Arc<Config>,
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestConfigBuilder {
    /// Create a new config
    pub fn new() -> TestConfigBuilder {
        TestConfigBuilder {
            starknet_client: None,
            da_client: None,
            prover_client: None,
            settlement_client: None,
            database: None,
            queue: None,
            storage: None,
            alerts: None,

            database_node: None,
            data_storage_node: None,
            data_storage_client: None,
            queue_node: None,
            queue_client: None,
            alert_client: None,
            alert_node: None,
        }
    }

    pub async fn testcontainer_s3_data_storage(mut self) -> TestConfigBuilder {
        let (node, storage_client, client) = s3_testcontainer_setup().await;
        self.data_storage_node = Some(node);
        self.data_storage_client = Some(client);
        self.storage = Some(storage_client);
        self
    }

    pub async fn testcontainer_sqs_queue(mut self) -> TestConfigBuilder {
        let (node, queue_client, client) = sqs_testcontainer_setup().await;
        self.queue = Some(queue_client);
        self.queue_client = Some(client);
        self.queue_node = Some(node);
        self
    }

    // IMP! Don't use SQS ans SNS testcontainer setups together
    pub async fn testcontainer_sns_sqs_alert(mut self) -> TestConfigBuilder {
        let (node, sns_alert, sqs_queue, sns_client, sqs_client, _sqs_arn, _queue_host_url) =
            sns_sqs_testcontainer_setup().await;
        self.queue = Some(sqs_queue);
        self.queue_client = Some(sqs_client);
        self.alerts = Some(sns_alert);
        self.alert_client = Some(sns_client);

        self.alert_node = Some(node);
        self
    }

    pub async fn testcontainer_mongo_database(mut self) -> TestConfigBuilder {
        let (node, database) = mongodb_testcontainer_setup().await;
        self.database = Some(database);
        self.database_node = Some(node);
        self
    }

    pub fn add_da_client(mut self, da_client: Box<dyn DaClient>) -> TestConfigBuilder {
        self.da_client = Some(da_client);
        self
    }

    pub fn add_db_client(mut self, db_client: Box<dyn Database>) -> TestConfigBuilder {
        self.database = Some(db_client);
        self
    }

    pub fn add_settlement_client(mut self, settlement_client: Box<dyn SettlementClient>) -> TestConfigBuilder {
        self.settlement_client = Some(settlement_client);
        self
    }

    pub fn add_starknet_client(mut self, starknet_client: Arc<JsonRpcClient<HttpTransport>>) -> TestConfigBuilder {
        self.starknet_client = Some(starknet_client);
        self
    }

    pub fn add_prover_client(mut self, prover_client: Box<dyn ProverClient>) -> TestConfigBuilder {
        self.prover_client = Some(prover_client);
        self
    }

    pub fn add_storage_client(mut self, storage_client: Box<dyn DataStorage>) -> TestConfigBuilder {
        self.storage = Some(storage_client);
        self
    }

    pub fn add_queue(mut self, queue: Box<dyn QueueProvider>) -> TestConfigBuilder {
        self.queue = Some(queue);
        self
    }

    pub fn add_alert(mut self, alerts: Box<dyn Alerts>) -> TestConfigBuilder {
        self.alerts = Some(alerts);
        self
    }

    pub async fn build(mut self) -> TestConfigBuildReturn {
        dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

        // All default initializations are Mocked.
        let server = MockServer::start();

        // Clients

        // init the DA client
        if self.da_client.is_none() {
            self.da_client = Some(Box::new(MockDaClient::new()));
        }

        // init the Settings client
        if self.settlement_client.is_none() {
            self.settlement_client = Some(Box::new(MockSettlementClient::new()));
        }

        // init the alert client
        if self.alerts.is_none() {
            self.alerts = Some(Box::new(MockAlerts::new()));
        }

        // init the prover client
        if self.prover_client.is_none() {
            self.prover_client = Some(Box::new(MockProverClient::new()))
        }

        // External Dependencies

        // init the storage client
        if self.storage.is_none() {
            self.storage = Some(Box::new(MockDataStorage::new()));
        }

        // init the database
        if self.database.is_none() {
            self.database = Some(Box::new(MockDatabase::new()));
        }

        // init the queue
        if self.queue.is_none() {
            self.queue = Some(Box::new(MockQueueProvider::new()));
        }

        let config = Arc::new(Config::new(
            self.starknet_client.unwrap_or_else(|| {
                let provider = JsonRpcClient::new(HttpTransport::new(
                    Url::parse(format!("http://localhost:{}", server.port()).as_str()).expect("Failed to parse URL"),
                ));
                Arc::new(provider)
            }),
            self.da_client.unwrap(),
            self.prover_client.unwrap(),
            self.settlement_client.unwrap(),
            self.database.unwrap(),
            self.queue.unwrap(),
            self.storage.unwrap(),
            self.alerts.unwrap(),
        ));

        TestConfigBuildReturn {
            mock_server: server,
            data_storage_node: self.data_storage_node,
            data_storage_client: self.data_storage_client,
            database_node: self.database_node,
            queue_client: self.queue_client,
            queue_node: self.queue_node,
            alert_client: self.alert_client,
            alert_node: self.alert_node,
            config,
        }
    }
}

/// LocalStack (s3 and sqs) & MongoDb Setup using TestContainers ////
use super::common::testcontainer_setups::LocalStack;
use crate::data_storage::aws_s3::config::AWSS3Config;
use crate::data_storage::aws_s3::AWSS3;
use crate::tests::common::testcontainer_setups::Mongo;
use aws_config::{BehaviorVersion, Region, SdkConfig};
use aws_sdk_s3 as s3;
use aws_sdk_sns as sns;
use aws_sdk_sqs as sqs;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};

/// LocalStack S3 testcontainer
pub async fn s3_testcontainer_setup() -> (ContainerAsync<LocalStack>, Box<dyn DataStorage>, s3::Client) {
    let (node, host_ip, host_port) = setup_localstack().await;
    let config = setup_aws_shared_config(host_ip, host_port).await;

    let s3_config_builder = aws_sdk_s3::config::Builder::from(&config);
    let s3_client = aws_sdk_s3::Client::from_conf(s3_config_builder.force_path_style(true).build());
    let aws_s3_bucket_name = get_env_var_or_panic("AWS_S3_BUCKET_NAME");

    // Creating s3 bucket.
    s3_client.create_bucket().bucket(aws_s3_bucket_name.clone()).send().await.unwrap();

    let storage_client = AWSS3::new(AWSS3Config::new_from_env(), &config);
    (node, Box::new(storage_client) as Box<dyn DataStorage>, s3_client)
}

fn transform_url(input: &str, host_port: &u16) -> String {
    let parsed_url = Url::parse(input).expect("Failed to parse URL");
    let host = parsed_url.host_str().unwrap();
    let first_path_segment = parsed_url.path_segments().and_then(|mut segments| segments.next()).unwrap_or("");
    format!("http://{}:{}/{}", host, host_port, first_path_segment)
}

/// Localstack SQS testcontainer
pub async fn sqs_testcontainer_setup() -> (ContainerAsync<LocalStack>, Box<dyn QueueProvider>, sqs::Client) {
    let (node, host_ip, host_port) = setup_localstack().await;
    let config = setup_aws_shared_config(host_ip, host_port).await;

    let sqs_client = sqs::Client::new(&config);

    // Queue creation
    let processing_queue_output =
        sqs_client.create_queue().queue_name(JOB_PROCESSING_QUEUE.to_string()).send().await.unwrap();
    let _verification_queue_output =
        sqs_client.create_queue().queue_name(JOB_VERIFICATION_QUEUE.to_string()).send().await.unwrap();

    let queue_host_url = transform_url(processing_queue_output.queue_url().unwrap(), &host_port);

    let sqs_queue = SqsQueue::new(queue_host_url.to_string());

    (node, Box::new(sqs_queue) as Box<dyn QueueProvider>, sqs_client)
}

/// LocalStack SNS-SQS testcontainer
pub async fn sns_sqs_testcontainer_setup(
) -> (ContainerAsync<LocalStack>, Box<dyn Alerts>, Box<dyn QueueProvider>, sns::Client, sqs::Client, String, String) {
    let (node, host_ip, host_port) = setup_localstack().await;
    let config = setup_aws_shared_config(host_ip, host_port).await;

    let sns_client = sns::Client::new(&config);
    let sqs_client = sqs::Client::new(&config);

    let create_topic_output = sns_client.create_topic().name(SNS_ALERT_TEST_TOPIC_NAME).send().await.unwrap();
    let sns_arn = create_topic_output.topic_arn().unwrap().to_string();

    let create_queue_output = sqs_client.create_queue().queue_name(SNS_ALERT_TEST_QUEUE_NAME).send().await.unwrap();

    let sqs_queue = SqsQueue::new(transform_url(create_queue_output.queue_url().unwrap(), &host_port).to_string());
    let queue_host_url = sqs_queue.get_queue_url(SNS_ALERT_TEST_QUEUE_NAME.to_string());

    let sqs_arn = sqs_client
        .get_queue_attributes()
        .queue_url(queue_host_url.clone())
        .attribute_names(QueueArn)
        .send()
        .await
        .unwrap()
        .attributes()
        .unwrap()
        .get(&QueueArn)
        .unwrap()
        .to_string();

    sns_client.subscribe().topic_arn(sns_arn.clone()).protocol("sqs").endpoint(&sqs_arn).send().await.unwrap();

    let sns_alert = AWSSNS::new(config, sns_arn).await;

    (
        node,
        Box::new(sns_alert) as Box<dyn Alerts>,
        Box::new(sqs_queue) as Box<dyn QueueProvider>,
        sns_client,
        sqs_client,
        sqs_arn,
        queue_host_url,
    )
}

async fn setup_localstack() -> (ContainerAsync<LocalStack>, Host, u16) {
    dotenvy::from_filename("../.env.test").unwrap();
    let _ = pretty_env_logger::try_init();

    let mut attempt_count: u16 = 1;

    loop {
        let logger = LoggingConsumer::new().with_stdout_level(log::Level::Info).with_stderr_level(log::Level::Error);

        let node = LocalStack::default().with_log_consumer(logger).start().await.unwrap();
        let host_ip = node.get_host().await.unwrap();
        let host_port = node.get_host_port_ipv4(4566).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        match reqwest::get(format!("http://{}:{}/", host_ip, host_port)).await {
            Ok(response) if response.status().as_u16() == 200 => {
                println!("LocalStack is healthy!");
                return (node, host_ip, host_port);
            }
            Ok(response) => {
                eprintln!("LocalStack is not healthy. Status: {}", response.status());
            }
            Err(e) => {
                eprintln!("Failed to perform health check: {:?}", e);
            }
        }

        attempt_count += 1;
        println!("Retrying LocalStack Setup...");

        if attempt_count >= DEFAULT_RETRY_ATTEMPTS {
            panic!("Too Many Attempts");
        }
    }
}

async fn setup_aws_shared_config(host_ip: Host, host_port: u16) -> SdkConfig {
    let aws_access_key_id = std::env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID must be set");
    let aws_secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY must be set");
    let aws_region = std::env::var("AWS_REGION").expect("AWS_REGION must be set");
    let region_provider = Region::new(aws_region);
    let aws_endpoint_url = format!("http://{host_ip}:{host_port}");

    let creds = Credentials::new(aws_access_key_id, aws_secret_access_key, None, None, "test");

    aws_config::defaults(BehaviorVersion::v2024_03_28())
        .region(region_provider)
        .credentials_provider(creds)
        .endpoint_url(aws_endpoint_url.clone())
        .load()
        .await
}

/// MongoDb testcontainer
pub async fn mongodb_testcontainer_setup() -> (ContainerAsync<Mongo>, Box<dyn Database>) {
    const LOCAL_PORT: u16 = 27017; // Default MongoDB port
    let _ = pretty_env_logger::try_init();

    let logger = LoggingConsumer::new()
        .with_stdout_level(log::Level::Info)
        .with_stdout_level(log::Level::Debug)
        .with_stdout_level(log::Level::Error)
        .with_stdout_level(log::Level::Trace)
        .with_stdout_level(log::Level::Warn)
        .with_stderr_level(log::Level::Info)
        .with_stderr_level(log::Level::Debug)
        .with_stderr_level(log::Level::Error)
        .with_stderr_level(log::Level::Trace)
        .with_stderr_level(log::Level::Warn);

    let node = Mongo::default().with_log_consumer(logger).start().await.unwrap();

    let host_ip = node.get_host().await.unwrap();
    let host_port = node.get_host_port_ipv4(LOCAL_PORT.tcp()).await.unwrap();
    let connection_url = format!("mongodb://{host_ip}:{host_port}/");

    let mongo_config = MongoDbConfig { url: connection_url };
    let database = MongoDb::new(mongo_config).await;

    (node, Box::new(database) as Box<dyn Database>)
}
