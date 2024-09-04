use std::sync::Arc;

use httpmock::MockServer;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Url};

use da_client_interface::{DaClient, MockDaClient};
use prover_client_interface::{MockProverClient, ProverClient};
use settlement_client_interface::{MockSettlementClient, SettlementClient};
use utils::env_utils::get_env_var_or_panic;
use utils::settings::default::DefaultSettingsProvider;

use crate::alerts::{Alerts, MockAlerts};
use crate::config::{
    build_alert_client, build_da_client, build_database_client, build_prover_service, build_queue_client,
    build_settlement_client, Config,
};
use crate::data_storage::{DataStorage, MockDataStorage};
use crate::database::{Database, MockDatabase};
use crate::queue::{MockQueueProvider, QueueProvider};
use crate::tests::common::{create_sns_arn, create_sqs_queues, drop_database};

use super::common::get_storage_client;

// Inspiration : https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
// TestConfigBuilder allows to heavily customise the global configs based on the test's requirement.
// Eg: We want to mock only the da client and leave rest to be as it is, use mock_da_client.

pub enum ClientType {
    // Internal Clients
    StarknetClient(Arc<JsonRpcClient<HttpTransport>>),
    DaClient(Box<dyn DaClient>),
    ProverClient(Box<dyn ProverClient>),
    SettlementClient(Box<dyn SettlementClient>),
    Alerts(Box<dyn Alerts>),

    // External Clients
    Database(Box<dyn Database>),
    Queue(Box<dyn QueueProvider>),
    Storage(Box<dyn DataStorage>),
}

// By default, everything is on Dummy.
pub enum ClientValue {
    MockBySelf(ClientType),
    Actual,
    Dummy,
}

impl From<JsonRpcClient<HttpTransport>> for ClientValue {
    fn from(client: JsonRpcClient<HttpTransport>) -> Self {
        ClientValue::MockBySelf(ClientType::StarknetClient(Arc::new(client)))
    }
}

impl From<MockProverClient> for ClientValue {
    fn from(client: MockProverClient) -> Self {
        ClientValue::MockBySelf(ClientType::ProverClient(Box::new(client)))
    }
}

impl From<MockDatabase> for ClientValue {
    fn from(client: MockDatabase) -> Self {
        ClientValue::MockBySelf(ClientType::Database(Box::new(client)))
    }
}

impl From<MockDaClient> for ClientValue {
    fn from(client: MockDaClient) -> Self {
        ClientValue::MockBySelf(ClientType::DaClient(Box::new(client)))
    }
}

impl From<MockQueueProvider> for ClientValue {
    fn from(client: MockQueueProvider) -> Self {
        ClientValue::MockBySelf(ClientType::Queue(Box::new(client)))
    }
}

impl From<MockDataStorage> for ClientValue {
    fn from(client: MockDataStorage) -> Self {
        ClientValue::MockBySelf(ClientType::Storage(Box::new(client)))
    }
}

impl From<MockSettlementClient> for ClientValue {
    fn from(client: MockSettlementClient) -> Self {
        ClientValue::MockBySelf(ClientType::SettlementClient(Box::new(client)))
    }
}

// impl From<Box<dyn SettlementClient + Send + Sync>> for ClientValue {
//     fn from(client: Box<dyn SettlementClient + Send + Sync>) -> Self {
//         ClientValue::MockBySelf(ClientType::SettlementClient(Arc::new(client)))
//     }
// }

// TestBuilder for Config
pub struct TestConfigBuilder {
    server: MockServer,
    // Internal Clients
    /// The starknet client to get data from the node
    starknet_client_option: ClientValue,
    /// Alerts client
    alerts_option: ClientValue,
    /// The DA client to interact with the DA layer
    da_client_option: ClientValue,
    /// The service that produces proof and registers it on chain
    prover_client_option: ClientValue,
    /// Settlement client
    settlement_client_option: ClientValue,

    // External Clients
    /// The database client
    database_option: ClientValue,
    /// Queue client
    queue_option: ClientValue,
    /// Storage client
    storage_option: ClientValue,
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TestConfigBuilderReturns {
    pub server: MockServer,
    pub config: Arc<Config>,
}
impl TestConfigBuilder {
    /// Create a new config
    pub fn new() -> TestConfigBuilder {
        TestConfigBuilder {
            server: MockServer::start(),
            starknet_client_option: ClientValue::Dummy,
            da_client_option: ClientValue::Dummy,
            prover_client_option: ClientValue::Dummy,
            settlement_client_option: ClientValue::Dummy,
            database_option: ClientValue::Dummy,
            queue_option: ClientValue::Dummy,
            storage_option: ClientValue::Dummy,
            alerts_option: ClientValue::Dummy,
        }
    }

    pub fn configure_da_client(mut self, da_client_option: ClientValue) -> TestConfigBuilder {
        self.da_client_option = da_client_option;
        self
    }

    pub fn configure_settlement_client(mut self, settlement_client_option: ClientValue) -> TestConfigBuilder {
        self.settlement_client_option = settlement_client_option;
        self
    }

    pub fn configure_starknet_client(mut self, starknet_client_option: ClientValue) -> TestConfigBuilder {
        self.starknet_client_option = starknet_client_option;
        self
    }

    pub fn configure_prover_client(mut self, prover_client_option: ClientValue) -> TestConfigBuilder {
        self.prover_client_option = prover_client_option;
        self
    }

    pub fn configure_alerts(mut self, alert_option: ClientValue) -> TestConfigBuilder {
        self.alerts_option = alert_option;
        self
    }

    pub fn configure_storage_client(mut self, storage_client_option: ClientValue) -> TestConfigBuilder {
        self.storage_option = storage_client_option;
        self
    }

    pub fn configure_queue_client(mut self, queue_option: ClientValue) -> TestConfigBuilder {
        self.queue_option = queue_option;
        self
    }
    pub fn configure_database(mut self, database_option: ClientValue) -> TestConfigBuilder {
        self.database_option = database_option;
        self
    }

    pub async fn build(self) -> TestConfigBuilderReturns {
        dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");
        use std::sync::Arc;

        let TestConfigBuilder {
            server,
            starknet_client_option,
            alerts_option,
            da_client_option,
            prover_client_option,
            settlement_client_option,
            database_option,
            queue_option,
            storage_option,
        } = self;

        // Usage in your code
        let port: u16 = server.port();
        let starknet_client = init_starknet_client(starknet_client_option, port).await;
        let alerts = init_alerts(alerts_option).await;
        let da_client = init_da_client(da_client_option).await;

        let settlement_client = init_settlement_client(settlement_client_option).await;

        let prover_client = init_prover_client(prover_client_option).await;

        // External Dependencies
        let storage = init_storage_client(storage_option).await;
        let database = init_database(database_option).await;
        let queue = init_queue_client(queue_option).await;
        // Deleting and Creating the queues in sqs.
        create_sqs_queues().await.expect("Not able to delete and create the queues.");
        // Deleting the database
        drop_database().await.expect("Unable to drop the database.");
        // Creating the SNS ARN
        create_sns_arn().await.expect("Unable to create the sns arn");

        let config = Arc::new(Config::new(
            starknet_client,
            da_client,
            prover_client,
            settlement_client,
            database,
            queue,
            storage,
            alerts,
        ));

        TestConfigBuilderReturns { server, config }
    }
}

async fn init_da_client(service: ClientValue) -> Box<dyn DaClient> {
    match service {
        ClientValue::MockBySelf(client) => {
            if let ClientType::DaClient(da_client) = client {
                da_client
            } else {
                panic!("MockBySelf client is not a DaClient");
            }
        }
        ClientValue::Actual => build_da_client().await,
        ClientValue::Dummy => Box::new(MockDaClient::new()),
    }
}

async fn init_settlement_client(service: ClientValue) -> Box<dyn SettlementClient> {
    let settings_provider = DefaultSettingsProvider {};
    match service {
        ClientValue::MockBySelf(client) => {
            if let ClientType::SettlementClient(settlement_client) = client {
                settlement_client
            } else {
                panic!("MockBySelf client is not a SettlementClient");
            }
        }
        ClientValue::Actual => build_settlement_client(&settings_provider).await,
        ClientValue::Dummy => Box::new(MockSettlementClient::new()),
    }
}

async fn init_starknet_client(service: ClientValue, port: u16) -> Arc<JsonRpcClient<HttpTransport>> {
    let provider = || {
        JsonRpcClient::new(HttpTransport::new(
            Url::parse(format!("http://localhost:{}", port).as_str()).expect("Failed to parse URL"),
        ))
    };

    match service {
        ClientValue::MockBySelf(client) => {
            if let ClientType::StarknetClient(starknet_client) = client {
                starknet_client
            } else {
                panic!("MockBySelf client is not a StarknetClient");
            }
        }
        ClientValue::Actual => Arc::new(provider()),
        ClientValue::Dummy => Arc::new(provider()),
    }
}

async fn init_prover_client(service: ClientValue) -> Box<dyn ProverClient> {
    let settings_provider = DefaultSettingsProvider {};
    match service {
        ClientValue::MockBySelf(client) => {
            if let ClientType::ProverClient(prover_client) = client {
                prover_client
            } else {
                panic!("MockBySelf client is not a ProverClient");
            }
        }
        ClientValue::Actual => build_prover_service(&settings_provider),
        ClientValue::Dummy => Box::new(MockProverClient::new()),
    }
}

async fn init_alerts(service: ClientValue) -> Box<dyn Alerts> {
    match service {
        ClientValue::MockBySelf(client) => {
            if let ClientType::Alerts(alerts) = client {
                alerts
            } else {
                panic!("MockBySelf client is not an Alerts");
            }
        }
        ClientValue::Actual => build_alert_client().await,
        ClientValue::Dummy => Box::new(MockAlerts::new()),
    }
}

async fn init_storage_client(service: ClientValue) -> Box<dyn DataStorage> {
    // let aws_config = aws_config::load_from_env().await;
    match service {
        ClientValue::MockBySelf(client) => {
            if let ClientType::Storage(storage) = client {
                storage
            } else {
                panic!("MockBySelf client is not a Storage");
            }
        }
        ClientValue::Actual => {
            let storage = get_storage_client().await;
            match get_env_var_or_panic("DATA_STORAGE").as_str() {
                "s3" => storage.as_ref().build_test_bucket(&get_env_var_or_panic("AWS_S3_BUCKET_NAME")).await.unwrap(),
                _ => panic!("Unsupported Storage Client"),
            }
            storage
        }
        ClientValue::Dummy => Box::new(MockDataStorage::new()),
    }
}

async fn init_queue_client(service: ClientValue) -> Box<dyn QueueProvider> {
    let aws_config = aws_config::load_from_env().await;
    match service {
        ClientValue::MockBySelf(client) => {
            if let ClientType::Queue(queue) = client {
                queue
            } else {
                panic!("MockBySelf client is not a Queue");
            }
        }
        ClientValue::Actual => build_queue_client(&aws_config),
        ClientValue::Dummy => Box::new(MockQueueProvider::new()),
    }
}

async fn init_database(service: ClientValue) -> Box<dyn Database> {
    match service {
        ClientValue::MockBySelf(client) => {
            if let ClientType::Database(database) = client {
                database
            } else {
                panic!("MockBySelf client is not a Database");
            }
        }
        ClientValue::Actual => build_database_client().await,
        ClientValue::Dummy => Box::new(MockDatabase::new()),
    }
}
