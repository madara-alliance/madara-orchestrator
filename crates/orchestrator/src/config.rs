use std::sync::Arc;

use aws_config::meta::region::RegionProviderChain;
use aws_config::{Region, SdkConfig};
use aws_credential_types::Credentials;
use color_eyre::eyre::eyre;
use da_client_interface::DaClient;
use dotenvy::dotenv;
use ethereum_da_client::EthereumDaClient;
use ethereum_settlement_client::EthereumSettlementClient;
use prover_client_interface::ProverClient;
use settlement_client_interface::SettlementClient;
use sharp_service::SharpProverService;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Url};
use starknet_settlement_client::StarknetSettlementClient;
use utils::cli::alert::AlertParams;
use utils::cli::aws_config::AWSConfigParams;
use utils::cli::da::DaParams;
use utils::cli::database::DatabaseParams;
use utils::cli::prover::ProverParams;
use utils::cli::queue::QueueParams;
use utils::cli::server::ServerParams;
use utils::cli::settlement::SettlementParams;
use utils::cli::storage::StorageParams;
use utils::cli::RunCmd;

use crate::alerts::aws_sns::AWSSNS;
use crate::alerts::Alerts;
use crate::data_storage::aws_s3::AWSS3;
use crate::data_storage::DataStorage;
use crate::database::mongodb::MongoDb;
use crate::database::Database;
use crate::queue::sqs::SqsQueue;
use crate::queue::QueueProvider;

/// The app config. It can be accessed from anywhere inside the service
/// by calling `config` function.
pub struct Config {
    /// The RPC url used by the [starknet_client]
    starknet_rpc_url: Url,

    server_config: ServerParams,
    /// The RPC url to be used when running SNOS
    /// When Madara supports getProof, we can re use
    /// starknet_rpc_url for SNOS as well
    pub snos_config: SnosConfig,
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
    /// Alerts client
    alerts: Box<dyn Alerts>,
}

#[derive(Debug, Clone)]
pub struct SnosConfig {
    pub rpc_url: Url,
    pub max_block_to_process: Option<u64>,
    pub min_block_to_process: Option<u64>,
}

/// `ProviderConfig` is an enum used to represent the global config built
/// using the settings provider. More providers can be added eg : GCP, AZURE etc.
///
/// We are using Arc<SdkConfig> because the config size is large and keeping it
/// a pointer is a better way to pass it through.
#[derive(Clone)]
pub enum ProviderConfig {
    AWS(Box<SdkConfig>),
}

impl ProviderConfig {
    pub fn get_aws_client_or_panic(&self) -> &SdkConfig {
        match self {
            ProviderConfig::AWS(config) => config.as_ref(),
        }
    }
}

/// To build a `SdkConfig` for AWS provider.
pub async fn get_aws_config(aws_config: &AWSConfigParams) -> SdkConfig {
    let region = aws_config.aws_region.clone();
    let region_provider = RegionProviderChain::first_try(Region::new(region)).or_default_provider();
    let credentials =
        Credentials::from_keys(aws_config.aws_access_key_id.clone(), aws_config.aws_secret_access_key.clone(), None);
    aws_config::from_env().credentials_provider(credentials).region(region_provider).load().await
}

/// Initializes the app config
pub async fn init_config(run_cmd: &RunCmd) -> color_eyre::Result<Arc<Config>> {
    dotenv().ok();

    let aws_config = &run_cmd.aws_config_args;
    let provider_config = Arc::new(ProviderConfig::AWS(Box::new(get_aws_config(aws_config).await)));

    // init starknet client
    let rpc_url = run_cmd.madara_rpc_url.clone();

    // init snos url
    let snos_config = SnosConfig {
        rpc_url: run_cmd.snos_args.rpc_for_snos.clone(),
        max_block_to_process: run_cmd.snos_args.max_block_to_process,
        min_block_to_process: run_cmd.snos_args.min_block_to_process,
    };

    let server_config = run_cmd.server.clone();
    let provider = JsonRpcClient::new(HttpTransport::new(rpc_url.clone()));

    // init database
    let database_params =
        run_cmd.validate_database_params().map_err(|e| eyre!("Failed to validate database params: {e}"))?;
    let database = build_database_client(&database_params).await;

    // init DA client
    let da_params = run_cmd.validate_da_params().map_err(|e| eyre!("Failed to validate DA params: {e}"))?;
    let da_client = build_da_client(&da_params).await;

    // init settlement
    let settlement_params =
        run_cmd.validate_settlement_params().map_err(|e| eyre!("Failed to validate settlement params: {e}"))?;
    let settlement_client = build_settlement_client(&settlement_params).await?;

    // init prover
    let prover_params = run_cmd.validate_prover_params().map_err(|e| eyre!("Failed to validate prover params: {e}"))?;
    let prover_client = build_prover_service(&prover_params);

    // init storage
    let data_storage_params =
        run_cmd.validate_storage_params().map_err(|e| eyre!("Failed to validate storage params: {e}"))?;
    let storage_client = build_storage_client(&data_storage_params, provider_config.clone()).await;

    // init alerts
    let alert_params = run_cmd.validate_alert_params().map_err(|e| eyre!("Failed to validate alert params: {e}"))?;
    let alerts_client = build_alert_client(&alert_params, provider_config.clone()).await;

    // init the queue
    // TODO: we use omniqueue for now which doesn't support loading AWS config
    // from `SdkConfig`. We can later move to using `aws_sdk_sqs`. This would require
    // us stop using the generic omniqueue abstractions for message ack/nack
    // init queue
    let queue_params = run_cmd.validate_queue_params().map_err(|e| eyre!("Failed to validate queue params: {e}"))?;
    let queue = build_queue_client(&queue_params);

    Ok(Arc::new(Config::new(
        rpc_url,
        server_config,
        snos_config,
        Arc::new(provider),
        da_client,
        prover_client,
        settlement_client,
        database,
        queue,
        storage_client,
        alerts_client,
    )))
}

impl Config {
    /// Create a new config
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        starknet_rpc_url: Url,
        server_config: ServerParams,
        snos_config: SnosConfig,
        starknet_client: Arc<JsonRpcClient<HttpTransport>>,
        da_client: Box<dyn DaClient>,
        prover_client: Box<dyn ProverClient>,
        settlement_client: Box<dyn SettlementClient>,
        database: Box<dyn Database>,
        queue: Box<dyn QueueProvider>,
        storage: Box<dyn DataStorage>,
        alerts: Box<dyn Alerts>,
    ) -> Self {
        Self {
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
        }
    }

    /// Returns the starknet rpc url
    pub fn starknet_rpc_url(&self) -> &Url {
        &self.starknet_rpc_url
    }

    /// Returns the server config
    pub fn server_config(&self) -> &ServerParams {
        &self.server_config
    }

    /// Returns the snos rpc url
    pub fn snos_config(&self) -> &SnosConfig {
        &self.snos_config
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

    /// Returns the alerts client
    pub fn alerts(&self) -> &dyn Alerts {
        self.alerts.as_ref()
    }
}

use std::str::FromStr;

use alloy::network::Ethereum;
use alloy::providers::ProviderBuilder;
use alloy::rpc::client::RpcClient;

/// Builds the DA client based on the environment variable DA_LAYER
pub async fn build_da_client(da_params: &DaParams) -> Box<dyn DaClient + Send + Sync> {
    match da_params {
        DaParams::Ethereum(ethereum_da_params) => {
            let client = RpcClient::new_http(
                Url::from_str(ethereum_da_params.da_rpc_url.as_str()).expect("Failed to parse DA_RPC_URL"),
            );
            let provider = ProviderBuilder::<_, Ethereum>::new().on_client(client);
            Box::new(EthereumDaClient { provider })
        }
    }
}

/// Builds the prover service based on the environment variable PROVER_SERVICE
pub fn build_prover_service(prover_params: &ProverParams) -> Box<dyn ProverClient> {
    match prover_params {
        ProverParams::Sharp(sharp_params) => Box::new(SharpProverService::new_with_settings(sharp_params)),
    }
}

/// Builds the settlement client depending on the env variable SETTLEMENT_LAYER
pub async fn build_settlement_client(
    settlement_params: &SettlementParams,
) -> color_eyre::Result<Box<dyn SettlementClient + Send + Sync>> {
    match settlement_params {
        SettlementParams::Ethereum(ethereum_settlement_params) => {
            #[cfg(not(feature = "testing"))]
            {
                Ok(Box::new(EthereumSettlementClient::new_with_settings(ethereum_settlement_params)))
            }
            #[cfg(feature = "testing")]
            {
                Ok(Box::new(EthereumSettlementClient::with_test_settings(ethereum_settlement_params)))
            }
        }
        SettlementParams::Starknet(starknet_settlement_params) => {
            Ok(Box::new(StarknetSettlementClient::new_with_settings(starknet_settlement_params).await))
        }
    }

    // match settlement_params {
    //     "ethereum" => {
    //         #[cfg(not(feature = "testing"))]
    //         {
    //             Ok(Box::new(EthereumSettlementClient::new_with_settings(settings_provider)))
    //         }
    //         #[cfg(feature = "testing")]
    //         {
    //             Ok(Box::new(EthereumSettlementClient::with_test_settings(
    //
    // RootProvider::new_http(get_env_var_or_panic("SETTLEMENT_RPC_URL").as_str().parse()?),
    //                 Address::from_str(&get_env_var_or_panic("L1_CORE_CONTRACT_ADDRESS"))?,
    //                 Url::from_str(get_env_var_or_panic("SETTLEMENT_RPC_URL").as_str())?,
    //
    // Some(Address::from_str(get_env_var_or_panic("STARKNET_OPERATOR_ADDRESS").as_str())?),
    //             )))
    //         }
    //     }
    //     "starknet" =>
    // Ok(Box::new(StarknetSettlementClient::new_with_settings(settings_provider).await)),     _
    // => panic!("Unsupported Settlement layer"), }
}

pub async fn build_storage_client(
    data_storage_params: &StorageParams,
    provider_config: Arc<ProviderConfig>,
) -> Box<dyn DataStorage + Send + Sync> {
    match data_storage_params {
        StorageParams::AWSS3(aws_s3_params) => Box::new(AWSS3::new_with_settings(aws_s3_params, provider_config).await),
    }
}

pub async fn build_alert_client(
    alert_params: &AlertParams,
    provider_config: Arc<ProviderConfig>,
) -> Box<dyn Alerts + Send + Sync> {
    match alert_params {
        AlertParams::AWSSNS(aws_sns_params) => {
            Box::new(AWSSNS::new_with_settings(aws_sns_params, provider_config).await)
        }
    }
}

pub fn build_queue_client(queue_params: &QueueParams) -> Box<dyn QueueProvider + Send + Sync> {
    match queue_params {
        QueueParams::AWSSQS(aws_sqs_params) => Box::new(SqsQueue { params: aws_sqs_params.clone() }),
    }
}

pub async fn build_database_client(database_params: &DatabaseParams) -> Box<dyn Database + Send + Sync> {
    match database_params {
        DatabaseParams::MongoDB(mongodb_params) => Box::new(MongoDb::new_with_settings(mongodb_params).await),
    }
}
