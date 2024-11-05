use clap::ArgGroup;
use url::Url;

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
        ArgGroup::new("settlement_layer")
            .args(&["ethereum", "starknet"])
            .required(true)
            .multiple(false)
    ),

    group(
        ArgGroup::new("da_layer")
            .args(&["ethereum"])
            .required(true)
            .multiple(false)
    ),
)]

pub struct RunCmd {
    #[clap(flatten)]
    pub server: server::ServerParams,

    #[clap(flatten)]
    pub aws_config: aws_config::AWSConfigParams,

    // part of storage
    #[clap(flatten)]
    pub aws_s3: storage::aws_s3::AWSS3Params,

    // part of queue
    #[clap(flatten)]  
    pub aws_sqs: queue::aws_sqs::AWSSQSParams,

    // part of alert
    #[clap(flatten)]
    pub aws_sns: alert::aws_sns::AWSSNSParams,


    // part of database
    #[clap(flatten)]
    pub mongodb: database::mongodb::MongoDBParams,

    // part of prover
    #[clap(flatten)]
    pub sharp: prover::sharp::SharpParams,

    // part of da_layer
    #[clap(flatten)]
    pub ethereum_da: da::ethereum::EthereumParams,


    #[clap(flatten)]
    pub starknet_settlement: settlement::starknet::StarknetSettlementParams,

    #[clap(flatten)]
    pub ethereum_settlement: settlement::ethereum::EthereumSettlementParams,

    #[clap(flatten)]
    pub snos: snos::SNOSParams,

    pub madara_rpc_url: Url,

    #[clap(flatten)]
    pub instrumentation: instrumentation::InstrumentationParams,
}
