pub mod ethereum;
pub mod localstack;
pub mod mock_server;
pub mod mongodb;
pub mod node;
pub mod sharp;
pub mod starknet_client;

use ::mongodb::bson::doc;
use ::mongodb::options::{ClientOptions, ServerApi, ServerApiVersion};
use serde_json::json;
use starknet::core::types::{FieldElement, MaybePendingStateUpdate, StateUpdate};
use std::collections::HashMap;
use std::env::VarError;
use std::fs::File;
use std::io::Read;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::sharp::SharpClient;
use crate::starknet_client::StarknetClient;
pub use mongodb::MongoDbServer;
pub use node::Orchestrator;
pub use orchestrator::database::mongodb::MongoDb as MongoDbClient;
use orchestrator::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const MIN_PORT: u16 = 49_152;
const MAX_PORT: u16 = 65_535;

fn get_free_port() -> u16 {
    for port in MIN_PORT..=MAX_PORT {
        if let Ok(listener) = TcpListener::bind(("127.0.0.1", port)) {
            return listener.local_addr().expect("No local addr").port();
        }
        // otherwise port is occupied
    }
    panic!("No free ports available");
}

fn get_repository_root() -> PathBuf {
    let manifest_path = Path::new(&env!("CARGO_MANIFEST_DIR"));
    let repository_root = manifest_path.parent().expect("Failed to get parent directory of CARGO_MANIFEST_DIR");
    repository_root.to_path_buf()
}

pub async fn get_mongo_db_client(mongo_db: &MongoDbServer) -> ::mongodb::Client {
    let mut client_options = ClientOptions::parse(mongo_db.endpoint()).await.expect("Failed to parse MongoDB Url");
    // Set the server_api field of the client_options object to set the version of the Stable API on the
    // client
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);
    // Get a handle to the cluster
    let client = ::mongodb::Client::with_options(client_options).expect("Failed to create MongoDB client");
    // Ping the server to see if you can connect to the cluster
    client.database("admin").run_command(doc! {"ping": 1}, None).await.expect("Failed to ping MongoDB deployment");
    println!("Pinged your deployment. You successfully connected to MongoDB!");

    client
}

pub fn get_env_var(key: &str) -> Result<String, VarError> {
    std::env::var(key)
}

pub fn get_env_var_or_panic(key: &str) -> String {
    get_env_var(key).unwrap_or_else(|e| panic!("Failed to get env var {}: {}", key, e))
}

fn read_state_update_from_file(file_path: &str) -> color_eyre::Result<StateUpdate> {
    // let file_path = format!("state_update_block_no_{}.txt", block_no);
    let mut file = File::open(file_path)?;
    let mut json = String::new();
    file.read_to_string(&mut json)?;
    let state_update: StateUpdate = serde_json::from_str(&json)?;
    Ok(state_update)
}

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

/// Mocks the endpoint for sharp client
pub async fn mock_proving_job_endpoint_output(sharp_client: &mut SharpClient) {
    // Add job response
    let add_job_response = json!(
        {
            "code" : "JOB_RECEIVED_SUCCESSFULLY"
        }
    );
    sharp_client.add_mock_on_endpoint("/add_job", None, None, Some(200), &add_job_response);

    // Getting job response
    let get_job_response = json!(
        {
                "status": "ONCHAIN",
                "validation_done": true
        }
    );
    sharp_client.add_mock_on_endpoint("/get_status", None, None, Some(200), &get_job_response);
}

/// Mocks the starknet get state update call (happens in da client for ethereum)
pub async fn mock_starknet_get_state_update(starknet_client: &mut StarknetClient, l2_block_number: String) {
    let state_update = read_state_update_from_file(&format!("artifacts/get_state_update_{}.json", l2_block_number))
        .expect("issue while reading");

    let state_update = MaybePendingStateUpdate::Update(state_update);
    let state_update = serde_json::to_value(&state_update).unwrap();
    let response = json!({ "id": 640641,"jsonrpc":"2.0","result": state_update });

    starknet_client.add_mock_on_endpoint("/", Some("starknet_getStateUpdate"), None, Some(200), &response);
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
            Some("starknet_getNonce"),
            Some(&hex_field_element),
            Some(200),
            &response,
        );
    }
}

fn vec_u8_to_hex_string(data: &[u8]) -> String {
    let hex_chars: Vec<String> = data.iter().map(|byte| format!("{:02x}", byte)).collect();

    let mut new_hex_chars = hex_chars.join("");
    new_hex_chars = new_hex_chars.trim_start_matches('0').to_string();
    if new_hex_chars.is_empty() {
        "0x0".to_string()
    } else {
        format!("0x{}", new_hex_chars)
    }
}
