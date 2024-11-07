use alert::AlertParams;
use aws_config::AWSConfigParams;
use clap::{ArgGroup, Parser};
use da::DaParams;
use database::DatabaseParams;
use ethereum_da_client::config::EthereumDaParams;
use ethereum_settlement_client::config::EthereumSettlementParams;
use prover::ProverParams;
use queue::QueueParams;
use settlement::SettlementParams;
use sharp_service::config::SharpParams;
use snos::SNOSParams;
use starknet_settlement_client::config::StarknetSettlementParams;
use storage::StorageParams;
use url::Url;

use crate::alerts::aws_sns::AWSSNSParams;
use crate::config::ServiceParams;
use crate::data_storage::aws_s3::config::AWSS3Params;
use crate::database::mongodb::config::MongoDBParams;
use crate::queue::sqs::AWSSQSParams;
use crate::routes::ServerParams;
use crate::telemetry::InstrumentationParams;

pub mod alert;
pub mod aws_config;
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
)]
pub struct RunCmd {
    // AWS Config
    #[clap(flatten)]
    pub aws_config_args: AWSConfigParams,

    // Settlement Layer
    #[command(flatten)]
    ethereum_args: settlement::ethereum::EthereumSettlementCliArgs,

    #[command(flatten)]
    starknet_args: settlement::starknet::StarknetSettlementCliArgs,

    // Storage
    #[clap(flatten)]
    pub aws_s3_args: storage::aws_s3::AWSS3CliArgs,

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

    // SNOS
    #[clap(flatten)]
    pub snos_args: snos::SNOSCliArgs,

    #[arg(env = "MADARA_RPC_URL", long, required = true)]
    pub madara_rpc_url: Url,

    // Service
    #[clap(flatten)]
    pub service_args: service::ServiceCliArgs,
    #[clap(flatten)]
    pub instrumentation_args: instrumentation::InstrumentationCliArgs,
}

impl RunCmd {
    pub fn validate_settlement_params(&self) -> Result<settlement::SettlementParams, String> {
        match (self.ethereum_args.settle_on_ethereum, self.starknet_args.settle_on_starknet) {
            (true, false) => {
                // TODO: Ensure Starknet params are not provided

                // Get Ethereum params or error if none provided
                // Either all the values are provided or panic
                let ethereum_params = EthereumSettlementParams {
                    ethereum_rpc_url: self.ethereum_args.ethereum_rpc_url.clone().unwrap(),
                    ethereum_private_key: self.ethereum_args.ethereum_private_key.clone().unwrap(),
                    l1_core_contract_address: self.ethereum_args.l1_core_contract_address.clone().unwrap(),
                    starknet_operator_address: self.ethereum_args.starknet_operator_address.clone().unwrap(),
                };
                Ok(SettlementParams::Ethereum(ethereum_params))
            }
            (false, true) => {
                // TODO: Ensure Ethereum params are not provided

                // Get Starknet params or error if none provided
                // Either all the values are provided or panic
                let starknet_params = StarknetSettlementParams {
                    starknet_rpc_url: self.starknet_args.starknet_rpc_url.clone().unwrap(),
                    starknet_private_key: self.starknet_args.starknet_private_key.clone().unwrap(),
                    starknet_account_address: self.starknet_args.starknet_account_address.clone().unwrap(),
                    starknet_cairo_core_contract_address: self
                        .starknet_args
                        .starknet_cairo_core_contract_address
                        .clone()
                        .unwrap(),
                    starknet_finality_retry_wait_in_secs: self
                        .starknet_args
                        .starknet_finality_retry_wait_in_secs
                        .unwrap(),
                    madara_binary_path: self.starknet_args.madara_binary_path.clone().unwrap(),
                };
                Ok(SettlementParams::Starknet(starknet_params))
            }
            (true, true) | (false, false) => Err("Exactly one settlement layer must be selected".to_string()),
        }
    }

    pub fn validate_storage_params(&self) -> Result<StorageParams, String> {
        if self.aws_s3_args.aws_s3 {
            Ok(StorageParams::AWSS3(AWSS3Params { bucket_name: self.aws_s3_args.bucket_name.clone().unwrap() }))
        } else {
            Err("Only AWS S3 is supported as of now".to_string())
        }
    }

    pub fn validate_queue_params(&self) -> Result<QueueParams, String> {
        if self.aws_sqs_args.aws_sqs {
            Ok(QueueParams::AWSSQS(AWSSQSParams {
                queue_base_url: self.aws_sqs_args.queue_base_url.clone().unwrap(),
                sqs_prefix: self.aws_sqs_args.sqs_prefix.clone().unwrap(),
                sqs_suffix: self.aws_sqs_args.sqs_suffix.clone().unwrap(),
            }))
        } else {
            Err("Only AWS SQS is supported as of now".to_string())
        }
    }

    pub fn validate_alert_params(&self) -> Result<AlertParams, String> {
        if self.aws_sns_args.aws_sns {
            Ok(AlertParams::AWSSNS(AWSSNSParams { sns_arn: self.aws_sns_args.sns_arn.clone().unwrap() }))
        } else {
            Err("Only AWS SNS is supported as of now".to_string())
        }
    }

    pub fn validate_database_params(&self) -> Result<DatabaseParams, String> {
        if self.mongodb_args.mongodb {
            Ok(DatabaseParams::MongoDB(MongoDBParams {
                connection_url: self.mongodb_args.connection_url.clone().unwrap(),
                database_name: self.mongodb_args.database_name.clone().unwrap(),
            }))
        } else {
            Err("Only MongoDB is supported as of now".to_string())
        }
    }

    pub fn validate_da_params(&self) -> Result<DaParams, String> {
        if self.ethereum_da_args.da_on_ethereum {
            Ok(DaParams::Ethereum(EthereumDaParams { da_rpc_url: self.ethereum_da_args.da_rpc_url.clone().unwrap() }))
        } else {
            Err("Only Ethereum is supported as of now".to_string())
        }
    }

    pub fn validate_prover_params(&self) -> Result<ProverParams, String> {
        if self.sharp_args.sharp {
            Ok(ProverParams::Sharp(SharpParams {
                sharp_customer_id: self.sharp_args.sharp_customer_id.clone().unwrap(),
                sharp_url: self.sharp_args.sharp_url.clone().unwrap(),
                sharp_user_crt: self.sharp_args.sharp_user_crt.clone().unwrap(),
                sharp_user_key: self.sharp_args.sharp_user_key.clone().unwrap(),
                sharp_rpc_node_url: self.sharp_args.sharp_rpc_node_url.clone().unwrap(),
                sharp_proof_layout: self.sharp_args.sharp_proof_layout.clone().unwrap(),
                gps_verifier_contract_address: self.sharp_args.gps_verifier_contract_address.clone().unwrap(),
                sharp_server_crt: self.sharp_args.sharp_server_crt.clone().unwrap(),
            }))
        } else {
            Err("Only Sharp is supported as of now".to_string())
        }
    }

    pub fn validate_instrumentation_params(&self) -> Result<InstrumentationParams, String> {
        Ok(InstrumentationParams {
            otel_service_name: self.instrumentation_args.otel_service_name.clone().unwrap(),
            otel_collector_endpoint: self.instrumentation_args.otel_collector_endpoint.clone(),
            log_level: self.instrumentation_args.log_level,
        })
    }

    pub fn validate_server_params(&self) -> Result<ServerParams, String> {
        Ok(ServerParams { host: self.server_args.host.clone(), port: self.server_args.port })
    }

    pub fn validate_service_params(&self) -> Result<ServiceParams, String> {
        Ok(ServiceParams {
            max_block_to_process: self.service_args.max_block_to_process,
            min_block_to_process: self.service_args.min_block_to_process,
        })
    }

    pub fn validate_snos_params(&self) -> Result<SNOSParams, String> {
        Ok(SNOSParams { rpc_for_snos: self.snos_args.rpc_for_snos.clone() })
    }
}
