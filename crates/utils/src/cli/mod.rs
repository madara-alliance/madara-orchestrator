use alert::AlertParams;
use clap::ArgGroup;
use da::DaParams;
use database::DatabaseParams;
use prover::ProverParams;
use queue::QueueParams;
use settlement::SettlementParams;
use storage::StorageParams;

pub mod aws_config;
pub mod database;
pub mod instrumentation;
pub mod server;
pub mod storage;
pub mod queue;
pub mod alert;
pub mod prover;
pub mod da;
pub mod settlement;
pub mod snos;

#[derive(Clone, Debug, clap::Parser)]
#[clap(

    // Mutual Exclusion Solved 
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
    #[clap(long, group = "settlement_layer")]
    pub settle_on_ethereum: bool,

    #[clap(long, group = "settlement_layer")]
    pub settle_on_starknet: bool,

    #[clap(flatten)]
    ethereum_settlement_params: Option<settlement::ethereum::EthereumSettlementParams>,

    #[clap(flatten)]
    starknet_settlement_params: Option<settlement::starknet::StarknetSettlementParams>,

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

    // pub madara_rpc_url: Url,

    #[clap(flatten)]
    pub instrumentation: instrumentation::InstrumentationParams,
}

impl RunCmd {
    pub fn validate_settlement_params(self) -> Result<SettlementParams, String> {
        match (self.settle_on_ethereum, self.settle_on_starknet) {
            (true, false) => {
                // Ensure Starknet params are not provided
                if self.starknet_settlement_params.is_some() {
                    return Err("Starknet parameters cannot be specified when Ethereum settlement is selected".to_string());
                }
                
                // Get Ethereum params or error if none provided
                self.ethereum_settlement_params
                    .map(SettlementParams::Ethereum)
                    .ok_or_else(|| "Ethereum parameters must be provided when Ethereum settlement is selected".to_string())
            }
            (false, true) => {
                // Ensure Ethereum params are not provided
                if self.ethereum_settlement_params.is_some() {
                    return Err("Ethereum parameters cannot be specified when Starknet settlement is selected".to_string());
                }
                
                // Get Starknet params or error if none provided
                self.starknet_settlement_params
                    .map(SettlementParams::Starknet)
                    .ok_or_else(|| "Starknet parameters must be provided when Starknet settlement is selected".to_string())
            }
            (true, true) | (false, false) => {
                Err("Exactly one settlement layer must be selected".to_string())
            }
        }
    }

    pub fn validate_storage_params(self) -> Result<StorageParams, String> {
        if self.aws_s3 {
            Ok(StorageParams::AWSS3(self.aws_s3_params))
        } else {
            Err("Only AWS S3 is supported as of now".to_string())
        }
    }

    pub fn validate_queue_params(self) -> Result<QueueParams, String> {
        if self.aws_sqs {
            Ok(QueueParams::AWSSQS(self.aws_sqs_params))
        } else {
            Err("Only AWS SQS is supported as of now".to_string())
        }
    }

    pub fn validate_alert_params(self) -> Result<AlertParams, String> {
        if self.aws_sns {
            Ok(AlertParams::AWSSNS(self.aws_sns_params))
        } else {
            Err("Only AWS SNS is supported as of now".to_string())
        }
    }

    pub fn validate_database_params(self) -> Result<DatabaseParams, String> {
        if self.mongodb {
            Ok(DatabaseParams::MongoDB(self.mongodb_params))
        } else {
            Err("Only MongoDB is supported as of now".to_string())
        }
    }

    pub fn validate_da_params(self) -> Result<DaParams, String> {
        if self.da_on_ethereum {
            Ok(DaParams::Ethereum(self.ethereum_da_params))
        } else {
            Err("Only Ethereum is supported as of now".to_string())
        }
    }

    pub fn validate_prover_params(self) -> Result<ProverParams, String> {
        if self.sharp {
            Ok(ProverParams::Sharp(self.sharp_params))
        } else {
            Err("Only Sharp is supported as of now".to_string())
        }
    }

}