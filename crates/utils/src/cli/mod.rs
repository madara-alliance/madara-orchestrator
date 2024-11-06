use clap::ArgGroup;
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
     
    // group(
    //   ArgGroup::new("queue")
    //       .args(&["aws_sqs"])
    //       .required(true)
    //       .multiple(false)
    // ),

    // group(
    //   ArgGroup::new("alert")
    //       .args(&["aws_sns"])
    //       .required(true)
    //       .multiple(false)
    // ),

    // group(
    //     ArgGroup::new("prover")
    //         .args(&["sharp"])
    //         .required(true)
    //         .multiple(false)
    // ),

   

    // group(
    //     ArgGroup::new("da_layer")
    //         .args(&["ethereum"])
    //         .required(true)
    //         .multiple(false)
    // ),
)]

pub struct RunCmd {

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

    // #[clap(flatten)]
    // pub server: server::ServerParams,

    // #[clap(flatten)]
    // pub aws_config: aws_config::AWSConfigParams,

    // // part of storage
    

    // // part of queue
    // #[clap(flatten)]  
    // pub aws_sqs: queue::aws_sqs::AWSSQSParams,

    // // part of alert
    // #[clap(flatten)]
    // pub aws_sns: alert::aws_sns::AWSSNSParams,


    // // part of database
    // #[clap(flatten)]
    // pub mongodb: database::mongodb::MongoDBParams,

    // // part of prover
    // #[clap(flatten)]
    // pub sharp: prover::sharp::SharpParams,

    // // part of da_layer
    // #[clap(flatten)]
    // pub ethereum_da: da::ethereum::EthereumParams,


    // #[clap(flatten)]
    // pub starknet_settlement: settlement::starknet::StarknetSettlementParams,

    // #[clap(flatten)]
    // pub ethereum_settlement: settlement::ethereum::EthereumSettlementParams,

    // #[clap(flatten)]
    // pub snos: snos::SNOSParams,

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

}