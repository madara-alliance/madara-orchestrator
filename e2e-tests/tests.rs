use e2e_tests::ethereum::EthereumClient;
use e2e_tests::localstack::LocalStack;
use e2e_tests::sharp::SharpClient;
use e2e_tests::starknet_client::StarknetClient;
use e2e_tests::{
    get_env_var_or_panic, get_mongo_db_client, mock_proving_job_endpoint_output, mock_starknet_get_nonce,
    mock_starknet_get_state_update, put_job_data_in_db_da, put_job_data_in_db_snos, put_job_data_in_db_update_state,
    MongoDbServer, Orchestrator,
};
use mongodb::bson::doc;
use orchestrator::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
use orchestrator::queue::job_queue::WorkerTriggerType;
use std::time::Duration;
use tokio::time::sleep;

extern crate e2e_tests;

#[tokio::test]
async fn test_orchestrator_workflow() {
    // Fetching the env vars from the test env file because setting up of the environment
    // requires all these variables.
    dotenvy::from_filename(".env.test").expect("Failed to load the .env file");

    let (
        mongo_db_instance,
        mut starknet_client_mock,
        _ethereum_client_mock,
        mut sharp_client_mock,
        env_vec,
        l2_block_number,
    ) = setup_for_test().await.unwrap();

    // Step 1 : SNOS job runs =========================================
    // TODO : Update the code with actual SNOS implementation
    // Updates the job in the db
    put_job_data_in_db_snos(&mongo_db_instance, l2_block_number.clone()).await;

    // Step 2: Proving Job ============================================
    // Mocking the endpoint
    mock_proving_job_endpoint_output(&mut sharp_client_mock).await;

    // Step 3: DA job =================================================
    // mocking get_block_call from starknet client

    // Adding a mock da job so that worker does not create 60k+ jobs
    put_job_data_in_db_da(&mongo_db_instance, l2_block_number.clone()).await;
    mock_starknet_get_state_update(&mut starknet_client_mock, l2_block_number.clone()).await;
    mock_starknet_get_nonce(&mut starknet_client_mock, l2_block_number.clone()).await;

    // Step 4: State Update job =======================================
    put_job_data_in_db_update_state(&mongo_db_instance, l2_block_number).await;

    println!("✅ Orchestrator setup completed.");

    // Run orchestrator
    let mut orchestrator = Orchestrator::run(env_vec);
    orchestrator.wait_till_started().await;

    // TODO : need to make this dynamic
    sleep(Duration::from_secs(900)).await;

    // Adding a case here to check for required state of the orchestrator to end the test.
    let l2_block_for_testing = get_env_var_or_panic("L2_BLOCK_NUMBER_FOR_TEST");
    let latest_job_in_db = get_database_state(&mongo_db_instance, l2_block_for_testing.clone()).await.unwrap();
    assert!(latest_job_in_db.is_some(), "Job doesn't exists in db");
    let job = latest_job_in_db.unwrap();

    // Asserts for the latest job for test to pass
    assert_eq!(job.internal_id, l2_block_for_testing);
    assert_eq!(job.external_id, ExternalId::String(Box::from(l2_block_for_testing)));
    assert_eq!(job.job_type, JobType::StateTransition);
    assert_eq!(job.status, JobStatus::Completed);
    assert_eq!(job.version, 3);
}

async fn get_database_state(
    mongo_db_server: &MongoDbServer,
    l2_block_for_testing: String,
) -> color_eyre::Result<Option<JobItem>> {
    let mongo_db_client = get_mongo_db_client(mongo_db_server).await;
    let collection = mongo_db_client.database("orchestrator").collection::<JobItem>("jobs");
    let filter = doc! { "internal_id": l2_block_for_testing, "job_type" : "StateTransition" };
    Ok(collection.find_one(filter, None).await.unwrap())
}

pub async fn setup_for_test(
) -> color_eyre::Result<(MongoDbServer, StarknetClient, EthereumClient, SharpClient, Vec<(String, String)>, String)> {
    let mongo_db_instance = MongoDbServer::run().await;
    println!("✅ Mongo DB setup completed");
    let starknet_client = StarknetClient::new();
    println!("✅ Starknet/Madara client setup completed");
    let ethereum_client = EthereumClient::new();
    ethereum_client.impersonate_account_as_starknet_operator().await;
    println!("✅ Ethereum client setup completed");
    let sharp_client = SharpClient::new();
    println!("✅ Sharp client setup completed");

    // Setting up LocalStack
    let localstack_instance = LocalStack::new();
    localstack_instance.setup_s3().await.unwrap();
    localstack_instance.setup_sqs().await.unwrap();
    localstack_instance.delete_event_bridge_rule("worker_trigger_scheduled").await.unwrap();
    localstack_instance.setup_event_bridge(WorkerTriggerType::Proving).await.unwrap();
    localstack_instance.setup_event_bridge(WorkerTriggerType::DataSubmission).await.unwrap();
    localstack_instance.setup_event_bridge(WorkerTriggerType::UpdateState).await.unwrap();
    println!("✅ Localstack instance setup completed");

    let mut env_vec = get_env_vec();

    // Adding other values to the environment variables vector
    env_vec.push(("MONGODB_CONNECTION_STRING".to_string(), mongo_db_instance.endpoint().to_string()));
    env_vec.push(("MADARA_RPC_URL".to_string(), starknet_client.url()));
    env_vec.push(("ETHEREUM_RPC_URL".to_string(), ethereum_client.endpoint()));
    env_vec.push(("SETTLEMENT_CLIENT_RPC".to_string(), ethereum_client.endpoint()));
    env_vec.push(("SHARP_URL".to_string(), sharp_client.url()));

    // Sharp envs
    env_vec.push(("SHARP_CUSTOMER_ID".to_string(), get_env_var_or_panic("SHARP_CUSTOMER_ID")));
    env_vec.push(("SHARP_USER_CRT".to_string(), get_env_var_or_panic("SHARP_USER_CRT")));
    env_vec.push(("SHARP_USER_KEY".to_string(), get_env_var_or_panic("SHARP_USER_KEY")));
    env_vec.push(("SHARP_SERVER_CRT".to_string(), get_env_var_or_panic("SHARP_SERVER_CRT")));

    Ok((
        mongo_db_instance,
        starknet_client,
        ethereum_client,
        sharp_client,
        env_vec,
        localstack_instance.l2_block_number(),
    ))
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
        ("DEFAULT_L1_CORE_CONTRACT_ADDRESS", "0xc662c410C0ECf747543f5bA90660f6ABeBD9C8c4"),
        ("STARKNET_OPERATOR_ADDRESS", "0x2C169DFe5fBbA12957Bdd0Ba47d9CEDbFE260CA7"),
        // Config URLs
        ("DA_LAYER", "ethereum"),
        ("PROVER_SERVICE", "sharp"),
        ("SETTLEMENT_LAYER", "ethereum_test"),
        ("DATA_STORAGE", "s3"),
        ("ALERTS", "sns"),
        // Sharp configs
        ("SHARP_PROOF_LAYOUT", "small"),
        ("MEMORY_PAGES_CONTRACT_ADDRESS", "0x47312450B3Ac8b5b8e247a6bB6d523e7605bDb60")
    ];

    env_vec.into_iter().map(|(first, second)| (first.to_string(), second.to_string())).collect()
}
