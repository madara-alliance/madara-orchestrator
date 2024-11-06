use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use clap::Parser;
use color_eyre::eyre::eyre;
use da_client_interface::{DaClient, MockDaClient};
use httpmock::MockServer;
use prover_client_interface::{MockProverClient, ProverClient};
use settlement_client_interface::{MockSettlementClient, SettlementClient};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use url::Url;
use utils::cli::RunCmd;

use crate::alerts::Alerts;
use crate::config::{get_aws_config, Config, ProviderConfig, SnosConfig};
use crate::data_storage::{DataStorage, MockDataStorage};
use crate::database::{Database, MockDatabase};
use crate::queue::{MockQueueProvider, QueueProvider};
use crate::routes::{get_server_url, setup_server};
use crate::tests::common::{create_queues, create_sns_arn, drop_database};

// Inspiration : https://rust-unofficial.github.io/patterns/patterns/creational/builder.html
// TestConfigBuilder allows to heavily customise the global configs based on the test's requirement.
// Eg: We want to mock only the da client and leave rest to be as it is, use mock_da_client.

pub enum MockType {
    Server(Router),
    RpcUrl(Url),
    StarknetClient(Arc<JsonRpcClient<HttpTransport>>),
    DaClient(Box<dyn DaClient>),
    ProverClient(Box<dyn ProverClient>),
    SettlementClient(Box<dyn SettlementClient>),

    Alerts(Box<dyn Alerts>),
    Database(Box<dyn Database>),
    Queue(Box<dyn QueueProvider>),
    Storage(Box<dyn DataStorage>),
}

// By default, everything is on Dummy.
#[derive(Default)]
pub enum ConfigType {
    Mock(MockType),
    Actual,
    #[default]
    Dummy,
}

impl From<JsonRpcClient<HttpTransport>> for ConfigType {
    fn from(client: JsonRpcClient<HttpTransport>) -> Self {
        ConfigType::Mock(MockType::StarknetClient(Arc::new(client)))
    }
}

macro_rules! impl_mock_from {
    ($($mock_type:ty => $variant:ident),+) => {
        $(
            impl From<$mock_type> for ConfigType {
                fn from(client: $mock_type) -> Self {
                    ConfigType::Mock(MockType::$variant(Box::new(client)))
                }
            }
        )+
    };
}

impl_mock_from! {
    MockProverClient => ProverClient,
    MockDatabase => Database,
    MockDaClient => DaClient,
    MockQueueProvider => Queue,
    MockDataStorage => Storage,
    MockSettlementClient => SettlementClient
}

// TestBuilder for Config
pub struct TestConfigBuilder {
    /// The RPC url used by the starknet client
    starknet_rpc_url_type: ConfigType,
    /// The starknet client to get data from the node
    starknet_client_type: ConfigType,
    /// The DA client to interact with the DA layer
    da_client_type: ConfigType,
    /// The service that produces proof and registers it on chain
    prover_client_type: ConfigType,
    /// Settlement client
    settlement_client_type: ConfigType,

    /// Alerts client
    alerts_type: ConfigType,
    /// The database client
    database_type: ConfigType,
    /// Queue client
    queue_type: ConfigType,
    /// Storage client
    storage_type: ConfigType,
    /// API Service
    api_server_type: ConfigType,
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TestConfigBuilderReturns {
    pub starknet_server: Option<MockServer>,
    pub config: Arc<Config>,
    pub provider_config: Arc<ProviderConfig>,
    pub api_server_address: Option<SocketAddr>,
}

impl TestConfigBuilder {
    /// Create a new config
    pub fn new() -> TestConfigBuilder {
        TestConfigBuilder {
            starknet_rpc_url_type: ConfigType::default(),
            starknet_client_type: ConfigType::default(),
            da_client_type: ConfigType::default(),
            prover_client_type: ConfigType::default(),
            settlement_client_type: ConfigType::default(),
            database_type: ConfigType::default(),
            queue_type: ConfigType::default(),
            storage_type: ConfigType::default(),
            alerts_type: ConfigType::default(),
            api_server_type: ConfigType::default(),
        }
    }

    pub fn configure_rpc_url(mut self, starknet_rpc_url_type: ConfigType) -> TestConfigBuilder {
        self.starknet_rpc_url_type = starknet_rpc_url_type;
        self
    }

    pub fn configure_da_client(mut self, da_client_type: ConfigType) -> TestConfigBuilder {
        self.da_client_type = da_client_type;
        self
    }

    pub fn configure_settlement_client(mut self, settlement_client_type: ConfigType) -> TestConfigBuilder {
        self.settlement_client_type = settlement_client_type;
        self
    }

    pub fn configure_starknet_client(mut self, starknet_client_type: ConfigType) -> TestConfigBuilder {
        self.starknet_client_type = starknet_client_type;
        self
    }

    pub fn configure_prover_client(mut self, prover_client_type: ConfigType) -> TestConfigBuilder {
        self.prover_client_type = prover_client_type;
        self
    }

    pub fn configure_alerts(mut self, alert_option: ConfigType) -> TestConfigBuilder {
        self.alerts_type = alert_option;
        self
    }

    pub fn configure_storage_client(mut self, storage_client_option: ConfigType) -> TestConfigBuilder {
        self.storage_type = storage_client_option;
        self
    }

    pub fn configure_queue_client(mut self, queue_type: ConfigType) -> TestConfigBuilder {
        self.queue_type = queue_type;
        self
    }
    pub fn configure_database(mut self, database_type: ConfigType) -> TestConfigBuilder {
        self.database_type = database_type;
        self
    }

    pub fn configure_api_server(mut self, api_server_type: ConfigType) -> TestConfigBuilder {
        self.api_server_type = api_server_type;
        self
    }

    pub async fn build(self) -> TestConfigBuilderReturns {
        dotenvy::from_filename("../.env.test").expect("Failed to load the .env.test file");
        let run_cmd: RunCmd = RunCmd::parse();

        let run_cmd_clone = run_cmd.clone();

        let aws_config = &run_cmd_clone.aws_config;
        let provider_config = Arc::new(ProviderConfig::AWS(Box::new(get_aws_config(aws_config).await)));
        let server_config = run_cmd_clone.server;

        let snos_config = SnosConfig {
            rpc_url : run_cmd_clone.snos.rpc_for_snos.clone(),
            max_block_to_process : run_cmd_clone.snos.max_block_to_process,
            min_block_to_process : run_cmd_clone.snos.min_block_to_process,
        };
        
        use std::sync::Arc;

        let TestConfigBuilder {
            starknet_rpc_url_type,
            starknet_client_type,
            alerts_type,
            da_client_type,
            prover_client_type,
            settlement_client_type,
            database_type,
            queue_type,
            storage_type,
            api_server_type,
        } = self;

        let (starknet_rpc_url, starknet_client, starknet_server) =
            implement_client::init_starknet_client(starknet_rpc_url_type, starknet_client_type).await;

        // init alerts
        let alert_params = run_cmd.clone().validate_alert_params().map_err(|e| eyre!("Failed to validate alert params: {e}")).unwrap();
        let alerts = implement_client::init_alerts(alerts_type, &alert_params, provider_config.clone()).await;

        let da_params = run_cmd.clone().validate_da_params().unwrap();
        let da_client = implement_client::init_da_client(da_client_type, &da_params).await;

        let settlement_params = run_cmd.clone().validate_settlement_params().unwrap();
        let settlement_client =
            implement_client::init_settlement_client(settlement_client_type, &settlement_params
            ).await;

        let prover_params = run_cmd.clone().validate_prover_params().map_err(|e| eyre!("Failed to validate prover params: {e}")).unwrap();
        let prover_client = implement_client::init_prover_client(prover_client_type, &prover_params).await  ;

        
        // External Dependencies
        let data_storage_params = run_cmd.clone().validate_storage_params().map_err(|e| eyre!("Failed to validate storage params: {e}")).unwrap();
        let storage = implement_client::init_storage_client(storage_type, &data_storage_params, provider_config.clone()).await;

        let database_params = run_cmd.clone().validate_database_params().map_err(|e| eyre!("Failed to validate database params: {e}")).unwrap();
        let database = implement_client::init_database(database_type, &database_params).await;

        let queue_params = run_cmd.clone().validate_queue_params().map_err(|e| eyre!("Failed to validate queue params: {e}")).unwrap();
        let queue = implement_client::init_queue_client(queue_type, queue_params.clone()).await;
        // Deleting and Creating the queues in sqs.

        create_queues(provider_config.clone(), &queue_params).await.expect("Not able to delete and create the queues.");
        // Deleting the database
        drop_database(&database_params).await.expect("Unable to drop the database.");
        // Creating the SNS ARN
        create_sns_arn(provider_config.clone()).await.expect("Unable to create the sns arn");

        let config = Arc::new(Config::new(
            starknet_rpc_url,
            server_config,
            snos_config,
            starknet_client,
            da_client,
            prover_client,
            settlement_client,
            database,
            queue,
            storage,
            alerts,
        ));

        let api_server_address = implement_api_server(api_server_type, config.clone()).await;

        TestConfigBuilderReturns {
            starknet_server,
            config,
            provider_config: provider_config.clone(),
            api_server_address,
        }
    }
}

async fn implement_api_server(api_server_type: ConfigType, config: Arc<Config>) -> Option<SocketAddr> {
    match api_server_type {
        ConfigType::Mock(client) => {
            if let MockType::Server(router) = client {
                let (api_server_url, listener) = get_server_url(config.server_config()).await;
                let app = Router::new().merge(router);

                tokio::spawn(async move {
                    axum::serve(listener, app).await.expect("Failed to start axum server");
                });

                Some(api_server_url)
            } else {
                panic!(concat!("Mock client is not a ", stringify!($client_type)));
            }
        }
        ConfigType::Actual => Some(setup_server(config.clone()).await),
        ConfigType::Dummy => None,
    }
}

pub mod implement_client {
    use std::sync::Arc;

    use da_client_interface::{DaClient, MockDaClient};
    use httpmock::MockServer;
    use prover_client_interface::{MockProverClient, ProverClient};
    use settlement_client_interface::{MockSettlementClient, SettlementClient};
    use starknet::providers::jsonrpc::HttpTransport;
    use starknet::providers::{JsonRpcClient, Url};
    use utils::cli::alert::AlertParams;
    use utils::cli::da::DaParams;
    use utils::cli::database::DatabaseParams;
    use utils::cli::prover::ProverParams;
    use utils::cli::queue::QueueParams;
    use utils::cli::settlement::SettlementParams;
    use utils::cli::storage::StorageParams;

    use super::{ConfigType, MockType};
    use crate::alerts::{Alerts, MockAlerts};
    use crate::config::{
        build_alert_client, build_da_client, build_database_client, build_prover_service, build_queue_client,
        build_settlement_client, ProviderConfig,
    };
    use crate::data_storage::{DataStorage, MockDataStorage};
    use crate::database::{Database, MockDatabase};
    use crate::queue::{MockQueueProvider, QueueProvider};
    use crate::tests::common::get_storage_client;

    macro_rules! implement_mock_client_conversion {
        ($client_type:ident, $mock_variant:ident) => {
            impl From<MockType> for Box<dyn $client_type> {
                fn from(client: MockType) -> Self {
                    if let MockType::$mock_variant(service_client) = client {
                        service_client
                    } else {
                        panic!(concat!("Mock client is not a ", stringify!($client_type)));
                    }
                }
            }
        };
    }

    implement_mock_client_conversion!(DataStorage, Storage);
    implement_mock_client_conversion!(QueueProvider, Queue);
    implement_mock_client_conversion!(Database, Database);
    implement_mock_client_conversion!(Alerts, Alerts);
    implement_mock_client_conversion!(ProverClient, ProverClient);
    implement_mock_client_conversion!(SettlementClient, SettlementClient);
    implement_mock_client_conversion!(DaClient, DaClient);

    pub(crate) async fn init_da_client(service: ConfigType, da_params: &DaParams) -> Box<dyn DaClient> {
        match service {
            ConfigType::Mock(client) => client.into(),
            ConfigType::Actual => build_da_client(da_params).await,
            ConfigType::Dummy => Box::new(MockDaClient::new()),
        }
    }

    pub(crate) async fn init_settlement_client(
        service: ConfigType,
        settlement_cfg : &SettlementParams
    ) -> Box<dyn SettlementClient> {
        match service {
            ConfigType::Mock(client) => client.into(),
            ConfigType::Actual => {
                build_settlement_client(settlement_cfg).await.expect("Failed to initialise settlement_client")
            }
            ConfigType::Dummy => Box::new(MockSettlementClient::new()),
        }
    }

    pub(crate) async fn init_prover_client(
        service: ConfigType,
        prover_params: &ProverParams,
    ) -> Box<dyn ProverClient> {
        match service {
            ConfigType::Mock(client) => client.into(),
            ConfigType::Actual => build_prover_service(prover_params),
            ConfigType::Dummy => Box::new(MockProverClient::new()),
        }
    }

    pub(crate) async fn init_alerts(
        service: ConfigType,
        alert_params: &AlertParams,
        provider_config: Arc<ProviderConfig>,
    ) -> Box<dyn Alerts> {
        match service {
            ConfigType::Mock(client) => client.into(),
            ConfigType::Actual => build_alert_client(alert_params, provider_config).await,
            ConfigType::Dummy => Box::new(MockAlerts::new()),
        }
    }

    pub(crate) async fn init_storage_client(
        service: ConfigType,
        storage_cfg : &StorageParams,
        provider_config: Arc<ProviderConfig>,
    ) -> Box<dyn DataStorage> {
        match service {
            ConfigType::Mock(client) => client.into(),
            ConfigType::Actual => {
                match storage_cfg {
                    StorageParams::AWSS3(aws_s3_params) => {
                        let storage = get_storage_client(aws_s3_params, provider_config).await;
                        storage.as_ref().build_test_bucket(&aws_s3_params.bucket_name).await.unwrap();  
                        storage
                    }
                }
            }
            ConfigType::Dummy => Box::new(MockDataStorage::new()),
        }
    }

    pub(crate) async fn init_queue_client(service: ConfigType, queue_params: QueueParams) -> Box<dyn QueueProvider> {
        match service {
            ConfigType::Mock(client) => client.into(),
            ConfigType::Actual => build_queue_client(&queue_params),
            ConfigType::Dummy => Box::new(MockQueueProvider::new()),
        }
    }

    pub(crate) async fn init_database(
        service: ConfigType,
        database_params: &DatabaseParams,
    ) -> Box<dyn Database> {
        match service {
            ConfigType::Mock(client) => client.into(),
            ConfigType::Actual => build_database_client(&database_params).await,
            ConfigType::Dummy => Box::new(MockDatabase::new()),
        }
    }

    pub(crate) async fn init_starknet_client(
        starknet_rpc_url_type: ConfigType,
        service: ConfigType,
    ) -> (Url, Arc<JsonRpcClient<HttpTransport>>, Option<MockServer>) {
        fn get_rpc_url() -> (Url, Option<MockServer>) {
            let server = MockServer::start();
            let port = server.port();
            let rpc_url = Url::parse(format!("http://localhost:{}", port).as_str()).expect("Failed to parse URL");
            (rpc_url, Some(server))
        }

        fn get_provider(rpc_url: &Url) -> Arc<JsonRpcClient<HttpTransport>> {
            Arc::new(JsonRpcClient::new(HttpTransport::new(rpc_url.clone())))
        }

        let (rpc_url, server) = match starknet_rpc_url_type {
            ConfigType::Mock(url_type) => {
                if let MockType::RpcUrl(starknet_rpc_url) = url_type {
                    (starknet_rpc_url, None)
                } else {
                    panic!("Mock Rpc URL is not an URL");
                }
            }
            ConfigType::Actual | ConfigType::Dummy => get_rpc_url(),
        };

        let starknet_client = match service {
            ConfigType::Mock(client) => {
                if let MockType::StarknetClient(starknet_client) = client {
                    starknet_client
                } else {
                    panic!("Mock client is not a Starknet Client");
                }
            }
            ConfigType::Actual | ConfigType::Dummy => get_provider(&rpc_url),
        };

        (rpc_url, starknet_client, server)
    }
}
