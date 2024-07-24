pub mod constants;

use std::collections::HashMap;
use std::sync::Arc;

use ::uuid::Uuid;
use constants::*;
use da_client_interface::MockDaClient;
use dotenvy::dotenv;
use mongodb::Client;
use prover_client_interface::MockProverClient;
use rstest::*;
use settlement_client_interface::MockSettlementClient;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use url::Url;
use utils::env_utils::get_env_var_or_panic;
use utils::settings::default::DefaultSettingsProvider;

use crate::config::{config_force_init, Config};
use crate::data_storage::MockDataStorage;
use crate::database::mongodb::config::MongoDbConfig;
use crate::database::mongodb::MongoDb;
use crate::database::{DatabaseConfig, MockDatabase};
use crate::jobs::types::JobStatus::Created;
use crate::jobs::types::JobType::DataSubmission;
use crate::jobs::types::{ExternalId, JobItem};
use crate::queue::MockQueueProvider;

pub async fn init_config(
    rpc_url: Option<String>,
    database: Option<MockDatabase>,
    queue: Option<MockQueueProvider>,
    da_client: Option<MockDaClient>,
    prover_client: Option<MockProverClient>,
    settlement_client: Option<MockSettlementClient>,
    storage_client: Option<MockDataStorage>,
) -> Config {
    let _ = tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).with_target(false).try_init();

    let rpc_url = rpc_url.unwrap_or(MADARA_RPC_URL.to_string());
    let database = database.unwrap_or_default();
    let queue = queue.unwrap_or_default();
    let da_client = da_client.unwrap_or_default();
    let prover_client = prover_client.unwrap_or_default();
    let settlement_client = settlement_client.unwrap_or_default();
    let storage_client = storage_client.unwrap_or_default();

    // init starknet client
    let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url.as_str()).expect("Failed to parse URL")));

    Config::new(
        Arc::new(provider),
        Box::new(da_client),
        Box::new(prover_client),
        Box::new(settlement_client),
        Box::new(database),
        Box::new(queue),
        Box::new(storage_client),
    )
}

#[fixture]
pub fn default_job_item() -> JobItem {
    JobItem {
        id: Uuid::new_v4(),
        internal_id: String::from("0"),
        job_type: DataSubmission,
        status: Created,
        external_id: ExternalId::String("0".to_string().into_boxed_str()),
        metadata: HashMap::new(),
        version: 0,
    }
}

#[fixture]
pub fn custom_job_item(default_job_item: JobItem, #[default(String::from("0"))] internal_id: String) -> JobItem {
    let mut job_item = default_job_item;
    job_item.internal_id = internal_id;

    job_item
}

/// For implementation of integration tests
#[fixture]
pub async fn build_config() -> color_eyre::Result<()> {
    dotenv().ok();

    // init starknet client
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse(get_env_var_or_panic("MADARA_RPC_URL").as_str()).expect("Failed to parse URL"),
    ));

    // init database
    let database = Box::new(MongoDb::new(MongoDbConfig::new_from_env()).await);

    // init the queue
    let queue = Box::new(crate::queue::sqs::SqsQueue {});

    let da_client = crate::config::build_da_client().await;
    let settings_provider = DefaultSettingsProvider {};
    let settlement_client = crate::config::build_settlement_client(&settings_provider).await;
    let prover_client = crate::config::build_prover_service(&settings_provider);
    let storage_client = crate::config::build_storage_client().await;

    let config =
        Config::new(Arc::new(provider), da_client, prover_client, settlement_client, database, queue, storage_client);
    config_force_init(config).await;

    Ok(())
}

#[fixture]
pub async fn get_database_client() -> Client {
    MongoDb::new(MongoDbConfig::new_from_env()).await.client()
}

#[fixture]
pub async fn drop_database() -> color_eyre::Result<()> {
    let db_client: Client = get_database_client().await;
    // dropping `jobs` collection.
    db_client.database("orchestrator").collection::<JobItem>("jobs").drop(None).await?;
    Ok(())
}
