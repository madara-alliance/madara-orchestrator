use std::sync::Arc;

use crate::data_storage::aws_s3::config::{AWSS3Config, AWSS3ConfigType};
use crate::data_storage::aws_s3::AWSS3;
use crate::data_storage::{DataStorage, DataStorageConfig};
use arc_swap::{ArcSwap, Guard};
use da_client_interface::{DaClient, DaConfig};
use dotenvy::dotenv;
use ethereum_da_client::config::EthereumDaConfig;
use ethereum_settlement_client::EthereumSettlementClient;
use prover_client_interface::ProverClient;
use settlement_client_interface::SettlementClient;
use sharp_service::SharpProverService;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Url};
use starknet_settlement_client::StarknetSettlementClient;
use tokio::sync::OnceCell;
use utils::env_utils::get_env_var_or_panic;
use utils::settings::default::DefaultSettingsProvider;
use utils::settings::SettingsProvider;

use crate::database::mongodb::config::MongoDbConfig;
use crate::database::mongodb::MongoDb;
use crate::database::{Database, DatabaseConfig};
use crate::queue::sqs::SqsQueue;
use crate::queue::QueueProvider;

/// The app config. It can be accessed from anywhere inside the service
/// by calling `config` function.
pub struct Config {
    /// The starknet client to get data from the node
    starknet_client: Arc<JsonRpcClient<HttpTransport>>,
    /// The DA client to interact with the DA layer
    da_client: Box<dyn DaClient>,
    /// The service that produces proof and registers it onchain
    prover_client: Box<dyn ProverClient>,
    /// Settlement client
    settlement_client: Box<dyn SettlementClient>,
    /// The database client
    database: Box<dyn Database>,
    /// Queue client
    queue: Box<dyn QueueProvider>,
    /// Storage client
    storage: Box<dyn DataStorage>,
}

/// Initializes the app config
pub async fn init_config() -> Config {
    dotenv().ok();

    // init starknet client
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse(get_env_var_or_panic("MADARA_RPC_URL").as_str()).expect("Failed to parse URL"),
    ));

    // init database
    let database = Box::new(MongoDb::new(MongoDbConfig::new_from_env()).await);

    // init the queue
    let queue = Box::new(SqsQueue::new_from_env());

    let da_client = build_da_client().await;

    let settings_provider = DefaultSettingsProvider {};
    let settlement_client = build_settlement_client(&settings_provider).await;
    let prover_client = build_prover_service(&settings_provider);

    let storage_client = build_storage_client().await;

    Config::new(Arc::new(provider), da_client, prover_client, settlement_client, database, queue, storage_client)
}

impl Config {
    /// Create a new config
    pub fn new(
        starknet_client: Arc<JsonRpcClient<HttpTransport>>,
        da_client: Box<dyn DaClient>,
        prover_client: Box<dyn ProverClient>,
        settlement_client: Box<dyn SettlementClient>,
        database: Box<dyn Database>,
        queue: Box<dyn QueueProvider>,
        storage: Box<dyn DataStorage>,
    ) -> Self {
        Self { starknet_client, da_client, prover_client, settlement_client, database, queue, storage }
    }

    /// Returns the starknet client
    pub fn starknet_client(&self) -> &Arc<JsonRpcClient<HttpTransport>> {
        &self.starknet_client
    }

    /// Returns the DA client
    pub fn da_client(&self) -> &dyn DaClient {
        self.da_client.as_ref()
    }

    /// Returns the proving service
    pub fn prover_client(&self) -> &dyn ProverClient {
        self.prover_client.as_ref()
    }

    /// Returns the settlement client
    pub fn settlement_client(&self) -> &dyn SettlementClient {
        self.settlement_client.as_ref()
    }

    /// Returns the database client
    pub fn database(&self) -> &dyn Database {
        self.database.as_ref()
    }

    /// Returns the queue provider
    pub fn queue(&self) -> &dyn QueueProvider {
        self.queue.as_ref()
    }

    /// Returns the storage provider
    pub fn storage(&self) -> &dyn DataStorage {
        self.storage.as_ref()
    }
}

/// The app config. It can be accessed from anywhere inside the service.
/// It's initialized only once.
/// We are using `ArcSwap` as it allow us to replace the new `Config` with
/// a new one which is required when running test cases. This approach was
/// inspired from here - https://github.com/matklad/once_cell/issues/127
pub static CONFIG: OnceCell<ArcSwap<Config>> = OnceCell::const_new();

/// Returns the app config. Initializes if not already done.
pub async fn config() -> Guard<Arc<Config>> {
    let cfg = CONFIG.get_or_init(|| async { ArcSwap::from_pointee(init_config().await) }).await;
    cfg.load()
}

/// OnceCell only allows us to initialize the config once and that's how it should be on production.
/// However, when running tests, we often want to reinitialize because we want to clear the DB and
/// set it up again for reuse in new tests. By calling `config_force_init` we replace the already
/// stored config inside `ArcSwap` with the new configuration and pool settings.
#[cfg(test)]
pub async fn config_force_init(config: Config) {
    match CONFIG.get() {
        Some(arc) => arc.store(Arc::new(config)),
        None => {
            CONFIG.get_or_init(|| async { ArcSwap::from_pointee(config) }).await;
        }
    }
}

/// Builds the DA client based on the environment variable DA_LAYER
pub async fn build_da_client() -> Box<dyn DaClient + Send + Sync> {
    match get_env_var_or_panic("DA_LAYER").as_str() {
        "ethereum" => {
            let config = EthereumDaConfig::new_from_env();
            Box::new(config.build_client().await)
        }
        _ => panic!("Unsupported DA layer"),
    }
}

/// Builds the prover service based on the environment variable PROVER_SERVICE
pub fn build_prover_service(settings_provider: &impl SettingsProvider) -> Box<dyn ProverClient> {
    match get_env_var_or_panic("PROVER_SERVICE").as_str() {
        "sharp" => Box::new(SharpProverService::with_settings(settings_provider)),
        _ => panic!("Unsupported prover service"),
    }
}

/// Builds the settlement client depending on the env variable SETTLEMENT_LAYER
pub async fn build_settlement_client(
    settings_provider: &impl SettingsProvider,
) -> Box<dyn SettlementClient + Send + Sync> {
    match get_env_var_or_panic("SETTLEMENT_LAYER").as_str() {
        "ethereum" => Box::new(EthereumSettlementClient::with_settings(settings_provider)),
        "starknet" => Box::new(StarknetSettlementClient::with_settings(settings_provider).await),
        _ => panic!("Unsupported Settlement layer"),
    }
}

pub async fn build_storage_client() -> Box<dyn DataStorage + Send + Sync> {
    match get_env_var_or_panic("DATA_STORAGE").as_str() {
        "s3" => Box::new(AWSS3::new(AWSS3ConfigType::WithoutEndpoint(AWSS3Config::new_from_env())).await),
        _ => panic!("Unsupported Storage Client"),
    }
}
