use alert::AlertParams;
use aws_config::{AWSConfigCliArgs, AWSConfigParams};
use clap::{ArgGroup, Parser};
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
use crate::data_storage::aws_s3::AWSS3ValidatedArgs;
use crate::database::mongodb::MongoDBValidatedArgs;
use crate::queue::sqs::AWSSQSValidatedArgs;
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
