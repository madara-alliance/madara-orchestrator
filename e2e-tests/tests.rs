use alloy::primitives::Address;
use e2e_tests::ethereum::EthereumClient;
use e2e_tests::localstack::LocalStack;
use e2e_tests::sharp::SharpClient;
use e2e_tests::starknet_client::StarknetClient;
use e2e_tests::utils::{get_mongo_db_client, read_state_update_from_file, vec_u8_to_hex_string};
use e2e_tests::{MongoDbServer, Orchestrator};
use mongodb::bson::doc;
use orchestrator::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
use orchestrator::queue::job_queue::WorkerTriggerType;
use serde::{Deserialize, Serialize};
use serde_json::json;
use starknet::core::types::{FieldElement, MaybePendingStateUpdate};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use utils::env_utils::get_env_var_or_panic;
use uuid::Uuid;

extern crate e2e_tests;

struct Setup {
    mongo_db_instance: MongoDbServer,
    starknet_client: StarknetClient,
    ethereum_client: EthereumClient,
    sharp_client: SharpClient,
    env_vector: Vec<(String, String)>,
    l2_block_number: String,
}

impl Setup {
    pub async fn new() -> Self {
        let mongo_db_instance = MongoDbServer::run().await;
        println!("✅ Mongo DB setup completed");
        let starknet_client = StarknetClient::new();
        println!("✅ Starknet/Madara client setup completed");
        let ethereum_client = EthereumClient::new();
        println!("✅ Ethereum client setup completed");
        let sharp_client = SharpClient::new();
        println!("✅ Sharp client setup completed");

        // Setting up LocalStack
        let localstack_instance = LocalStack::new().await;
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

        Self {
            mongo_db_instance,
            starknet_client,
            ethereum_client,
            sharp_client,
            env_vector: env_vec,
            l2_block_number: localstack_instance.l2_block_number(),
        }
    }

    pub fn mongo_db_instance(&self) -> &MongoDbServer {
        &self.mongo_db_instance
    }

    pub fn ethereum_client(&mut self) -> &mut EthereumClient {
        &mut self.ethereum_client
    }

    pub fn starknet_client(&mut self) -> &mut StarknetClient {
        &mut self.starknet_client
    }

    pub fn sharp_client(&mut self) -> &mut SharpClient {
        &mut self.sharp_client
    }

    pub fn envs(&self) -> Vec<(String, String)> {
        self.env_vector.clone()
    }

    pub fn l2_block_number(&self) -> String {
        self.l2_block_number.clone()
    }
}

#[tokio::test]
async fn test_orchestrator_workflow() {
    // Fetching the env vars from the test env file because setting up of the environment
    // requires all these variables.
    dotenvy::from_filename(".env.test").expect("Failed to load the .env file");

    let mut setup_config = Setup::new().await;

    // Impersonate account : Starknet operator
    let operator_address = get_env_var_or_panic("STARKNET_OPERATOR_ADDRESS");
    setup_config.ethereum_client().impersonate_account_as_address(Address::from_str(&operator_address).unwrap()).await;

    // Step 1 : SNOS job runs =========================================
    // TODO : Update the code with actual SNOS implementation
    // Updates the job in the db
    put_job_data_in_db_snos(setup_config.mongo_db_instance(), setup_config.l2_block_number()).await;

    // Step 2: Proving Job ============================================
    // Mocking the endpoint
    mock_proving_job_endpoint_output(setup_config.sharp_client()).await;

    // Step 3: DA job =================================================
    // mocking get_block_call from starknet client
    let l2_block_number = setup_config.l2_block_number();

    // Adding a mock da job so that worker does not create 60k+ jobs
    put_job_data_in_db_da(setup_config.mongo_db_instance(), l2_block_number.clone()).await;
    mock_starknet_get_state_update(setup_config.starknet_client(), l2_block_number.clone()).await;
    mock_starknet_get_nonce(setup_config.starknet_client(), l2_block_number.clone()).await;

    // Step 4: State Update job =======================================
    put_job_data_in_db_update_state(setup_config.mongo_db_instance(), l2_block_number).await;

    println!("✅ Orchestrator setup completed.");

    // Run orchestrator
    let mut orchestrator = Orchestrator::run(setup_config.envs());
    orchestrator.wait_till_started().await;

    // TODO : need to make this dynamic
    sleep(Duration::from_secs(900)).await;

    // Adding a case here to check for required state of the orchestrator to end the test.
    let l2_block_for_testing = get_env_var_or_panic("L2_BLOCK_NUMBER_FOR_TEST");
    let latest_job_in_db =
        get_database_state(setup_config.mongo_db_instance(), l2_block_for_testing.clone()).await.unwrap();
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

// ======================================
// Util functions
// ======================================

/// Puts after SNOS job state into the database
pub async fn put_job_data_in_db_snos(mongo_db: &MongoDbServer, l2_block_number: String) {
    let job_item = JobItem {
        id: Uuid::new_v4(),
        internal_id: l2_block_number,
        job_type: JobType::SnosRun,
        status: JobStatus::Completed,
        external_id: ExternalId::Number(0),
        metadata: HashMap::new(),
        version: 0,
    };

    let mongo_db_client = get_mongo_db_client(mongo_db).await;
    mongo_db_client.database("orchestrator").drop(None).await.unwrap();
    mongo_db_client.database("orchestrator").collection("jobs").insert_one(job_item, None).await.unwrap();
}

/// Mocks the endpoint for sharp client
pub async fn mock_proving_job_endpoint_output(sharp_client: &mut SharpClient) {
    // Add job response
    let add_job_response = json!(
        {
            "code" : "JOB_RECEIVED_SUCCESSFULLY"
        }
    );
    sharp_client.add_mock_on_endpoint("/add_job", vec!["".to_string()], Some(200), &add_job_response);

    // Getting job response
    let get_job_response = json!(
        {
                "status": "ONCHAIN",
                "validation_done": true
        }
    );
    sharp_client.add_mock_on_endpoint("/get_status", vec!["".to_string()], Some(200), &get_job_response);
}

/// Puts after SNOS job state into the database
pub async fn put_job_data_in_db_da(mongo_db: &MongoDbServer, l2_block_number: String) {
    let job_item = JobItem {
        id: Uuid::new_v4(),
        internal_id: (l2_block_number.parse::<u32>().unwrap() - 1).to_string(),
        job_type: JobType::DataSubmission,
        status: JobStatus::Completed,
        external_id: ExternalId::Number(0),
        metadata: HashMap::new(),
        version: 0,
    };

    let mongo_db_client = get_mongo_db_client(mongo_db).await;
    mongo_db_client.database("orchestrator").collection("jobs").insert_one(job_item, None).await.unwrap();
}

/// Mocks the starknet get nonce call (happens in da client for ethereum)
pub async fn mock_starknet_get_nonce(starknet_client: &mut StarknetClient, l2_block_number: String) {
    let mut file = File::open(format!("artifacts/nonces_{}.json", l2_block_number)).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    #[derive(Deserialize, Debug, Serialize)]
    struct NonceAddress {
        nonce: String,
        address: String,
    }

    // Parse the JSON string into a HashMap
    let vec: Vec<NonceAddress> = serde_json::from_str(&contents).unwrap();

    for ele in vec {
        let address = FieldElement::from_str(&ele.address).unwrap();
        let hex_field_element = vec_u8_to_hex_string(&address.to_bytes_be());

        let response = json!({ "id": 640641,"jsonrpc":"2.0","result": ele.nonce });
        starknet_client.add_mock_on_endpoint(
            "/",
            vec!["starknet_getNonce".to_string(), hex_field_element],
            Some(200),
            &response,
        );
    }
}

/// Mocks the starknet get state update call (happens in da client for ethereum)
pub async fn mock_starknet_get_state_update(starknet_client: &mut StarknetClient, l2_block_number: String) {
    let state_update = read_state_update_from_file(&format!("artifacts/get_state_update_{}.json", l2_block_number))
        .expect("issue while reading");

    let state_update = MaybePendingStateUpdate::Update(state_update);
    let state_update = serde_json::to_value(&state_update).unwrap();
    let response = json!({ "id": 640641,"jsonrpc":"2.0","result": state_update });

    starknet_client.add_mock_on_endpoint("/", vec!["starknet_getStateUpdate".to_string()], Some(200), &response);
}

/// Puts after SNOS job state into the database
pub async fn put_job_data_in_db_update_state(mongo_db: &MongoDbServer, l2_block_number: String) {
    let job_item = JobItem {
        id: Uuid::new_v4(),
        internal_id: (l2_block_number.parse::<u32>().unwrap() - 1).to_string(),
        job_type: JobType::StateTransition,
        status: JobStatus::Completed,
        external_id: ExternalId::Number(0),
        metadata: HashMap::new(),
        version: 0,
    };

    let mongo_db_client = get_mongo_db_client(mongo_db).await;
    mongo_db_client.database("orchestrator").collection("jobs").insert_one(job_item, None).await.unwrap();
}
