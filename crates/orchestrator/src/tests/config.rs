use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::config::{build_da_client, build_prover_service, build_settlement_client, Config};
use crate::data_storage::{DataStorage, DataStorageConfig, MockDataStorage};
use crate::queue::job_queue::{JOB_PROCESSING_QUEUE, JOB_VERIFICATION_QUEUE};
use da_client_interface::DaClient;
use httpmock::MockServer;

use prover_client_interface::ProverClient;
use settlement_client_interface::SettlementClient;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Url};
use testcontainers::core::logs::consumer::logging_consumer::LoggingConsumer;
use tokio::time::sleep;
use url::Host;
use utils::env_utils::get_env_var_or_panic;
use utils::settings::default::DefaultSettingsProvider;

use crate::database::mongodb::config::MongoDbConfig;
use crate::database::mongodb::MongoDb;
use crate::database::{Database, MockDatabase};
use crate::queue::sqs::SqsQueue;
use crate::queue::QueueProvider;

// Inspiration : https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
// TestConfigBuilder allows to heavily customise the global configs based on the test's requirement.
// Eg: We want to mock only the da client and leave rest to be as it is, use mock_da_client.

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

    // Storing for Data Storage client
    // These are need to be kept in scope to keep the Server alive
    database_node: Option<ContainerAsync<Mongo>>,
    data_storage_node: Option<ContainerAsync<LocalStack>>,
    data_storage_client: Option<aws_sdk_s3::Client>,
    queue_node: Option<ContainerAsync<LocalStack>>,
    queue_client: Option<sqs::Client>,
}

pub struct TestConfigBuildReturn {
    pub mock_server: MockServer,
    pub data_storage_node: Option<ContainerAsync<LocalStack>>,
    pub data_storage_client: Option<aws_sdk_s3::Client>,
    pub database_node: Option<ContainerAsync<Mongo>>,
    pub queue_client: Option<sqs::Client>,
    pub queue_node: Option<ContainerAsync<LocalStack>>,
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

            database_node: None,
            data_storage_node: None,
            data_storage_client: None,
            queue_node: None,
            queue_client: None,
        }
    }

    pub async fn testcontainer_s3_data_storage(mut self) -> TestConfigBuilder {
        let (node, storage_client, client) = s3_testcontainer_setup().await;
        self.data_storage_node = Some(node);
        self.data_storage_client = Some(client);
        self.storage = Some(storage_client);
        self
    }

    pub async fn testcontainer_sqs_data_storage(mut self) -> TestConfigBuilder {
        let (node, queue_client, client) = sqs_testcontainer_setup().await;
        self.queue = Some(queue_client);
        self.queue_client = Some(client);
        self.queue_node = Some(node);
        self
    }

    pub async fn testcontainer_mongo_database(mut self) -> TestConfigBuilder {
        let (node, database) = mongodb_testcontainer_setup().await;
        self.database = Some(database);
        self.database_node = Some(node);
        self
    }

    pub fn mock_da_client(mut self, da_client: Box<dyn DaClient>) -> TestConfigBuilder {
        self.da_client = Some(da_client);
        self
    }

    pub fn mock_db_client(mut self, db_client: Box<dyn Database>) -> TestConfigBuilder {
        self.database = Some(db_client);
        self
    }

    pub fn mock_settlement_client(mut self, settlement_client: Box<dyn SettlementClient>) -> TestConfigBuilder {
        self.settlement_client = Some(settlement_client);
        self
    }

    pub fn mock_starknet_client(mut self, starknet_client: Arc<JsonRpcClient<HttpTransport>>) -> TestConfigBuilder {
        self.starknet_client = Some(starknet_client);
        self
    }

    pub fn mock_prover_client(mut self, prover_client: Box<dyn ProverClient>) -> TestConfigBuilder {
        self.prover_client = Some(prover_client);
        self
    }

    pub fn mock_storage_client(mut self, storage_client: Box<dyn DataStorage>) -> TestConfigBuilder {
        self.storage = Some(storage_client);
        self
    }

    pub fn mock_queue(mut self, queue: Box<dyn QueueProvider>) -> TestConfigBuilder {
        self.queue = Some(queue);
        self
    }

    pub async fn build(mut self) -> TestConfigBuildReturn {
        dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

        let server = MockServer::start();
        let settings_provider = DefaultSettingsProvider {};

        // init database
        if self.database.is_none() {
            self.database = Some(Box::new(MockDatabase::new()));
        }

        // init the DA client
        if self.da_client.is_none() {
            self.da_client = Some(build_da_client().await);
        }

        // init the Settings client
        if self.settlement_client.is_none() {
            self.settlement_client = Some(build_settlement_client(&settings_provider).await);
        }

        // init the storage client
        if self.storage.is_none() {
            self.storage = Some(Box::new(MockDataStorage::new()));

            // self.storage = Some(MockDataStorage::new());
            // match get_env_var_or_panic("DATA_STORAGE").as_str() {
            //     "s3" => self
            //         .storage
            //         .as_ref()
            //         .unwrap()
            //         .build_test_bucket(&get_env_var_or_panic("AWS_S3_BUCKET_NAME"))
            //         .await
            //         .unwrap(),
            //     _ => panic!("Unsupported Storage Client"),
            // }
        }

        // Deleting and Creating the queues in sqs.
        // create_sqs_queues().await.expect("Not able to delete and create the queues.");

        // Deleting the database
        // drop_database().await.expect("Unable to drop the database.");

        let config = Arc::new(Config::new(
            self.starknet_client.unwrap_or_else(|| {
                let provider = JsonRpcClient::new(HttpTransport::new(
                    Url::parse(format!("http://localhost:{}", server.port()).as_str()).expect("Failed to parse URL"),
                ));
                Arc::new(provider)
            }),
            self.da_client.unwrap(),
            self.prover_client.unwrap_or_else(|| build_prover_service(&settings_provider)),
            self.settlement_client.unwrap(),
            self.database.unwrap(),
            self.queue.unwrap_or_else(|| Box::new(SqsQueue::new_from_env())),
            self.storage.unwrap(),
        ));

        // drop_database().await.unwrap();

        TestConfigBuildReturn {
            mock_server: server,
            data_storage_node: self.data_storage_node,
            data_storage_client: self.data_storage_client,
            database_node: self.database_node,
            queue_client: self.queue_client,
            queue_node: self.queue_node,
            config,
        }
    }
}

/// LocalStack (s3 and sqs) & MongoDb Setup using TestContainers ////
use super::common::testcontainer_setups::LocalStack;
use crate::data_storage::aws_s3::config::AWSS3Config;
use crate::data_storage::aws_s3::AWSS3;
use crate::tests::common::testcontainer_setups::Mongo;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3 as s3;
use aws_sdk_sqs as sqs;
use aws_sdk_sqs::config::Credentials;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};

/// Localstack SQS testcontainer
pub async fn sqs_testcontainer_setup() -> (ContainerAsync<LocalStack>, Box<dyn QueueProvider>, sqs::Client) {
    dotenvy::from_filename("../.env.test").unwrap();
    let _ = pretty_env_logger::try_init();

    let mut node : ContainerAsync<LocalStack> ;
    let mut host_ip : Host;
    let mut host_port : u16;

    let mut attempt_count : u16 = 1;

    loop {
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

        let i_node = LocalStack::default().with_log_consumer(logger).start().await.unwrap();
        let i_host_ip = i_node.get_host().await.unwrap();
        let i_host_port = i_node.get_host_port_ipv4(4566).await.unwrap();

        sleep(Duration::from_secs(3)).await;
        // curl
        match reqwest::get(format!("http://{}:{}/", i_host_ip, i_host_port)).await {
            Ok(response) => {
                if response.status().as_u16() == 200 {
                    println!("LocalStack is healthy!");
                    node = i_node;
                    host_ip = i_host_ip;
                    host_port = i_host_port;
                    break; // Exit the loop if the health check is successful
                } else {
                    eprintln!("LocalStack is not healthy. Status: {}", response.status());
                    attempt_count+=1;
                    if attempt_count == 10 {
                        panic!("Too Many Attempts");
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to perform health check: {:?}", e);
            }
        }
      
    }
    
    let aws_access_key_id = get_env_var_or_panic("AWS_ACCESS_KEY_ID");
    let aws_secret_access_key = get_env_var_or_panic("AWS_SECRET_ACCESS_KEY");
    let aws_region = get_env_var_or_panic("AWS_REGION");
    let region_provider = Region::new(aws_region);
    let aws_endpoint_url = format!("http://{host_ip}:{host_port}");

    println!("{:?} SQS {:?}", SystemTime::now(), host_port);

    let creds = Credentials::new(aws_access_key_id, aws_secret_access_key, None, None, "test");
    let config = aws_config::defaults(BehaviorVersion::v2024_03_28())
        .region(region_provider)
        .credentials_provider(creds)
        .endpoint_url(aws_endpoint_url.clone())
        .load()
        .await;

    let client = sqs::Client::new(&config);

    // Queue creation
    let processing_queue_output =
        client.create_queue().queue_name(JOB_PROCESSING_QUEUE.to_string()).send().await.unwrap();
    let _verification_queue_output =
        client.create_queue().queue_name(JOB_VERIFICATION_QUEUE.to_string()).send().await.unwrap();

    let queue_host_url = transform_url(processing_queue_output.queue_url().unwrap(), &host_port);

    let sqs_queue = SqsQueue::new(queue_host_url.to_string());

    (node, Box::new(sqs_queue) as Box<dyn QueueProvider>, client)
}

fn transform_url(input: &str, host_port: &u16) -> String {
    let parsed_url = Url::parse(input).expect("Failed to parse URL");
    let host = parsed_url.host_str().unwrap();
    let first_path_segment = parsed_url.path_segments().and_then(|mut segments| segments.next()).unwrap_or("");
    format!("http://{}:{}/{}", host, host_port, first_path_segment)
}

/// LocalStack S3 testcontainer
pub async fn s3_testcontainer_setup() -> (ContainerAsync<LocalStack>, Box<dyn DataStorage>, s3::Client) {
    dotenvy::from_filename("../.env.test").unwrap();

    // let (tx, rx) = std::sync::mpsc::sync_channel(1);

    let _ = pretty_env_logger::try_init();

    let mut node : ContainerAsync<LocalStack> ;
    let mut host_ip : Host;
    let mut host_port : u16;

    let mut attempt_count : u16 = 1;

    loop {
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

        let i_node = LocalStack::default().with_log_consumer(logger).start().await.unwrap();
        let i_host_ip = i_node.get_host().await.unwrap();
        let i_host_port = i_node.get_host_port_ipv4(4566).await.unwrap();
    
        sleep(Duration::from_secs(3)).await;
        // curl
        match reqwest::get(format!("http://{}:{}/", i_host_ip, i_host_port)).await {
            Ok(response) => {
                if response.status().as_u16() == 200 {
                    println!("LocalStack is healthy!");
                    node = i_node;
                    host_ip = i_host_ip;
                    host_port = i_host_port;
                    break; // Exit the loop if the health check is successful
                } else {
                    eprintln!("LocalStack is not healthy. Status: {}", response.status());
                    attempt_count+=1;
                    if attempt_count == 10 {
                        panic!("Too Many Attempts");
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to perform health check: {:?}", e);
            }
        }

      
    }

    
    let aws_access_key_id = get_env_var_or_panic("AWS_ACCESS_KEY_ID");
    let aws_secret_access_key = get_env_var_or_panic("AWS_SECRET_ACCESS_KEY");
    let aws_region = get_env_var_or_panic("AWS_REGION");
    let region_provider = Region::new(aws_region);
    let aws_endpoint_url = format!("http://{host_ip}:{host_port}");

    println!("{:?} S3 {:?}", SystemTime::now(), host_port);

    let aws_s3_bucket_name = get_env_var_or_panic("AWS_S3_BUCKET_NAME");

    // Set up AWS client
    let creds = Credentials::new(aws_access_key_id, aws_secret_access_key, None, None, "test");

    let config = aws_sdk_s3::config::Builder::default()
        .behavior_version(BehaviorVersion::v2024_03_28())
        .region(region_provider)
        .credentials_provider(creds)
        .endpoint_url(aws_endpoint_url.clone())
        .force_path_style(true)
        .build();

    let client = s3::Client::from_conf(config);

    client.create_bucket().bucket(aws_s3_bucket_name.clone()).send().await.unwrap();

    let aws_config = aws_config::load_from_env().await.into_builder().endpoint_url(aws_endpoint_url.as_str()).build();

    let storage_client = AWSS3::new(AWSS3Config::new_from_env(), &aws_config);
    (node, Box::new(storage_client) as Box<dyn DataStorage>, client)
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

    println!("{:?} MONGO {:?}", SystemTime::now(), host_port);

    let mongo_config = MongoDbConfig { url: connection_url };
    let database = MongoDb::new(mongo_config).await;

    (node, Box::new(database) as Box<dyn Database>)
}
