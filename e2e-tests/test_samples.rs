use e2e_tests::ethereum::EthereumClient;
use e2e_tests::sharp::SharpClient;
use e2e_tests::starknet_client::StarknetClient;
use e2e_tests::{MongoDbServer, Orchestrator};

extern crate e2e_tests;

#[ignore]
#[tokio::test]
async fn test_orchestrator_launches() {
    let mongodb = MongoDbServer::run().await;
    let mut orchestrator = Orchestrator::run(vec![
        // TODO: mock Madara RPC API
        ("MADARA_RPC_URL", "http://localhost"),
        ("MONGODB_CONNECTION_STRING", mongodb.endpoint().as_str()),
    ]);
    orchestrator.wait_till_started().await;
}

#[ignore]
#[tokio::test]
async fn test_orchestrator_workflow() {
    let mongo_db_instance = MongoDbServer::run().await;
    let starknet_client = StarknetClient::new();
    let ethereum_client = EthereumClient::new();
    let sharp_client = SharpClient::new();

    let mut env_vec = get_env_vec();

    let starknet_client_url = starknet_client.url();
    let sharp_client_url = sharp_client.url();
    let ethereum_client_endpoint = ethereum_client.endpoint();

    // Adding other values to the environment variables vector
    env_vec.push(("MONGODB_CONNECTION_STRING", mongo_db_instance.endpoint().as_str()));
    env_vec.push(("MADARA_RPC_URL", starknet_client_url.as_str()));
    env_vec.push(("ETHEREUM_RPC_URL", ethereum_client_endpoint.as_str()));
    env_vec.push(("SHARP_URL", sharp_client_url.as_str()));
}

/// To get env variables to be used in testing
fn get_env_vec() -> Vec<(&'static str, &'static str)> {
    vec![
        // AWS env vars
        ("AWS_ACCESS_KEY_ID", "AWS_ACCESS_KEY_ID"),
        ("AWS_SECRET_ACCESS_KEY", "AWS_SECRET_ACCESS_KEY"),
        ("AWS_S3_BUCKET_NAME", "madara-orchestrator-test-bucket"),
        ("AWS_S3_BUCKET_REGION", "us-east-1"),
        ("AWS_ENDPOINT_URL", "http://localhost.localstack.cloud:4566"),
        ("SQS_JOB_PROCESSING_QUEUE_URL", "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_job_processing_queue"),
        ("SQS_JOB_VERIFICATION_QUEUE_URL", "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_job_verification_queue"),
        ("AWS_DEFAULT_REGION", "localhost"),
        // On chain config urls
        ("ETHEREUM_PRIVATE_KEY", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"), // Private key from anvil
        ("PRIVATE_KEY", "0xdead"), // placeholder key for starknet private key (will not be used as we would be using mocking for madara client for now)
        // Config URLs
        ("DA_LAYER", "ethereum"),
        ("PROVER_SERVICE", "sharp"),
        ("SETTLEMENT_CLIENT", "ethereum"),
        ("DATA_STORAGE", "s3"),
        ("ALERTS", "sns")
    ]
}
