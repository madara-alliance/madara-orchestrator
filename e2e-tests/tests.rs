use e2e_tests::ethereum::EthereumClient;
use e2e_tests::localstack::LocalStack;
use e2e_tests::sharp::SharpClient;
use e2e_tests::starknet_client::StarknetClient;
use e2e_tests::{
    get_env_var_or_panic, mock_proving_job_endpoint_output, mock_starknet_get_state_update, put_job_data_in_db,
    MongoDbServer, Orchestrator,
};
use orchestrator::queue::job_queue::WorkerTriggerType;
use std::time::Duration;
use tokio::time::sleep;

extern crate e2e_tests;

#[tokio::test]
async fn test_orchestrator_workflow() {
    // Fetching the env vars from the test env file because setting up of the environment
    // requires all these variables.
    dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

    let (mongo_db_instance, mut starknet_client_mock, _ethereum_client_mock, mut sharp_client_mock, env_vec) =
        setup_for_test().await.unwrap();

    // Step 1 : SNOS job runs
    // TODO : Update the code with actual SNOS implementation
    // Updates the job in the db
    put_job_data_in_db(&mongo_db_instance).await;

    // Step 2: Proving Job
    // Mocking the endpoint
    mock_proving_job_endpoint_output(&mut sharp_client_mock).await;

    // Step 3: DA job
    // mocking get_block_call from starknet client
    mock_starknet_get_state_update(&mut starknet_client_mock).await;

    // Step 4: State Update job
    // For now use_kzg_da is 0
    // TODO : After getting latest PIE file update the code here to perform actual `update_state_with_blobs`

    println!("Orchestrator setup completed ✅ >>> ");

    // Run orchestrator
    let mut orchestrator = Orchestrator::run(env_vec);
    orchestrator.wait_till_started().await;

    sleep(Duration::from_secs(1200)).await;

    // TODO :
    // Adding a case here to check for required state of the orchestrator to end the test.
}

pub async fn setup_for_test(
) -> color_eyre::Result<(MongoDbServer, StarknetClient, EthereumClient, SharpClient, Vec<(String, String)>)> {
    let mongo_db_instance = MongoDbServer::run().await;
    let starknet_client = StarknetClient::new();
    let ethereum_client = EthereumClient::new();
    let sharp_client = SharpClient::new();

    // Setting up LocalStack
    let localstack_instance = LocalStack {};
    // TODO : uncomment
    localstack_instance.setup_s3().await.unwrap();
    localstack_instance.setup_sqs().await.unwrap();
    localstack_instance.delete_event_bridge_rule("worker_trigger_scheduled").await.unwrap();
    localstack_instance.setup_event_bridge(WorkerTriggerType::Proving).await.unwrap();
    localstack_instance.setup_event_bridge(WorkerTriggerType::DataSubmission).await.unwrap();
    localstack_instance.setup_event_bridge(WorkerTriggerType::UpdateState).await.unwrap();
    println!("Localstack instance setup completed ✅");

    let mut env_vec = get_env_vec();

    // Adding other values to the environment variables vector
    env_vec.push(("MONGODB_CONNECTION_STRING".to_string(), mongo_db_instance.endpoint().to_string()));
    env_vec.push(("MADARA_RPC_URL".to_string(), starknet_client.url()));
    env_vec.push(("ETHEREUM_RPC_URL".to_string(), ethereum_client.endpoint()));
    env_vec.push(("SHARP_URL".to_string(), sharp_client.url()));

    // Sharp envs
    env_vec.push(("SHARP_CUSTOMER_ID".to_string(), get_env_var_or_panic("SHARP_CUSTOMER_ID")));
    env_vec.push(("SHARP_USER_CRT".to_string(), get_env_var_or_panic("SHARP_USER_CRT")));
    env_vec.push(("SHARP_USER_KEY".to_string(), get_env_var_or_panic("SHARP_USER_KEY")));
    env_vec.push(("SHARP_SERVER_CRT".to_string(), get_env_var_or_panic("SHARP_SERVER_CRT")));

    Ok((mongo_db_instance, starknet_client, ethereum_client, sharp_client, env_vec))
}

/// To get env variables to be used in testing
fn get_env_vec() -> Vec<(String, String)> {
    let env_vec = vec![
        // AWS env vars
        ("AWS_ACCESS_KEY_ID", "AWS_ACCESS_KEY_ID"),
        ("AWS_SECRET_ACCESS_KEY", "AWS_SECRET_ACCESS_KEY"),
        ("AWS_S3_BUCKET_NAME", "madara-orchestrator-test-bucket"),
        ("AWS_S3_BUCKET_REGION", "us-east-1"),
        ("AWS_ENDPOINT_URL", "http://localhost.localstack.cloud:4566"),
        ("SQS_JOB_PROCESSING_QUEUE_URL", "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_job_processing_queue"),
        ("SQS_JOB_VERIFICATION_QUEUE_URL", "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_job_verification_queue"),
        ("SQS_JOB_HANDLE_FAILURE_QUEUE_URL", "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_job_handle_failure_queue"),
        ("SQS_WORKER_TRIGGER_QUEUE_URL", "http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/madara_orchestrator_worker_trigger_queue"),
        ("AWS_DEFAULT_REGION", "localhost"),
        // On chain config urls
        ("ETHEREUM_PRIVATE_KEY", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"), // Private key from anvil
        ("PRIVATE_KEY", "0xdead"), // placeholder key for starknet private key (will not be used as we would be using mocking for madara client for now)
        // Config URLs
        ("DA_LAYER", "ethereum"),
        ("PROVER_SERVICE", "sharp"),
        ("SETTLEMENT_CLIENT", "ethereum"),
        ("DATA_STORAGE", "s3_localstack"),
        ("ALERTS", "sns"),
        // Sharp configs
        ("SHARP_PROOF_LAYOUT", "small")
    ];

    env_vec.into_iter().map(|(first, second)| (first.to_string(), second.to_string())).collect()
}
