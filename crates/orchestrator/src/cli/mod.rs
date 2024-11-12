use std::time::Duration;

use alert::AlertParams;
use aws_config::{AWSConfigCliArgs, AWSConfigParams};
use clap::{ArgGroup, Parser};
use cron::event_bridge::AWSEventBridgeCliArgs;
use da::DaParams;
use database::DatabaseParams;
use ethereum_da_client::EthereumDaValidatedArgs;
use ethereum_settlement_client::EthereumSettlementValidatedArgs;
use prover::ProverParams;
use queue::QueueParams;
use settlement::SettlementParams;
use sharp_service::SharpValidatedArgs;
use snos::SNOSParams;
use starknet_settlement_client::StarknetSettlementValidatedArgs;
use storage::StorageParams;
use url::Url;

use crate::alerts::aws_sns::AWSSNSValidatedArgs;
use crate::config::ServiceParams;
use crate::cron::event_bridge::AWSEventBridgeValidatedArgs;
use crate::cron::CronParams;
use crate::data_storage::aws_s3::AWSS3ValidatedArgs;
use crate::database::mongodb::MongoDBValidatedArgs;
use crate::queue::sqs::AWSSQSValidatedArgs;
use crate::routes::ServerParams;
use crate::telemetry::InstrumentationParams;

pub mod alert;
pub mod aws_config;
pub mod cron;
pub mod da;
pub mod database;
pub mod instrumentation;
pub mod prover;
pub mod queue;
pub mod server;
pub mod service;
pub mod settlement;
pub mod snos;
pub mod storage;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[clap(
    group(
        ArgGroup::new("settlement_layer")
            .args(&["settle_on_ethereum", "settle_on_starknet"])
            .required(true)
            .multiple(false)
    ),
    group(
        ArgGroup::new("storage")
            .args(&["aws_s3"])
            .required(true)
            .multiple(false)
    ),
    group(
      ArgGroup::new("queue")
          .args(&["aws_sqs"])
          .required(true)
          .multiple(false)
    ),
    group(
      ArgGroup::new("alert")
          .args(&["aws_sns"])
          .required(true)
          .multiple(false)
    ),
    group(
        ArgGroup::new("prover")
            .args(&["sharp"])
            .required(true)
            .multiple(false)
    ),
    group(
        ArgGroup::new("da_layer")
            .args(&["da_on_ethereum"])
            .required(true)
            .multiple(false)
    ),
    group(
        ArgGroup::new("cron")
            .args(&["aws_event_bridge"])
            .required(true)
            .multiple(false)
    ),
)]
pub struct RunCmd {
    // AWS Config
    #[clap(flatten)]
    pub aws_config_args: AWSConfigCliArgs,

    // Settlement Layer
    #[command(flatten)]
    ethereum_args: settlement::ethereum::EthereumSettlementCliArgs,

    #[command(flatten)]
    starknet_args: settlement::starknet::StarknetSettlementCliArgs,

    // Storage
    #[clap(flatten)]
    pub aws_s3_args: storage::aws_s3::AWSS3CliArgs,

    // Queue
    #[clap(flatten)]
    pub aws_sqs_args: queue::aws_sqs::AWSSQSCliArgs,

    // Server
    #[clap(flatten)]
    pub server_args: server::ServerCliArgs,

    // Alert
    #[clap(flatten)]
    pub aws_sns_args: alert::aws_sns::AWSSNSCliArgs,

    // Database
    #[clap(flatten)]
    pub mongodb_args: database::mongodb::MongoDBCliArgs,

    // Data Availability Layer
    #[clap(flatten)]
    pub ethereum_da_args: da::ethereum::EthereumDaCliArgs,

    // Prover
    #[clap(flatten)]
    pub sharp_args: prover::sharp::SharpCliArgs,

    // Cron
    #[clap(flatten)]
    pub aws_event_bridge_args: AWSEventBridgeCliArgs,

    // SNOS
    #[clap(flatten)]
    pub snos_args: snos::SNOSCliArgs,

    #[arg(env = "MADARA_ORCHESTRATOR_MADARA_RPC_URL", long, required = true)]
    pub madara_rpc_url: Url,

    // Service
    #[clap(flatten)]
    pub service_args: service::ServiceCliArgs,
    #[clap(flatten)]
    pub instrumentation_args: instrumentation::InstrumentationCliArgs,
}

impl RunCmd {
    pub fn validate_aws_config_params(&self) -> Result<AWSConfigParams, String> {
        let aws_endpoint_url =
            Url::parse("http://localhost.localstack.cloud:4566").expect("Failed to parse AWS endpoint URL");
        let aws_default_region = "localhost".to_string();

        tracing::warn!("Setting AWS_ENDPOINT_URL to {} for AWS SDK to use", aws_endpoint_url);
        tracing::warn!("Setting AWS_DEFAULT_REGION to {} for Omniqueue to use", aws_default_region);

        Ok(AWSConfigParams {
            aws_access_key_id: self.aws_config_args.aws_access_key_id.clone(),
            aws_secret_access_key: self.aws_config_args.aws_secret_access_key.clone(),
            aws_region: self.aws_config_args.aws_region.clone(),
            aws_endpoint_url,
            aws_default_region,
        })
    }

    pub fn validate_alert_params(&self) -> Result<AlertParams, String> {
        if self.aws_sns_args.aws_sns {
            Ok(AlertParams::AWSSNS(AWSSNSValidatedArgs {
                sns_arn: self.aws_sns_args.sns_arn.clone().expect("SNS ARN is required"),
            }))
        } else {
            Err("Only AWS SNS is supported as of now".to_string())
        }
    }

    pub fn validate_queue_params(&self) -> Result<QueueParams, String> {
        if self.aws_sqs_args.aws_sqs {
            Ok(QueueParams::AWSSQS(AWSSQSValidatedArgs {
                queue_base_url: self.aws_sqs_args.queue_base_url.clone().expect("Queue base URL is required"),
                sqs_prefix: self.aws_sqs_args.sqs_prefix.clone().expect("SQS prefix is required"),
                sqs_suffix: self.aws_sqs_args.sqs_suffix.clone().expect("SQS suffix is required"),
            }))
        } else {
            Err("Only AWS SQS is supported as of now".to_string())
        }
    }

    pub fn validate_storage_params(&self) -> Result<StorageParams, String> {
        if self.aws_s3_args.aws_s3 {
            Ok(StorageParams::AWSS3(AWSS3ValidatedArgs {
                bucket_name: self.aws_s3_args.bucket_name.clone().expect("Bucket name is required"),
            }))
        } else {
            Err("Only AWS S3 is supported as of now".to_string())
        }
    }

    pub fn validate_database_params(&self) -> Result<DatabaseParams, String> {
        if self.mongodb_args.mongodb {
            Ok(DatabaseParams::MongoDB(MongoDBValidatedArgs {
                connection_url: self
                    .mongodb_args
                    .mongodb_connection_url
                    .clone()
                    .expect("MongoDB connection URL is required"),
                database_name: self
                    .mongodb_args
                    .mongodb_database_name
                    .clone()
                    .expect("MongoDB database name is required"),
            }))
        } else {
            Err("Only MongoDB is supported as of now".to_string())
        }
    }

    pub fn validate_da_params(&self) -> Result<DaParams, String> {
        if self.ethereum_da_args.da_on_ethereum {
            Ok(DaParams::Ethereum(EthereumDaValidatedArgs {
                ethereum_da_rpc_url: self
                    .ethereum_da_args
                    .ethereum_da_rpc_url
                    .clone()
                    .expect("Ethereum DA RPC URL is required"),
            }))
        } else {
            Err("Only Ethereum is supported as of now".to_string())
        }
    }

    pub fn validate_settlement_params(&self) -> Result<settlement::SettlementParams, String> {
        match (self.ethereum_args.settle_on_ethereum, self.starknet_args.settle_on_starknet) {
            (true, false) => {
                let ethereum_params = EthereumSettlementValidatedArgs {
                    ethereum_rpc_url: self
                        .ethereum_args
                        .ethereum_rpc_url
                        .clone()
                        .expect("Ethereum RPC URL is required"),
                    ethereum_private_key: self
                        .ethereum_args
                        .ethereum_private_key
                        .clone()
                        .expect("Ethereum private key is required"),
                    l1_core_contract_address: self
                        .ethereum_args
                        .l1_core_contract_address
                        .clone()
                        .expect("L1 core contract address is required"),
                    starknet_operator_address: self
                        .ethereum_args
                        .starknet_operator_address
                        .clone()
                        .expect("Starknet operator address is required"),
                };
                Ok(SettlementParams::Ethereum(ethereum_params))
            }
            (false, true) => {
                let starknet_params = StarknetSettlementValidatedArgs {
                    starknet_rpc_url: self
                        .starknet_args
                        .starknet_rpc_url
                        .clone()
                        .expect("Starknet RPC URL is required"),
                    starknet_private_key: self
                        .starknet_args
                        .starknet_private_key
                        .clone()
                        .expect("Starknet private key is required"),
                    starknet_account_address: self
                        .starknet_args
                        .starknet_account_address
                        .clone()
                        .expect("Starknet account address is required"),
                    starknet_cairo_core_contract_address: self
                        .starknet_args
                        .starknet_cairo_core_contract_address
                        .clone()
                        .expect("Starknet Cairo core contract address is required"),
                    starknet_finality_retry_wait_in_secs: self
                        .starknet_args
                        .starknet_finality_retry_wait_in_secs
                        .expect("Starknet finality retry wait in seconds is required"),
                    madara_binary_path: self
                        .starknet_args
                        .starknet_madara_binary_path
                        .clone()
                        .expect("Starknet Madara binary path is required"),
                };
                Ok(SettlementParams::Starknet(starknet_params))
            }
            (true, true) | (false, false) => Err("Exactly one settlement layer must be selected".to_string()),
        }
    }

    pub fn validate_prover_params(&self) -> Result<ProverParams, String> {
        if self.sharp_args.sharp {
            Ok(ProverParams::Sharp(SharpValidatedArgs {
                sharp_customer_id: self.sharp_args.sharp_customer_id.clone().expect("Sharp customer ID is required"),
                sharp_url: self.sharp_args.sharp_url.clone().expect("Sharp URL is required"),
                sharp_user_crt: self.sharp_args.sharp_user_crt.clone().expect("Sharp user certificate is required"),
                sharp_user_key: self.sharp_args.sharp_user_key.clone().expect("Sharp user key is required"),
                sharp_rpc_node_url: self.sharp_args.sharp_rpc_node_url.clone().expect("Sharp RPC node URL is required"),
                sharp_proof_layout: self.sharp_args.sharp_proof_layout.clone().expect("Sharp proof layout is required"),
                gps_verifier_contract_address: self
                    .sharp_args
                    .gps_verifier_contract_address
                    .clone()
                    .expect("GPS verifier contract address is required"),
                sharp_server_crt: self
                    .sharp_args
                    .sharp_server_crt
                    .clone()
                    .expect("Sharp server certificate is required"),
            }))
        } else {
            Err("Only Sharp is supported as of now".to_string())
        }
    }

    pub fn validate_cron_params(&self) -> Result<CronParams, String> {
        if self.aws_event_bridge_args.aws_event_bridge {
            Ok(CronParams::EventBridge(AWSEventBridgeValidatedArgs {
                target_queue_name: self
                    .aws_event_bridge_args
                    .target_queue_name
                    .clone()
                    .expect("Target queue name is required"),
                cron_time: Duration::from_secs(
                    self.aws_event_bridge_args
                        .cron_time
                        .clone()
                        .expect("Cron time is required")
                        .parse::<u64>()
                        .expect("Failed to parse cron time"),
                ),
                trigger_rule_name: self
                    .aws_event_bridge_args
                    .trigger_rule_name
                    .clone()
                    .expect("Trigger rule name is required"),
            }))
        } else {
            Err("Only AWS Event Bridge is supported as of now".to_string())
        }
    }

    pub fn validate_instrumentation_params(&self) -> Result<InstrumentationParams, String> {
        Ok(InstrumentationParams {
            otel_service_name: self
                .instrumentation_args
                .otel_service_name
                .clone()
                .expect("OTel service name is required"),
            otel_collector_endpoint: self.instrumentation_args.otel_collector_endpoint.clone(),
            log_level: self.instrumentation_args.log_level,
        })
    }

    pub fn validate_server_params(&self) -> Result<ServerParams, String> {
        Ok(ServerParams { host: self.server_args.host.clone(), port: self.server_args.port })
    }

    pub fn validate_service_params(&self) -> Result<ServiceParams, String> {
        Ok(ServiceParams {
            // return None if the value is empty string
            max_block_to_process: self.service_args.max_block_to_process.clone().and_then(|s| {
                if s.is_empty() { None } else { Some(s.parse::<u64>().expect("Failed to parse max block to process")) }
            }),
            min_block_to_process: self.service_args.min_block_to_process.clone().and_then(|s| {
                if s.is_empty() { None } else { Some(s.parse::<u64>().expect("Failed to parse min block to process")) }
            }),
        })
    }

    pub fn validate_snos_params(&self) -> Result<SNOSParams, String> {
        Ok(SNOSParams { rpc_for_snos: self.snos_args.rpc_for_snos.clone() })
    }
}

#[cfg(test)]
pub mod test {

    use rstest::{fixture, rstest};
    use tracing::Level;
    use url::Url;

    use super::alert::aws_sns::AWSSNSCliArgs;
    use super::aws_config::AWSConfigCliArgs;
    use super::cron::event_bridge::AWSEventBridgeCliArgs;
    use super::da::ethereum::EthereumDaCliArgs;
    use super::database::mongodb::MongoDBCliArgs;
    use super::instrumentation::InstrumentationCliArgs;
    use super::prover::sharp::SharpCliArgs;
    use super::queue::aws_sqs::AWSSQSCliArgs;
    use super::server::ServerCliArgs;
    use super::service::ServiceCliArgs;
    use super::settlement::ethereum::EthereumSettlementCliArgs;
    use super::settlement::starknet::StarknetSettlementCliArgs;
    use super::snos::SNOSCliArgs;
    use super::storage::aws_s3::AWSS3CliArgs;
    use crate::cli::RunCmd;

    // create a fixture for the CLI
    #[fixture]
    pub fn setup_cmd() -> RunCmd {
        RunCmd {
            aws_config_args: AWSConfigCliArgs {
                aws_access_key_id: "".to_string(),
                aws_secret_access_key: "".to_string(),
                aws_region: "".to_string(),
            },
            aws_event_bridge_args: AWSEventBridgeCliArgs {
                aws_event_bridge: true,
                target_queue_name: Some("".to_string()),
                cron_time: Some("".to_string()),
                trigger_rule_name: Some("".to_string()),
            },
            aws_s3_args: AWSS3CliArgs { aws_s3: true, bucket_name: Some("".to_string()) },
            aws_sqs_args: AWSSQSCliArgs {
                aws_sqs: true,
                queue_base_url: Some("".to_string()),
                sqs_prefix: Some("".to_string()),
                sqs_suffix: Some("".to_string()),
            },
            server_args: ServerCliArgs { host: "".to_string(), port: 0 },
            aws_sns_args: AWSSNSCliArgs { aws_sns: true, sns_arn: Some("".to_string()) },

            instrumentation_args: InstrumentationCliArgs {
                otel_service_name: Some("".to_string()),
                otel_collector_endpoint: None,
                log_level: Level::INFO,
            },

            mongodb_args: MongoDBCliArgs {
                mongodb: true,
                mongodb_connection_url: Some("".to_string()),
                mongodb_database_name: Some("".to_string()),
            },

            madara_rpc_url: Url::parse("http://localhost:8545").unwrap(),

            sharp_args: SharpCliArgs {
                sharp: true,
                sharp_customer_id: Some("".to_string()),
                sharp_url: Some(Url::parse("http://localhost:8545").unwrap()),
                sharp_user_crt: Some("".to_string()),
                sharp_user_key: Some("".to_string()),
                sharp_rpc_node_url: Some(Url::parse("http://localhost:8545").unwrap()),
                sharp_proof_layout: Some("".to_string()),
                gps_verifier_contract_address: Some("".to_string()),
                sharp_server_crt: Some("".to_string()),
            },

            starknet_args: StarknetSettlementCliArgs {
                starknet_rpc_url: Some(Url::parse("http://localhost:8545").unwrap()),
                starknet_private_key: Some("".to_string()),
                starknet_account_address: Some("".to_string()),
                starknet_cairo_core_contract_address: Some("".to_string()),
                starknet_finality_retry_wait_in_secs: Some(0),
                starknet_madara_binary_path: Some("".to_string()),
                settle_on_starknet: false,
            },

            ethereum_args: EthereumSettlementCliArgs {
                ethereum_rpc_url: Some(Url::parse("http://localhost:8545").unwrap()),
                ethereum_private_key: Some("".to_string()),
                l1_core_contract_address: Some("".to_string()),
                starknet_operator_address: Some("".to_string()),
                settle_on_ethereum: true,
            },

            ethereum_da_args: EthereumDaCliArgs {
                da_on_ethereum: true,
                ethereum_da_rpc_url: Some(Url::parse("http://localhost:8545").unwrap()),
            },

            snos_args: SNOSCliArgs { rpc_for_snos: Url::parse("http://localhost:8545").unwrap() },

            service_args: ServiceCliArgs {
                max_block_to_process: Some("".to_string()),
                min_block_to_process: Some("".to_string()),
            },
        }
    }

    // Let's create a test for the CLI each validator

    #[rstest]
    fn test_validate_aws_config_params(setup_cmd: RunCmd) {
        let aws_config_params = setup_cmd.validate_aws_config_params();
        assert!(aws_config_params.is_ok());
    }

    #[rstest]
    fn test_validate_alert_params(setup_cmd: RunCmd) {
        let alert_params = setup_cmd.validate_alert_params();
        assert!(alert_params.is_ok());
    }

    #[rstest]
    fn test_validate_queue_params(setup_cmd: RunCmd) {
        let queue_params = setup_cmd.validate_queue_params();
        assert!(queue_params.is_ok());
    }

    #[rstest]
    fn test_validate_storage_params(setup_cmd: RunCmd) {
        let storage_params = setup_cmd.validate_storage_params();
        assert!(storage_params.is_ok());
    }

    #[rstest]
    fn test_validate_database_params(setup_cmd: RunCmd) {
        let database_params = setup_cmd.validate_database_params();
        assert!(database_params.is_ok());
    }

    #[rstest]
    fn test_validate_da_params(setup_cmd: RunCmd) {
        let da_params = setup_cmd.validate_da_params();
        assert!(da_params.is_ok());
    }

    #[rstest]
    fn test_validate_settlement_params(setup_cmd: RunCmd) {
        let settlement_params = setup_cmd.validate_settlement_params();
        assert!(settlement_params.is_ok());
    }

    #[rstest]
    fn test_validate_prover_params(setup_cmd: RunCmd) {
        let prover_params = setup_cmd.validate_prover_params();
        assert!(prover_params.is_ok());
    }

    #[rstest]
    fn test_validate_cron_params(setup_cmd: RunCmd) {
        let cron_params = setup_cmd.validate_cron_params();
        assert!(cron_params.is_ok());
    }

    #[rstest]
    fn test_validate_instrumentation_params(setup_cmd: RunCmd) {
        let instrumentation_params = setup_cmd.validate_instrumentation_params();
        assert!(instrumentation_params.is_ok());
    }

    #[rstest]
    fn test_validate_server_params(setup_cmd: RunCmd) {
        let server_params = setup_cmd.validate_server_params();
        assert!(server_params.is_ok());
    }

    #[rstest]
    fn test_validate_snos_params(setup_cmd: RunCmd) {
        let snos_params = setup_cmd.validate_snos_params();
        assert!(snos_params.is_ok());
    }

    #[rstest]
    fn test_validate_service_params(setup_cmd: RunCmd) {
        let service_params = setup_cmd.validate_service_params();
        assert!(service_params.is_ok());
    }
}
