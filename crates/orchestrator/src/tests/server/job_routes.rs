use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::StatusCode;
use chrono::{SubsecRound as _, Utc};
use dotenvy::dotenv;
use hyper::body::Buf;
use hyper::{Body, Request};
use rstest::*;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use url::Url;
use utils::env_utils::{get_env_var_or_default, get_env_var_or_panic};
use uuid::Uuid;

use crate::config::{init_config, Config};
use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType};
use crate::queue::init_consumers;
use crate::routes::job_routes::job_routes;
use crate::tests::config::TestConfigBuilder;

#[fixture]
pub async fn setup_job_server() -> (SocketAddr, Arc<Config>) {
    dotenv().ok();
    let madara_url = get_env_var_or_panic("MADARA_RPC_URL");
    let provider =
        JsonRpcClient::new(HttpTransport::new(Url::parse(madara_url.as_str()).expect("Failed to parse URL")));

    TestConfigBuilder::new().configure_starknet_client(provider.into()).build().await;

    let host = get_env_var_or_default("HOST", "127.0.0.1");
    let port = get_env_var_or_default("PORT", "3000").parse::<u16>().expect("PORT must be a u16");
    let address = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(address.clone()).await.expect("Failed to get listener");
    let addr = listener.local_addr().expect("Unable to bind address to listener.");

    let config = init_config().await.expect("Config instantiation failed");
    let job_routes = job_routes(config.clone());

    tokio::spawn(async move {
        axum::serve(listener, job_routes).await.expect("Failed to start axum server");
    });

    (addr, config.clone())
}

#[rstest]
#[tokio::test]
#[case("process_job")]
#[case("verify_job")]
async fn test_trigger_job_endpoint(#[future] setup_job_server: (SocketAddr, Arc<Config>), #[case] job_type: &str) {
    use crate::jobs::types::{JobStatus, JobType};

    let (addr, config) = setup_job_server.await;

    let job_one = build_job_item(JobType::ProofCreation, JobStatus::Created, 1);
    config.database().create_job(job_one.clone()).await.unwrap();

    let job_id = job_one.id.to_string();

    let client = hyper::Client::new();
    let response = client
        .request(
            Request::builder()
                .uri(format!("http://{}/trigger/{job_type}?id={job_id}", addr))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status().as_str(), StatusCode::OK.as_str());

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let mut buf = String::new();
    let res = body.reader().read_to_string(&mut buf).unwrap();
    assert_eq!(res, 2);
}

#[rstest]
#[tokio::test]
async fn test_init_consumer() {
    let services = TestConfigBuilder::new().build().await;
    assert!(init_consumers(services.config).await.is_ok());
}

// Test Util Functions
// ==========================================

pub fn build_job_item(job_type: JobType, job_status: JobStatus, internal_id: u64) -> JobItem {
    JobItem {
        id: Uuid::new_v4(),
        internal_id: internal_id.to_string(),
        job_type,
        status: job_status,
        external_id: ExternalId::Number(0),
        metadata: Default::default(),
        version: 0,
        created_at: Utc::now().round_subsecs(0),
        updated_at: Utc::now().round_subsecs(0),
    }
}
