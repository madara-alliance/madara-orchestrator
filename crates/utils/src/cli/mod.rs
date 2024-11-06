use alert::AlertParams;
use clap::{command, ArgGroup, Parser};
use da::DaParams;
use database::DatabaseParams;
use prover::ProverParams;
use queue::QueueParams;
use settlement::{ethereum::EthereumSettlementParams, starknet::StarknetSettlementParams, SettlementParams};
use storage::StorageParams;
use url::Url;

pub mod alert;
pub mod aws_config;
pub mod da;
pub mod database;
pub mod instrumentation;
pub mod prover;
pub mod queue;
pub mod server;
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
    pub aws_config: aws_config::AWSConfigParams,

    // Settlement Layer
    #[command(flatten)]
    ethereum_args: settlement::ethereum::EthereumSettlementArgs,

    #[command(flatten)]
    starknet_args: settlement::starknet::StarknetSettlementArgs,

    // Storage
    #[clap(long, group = "storage")]
    pub aws_s3: bool,

    #[clap(flatten)]
    pub aws_s3_params: storage::aws_s3::AWSS3Params,

    // Queue
    #[clap(long, group = "queue")]
    pub aws_sqs: bool,

    #[clap(flatten)]
    pub aws_sqs_params: queue::aws_sqs::AWSSQSParams,

    // Server
    #[clap(flatten)]
    pub server: server::ServerParams,

    // Alert
    #[clap(long, group = "alert")]
    pub aws_sns: bool,

    #[clap(flatten)]
    pub aws_sns_params: alert::aws_sns::AWSSNSParams,

    // Database
    #[clap(long, group = "database")]
    pub mongodb: bool,

    #[clap(flatten)]
    pub mongodb_params: database::mongodb::MongoDBParams,

    // Data Availability Layer
    #[clap(long, group = "da_layer")]
    pub da_on_ethereum: bool,

    #[clap(flatten)]
    pub ethereum_da_params: da::ethereum::EthereumParams,

    // Prover
    #[clap(long, group = "prover")]
    pub sharp: bool,

    #[clap(flatten)]
    pub sharp_params: prover::sharp::SharpParams,

    #[clap(flatten)]
    pub snos: snos::SNOSParams,

    #[arg(env = "MADARA_RPC_URL", long, required = true)]
    pub madara_rpc_url: Url,

    #[clap(flatten)]
    pub instrumentation: instrumentation::InstrumentationParams,
}

impl RunCmd {
    pub fn validate_settlement_params(&self) -> Result<SettlementParams, String> {
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
                    starknet_cairo_core_contract_address: self.starknet_args.starknet_cairo_core_contract_address.clone().unwrap(),
                    starknet_finality_retry_wait_in_secs: self.starknet_args.starknet_finality_retry_wait_in_secs.clone().unwrap(),
                    madara_binary_path: self.starknet_args.madara_binary_path.clone().unwrap(),
                };
                Ok(SettlementParams::Starknet(starknet_params))
               
            }
            (true, true) | (false, false) => Err("Exactly one settlement layer must be selected".to_string()),
        }
    }

    pub fn validate_storage_params(&self) -> Result<StorageParams, String> {
        if self.aws_s3 {
            Ok(StorageParams::AWSS3(self.aws_s3_params.clone()))
        } else {
            Err("Only AWS S3 is supported as of now".to_string())
        }
    }

    pub fn validate_queue_params(&self) -> Result<QueueParams, String> {
        if self.aws_sqs {
            Ok(QueueParams::AWSSQS(self.aws_sqs_params.clone()))
        } else {
            Err("Only AWS SQS is supported as of now".to_string())
        }
    }

    pub fn validate_alert_params(&self) -> Result<AlertParams, String> {
        if self.aws_sns {
            Ok(AlertParams::AWSSNS(self.aws_sns_params.clone()))
        } else {
            Err("Only AWS SNS is supported as of now".to_string())
        }
    }

    pub fn validate_database_params(&self) -> Result<DatabaseParams, String> {
        if self.mongodb {
            Ok(DatabaseParams::MongoDB(self.mongodb_params.clone()))
        } else {
            Err("Only MongoDB is supported as of now".to_string())
        }
    }

    pub fn validate_da_params(&self) -> Result<DaParams, String> {
        if self.da_on_ethereum {
            Ok(DaParams::Ethereum(self.ethereum_da_params.clone()))
        } else {
            Err("Only Ethereum is supported as of now".to_string())
        }
    }

    pub fn validate_prover_params(&self) -> Result<ProverParams, String> {
        if self.sharp {
            Ok(ProverParams::Sharp(self.sharp_params.clone()))
        } else {
            Err("Only Sharp is supported as of now".to_string())
        }
    }
}
