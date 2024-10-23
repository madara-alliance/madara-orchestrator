use core::panic;
use std::sync::Arc;

use chrono::{SubsecRound as _, Utc};
use hyper::{Body, Request};
use mockall::predicate::eq;
use rstest::*;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use url::Url;
use utils::env_utils::{get_env_var_or_default, get_env_var_or_panic};
use uuid::Uuid;

use crate::jobs::job_handler_factory::mock_factory;
use crate::jobs::types::{ExternalId, JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::{Job, MockJob};
use crate::queue::init_consumers;
use crate::routes::app_routes::{app_router, handler_404};
use crate::routes::job_routes::job_routes;
use crate::tests::config::{ConfigType, TestConfigBuilder};

#[tokio::test]
#[rstest]
#[case("process-job")]
#[case("verify-job")]
async fn test_trigger_job_endpoint(#[case] job_type: &str) {
    use axum::Router;

    dotenvy::from_filename("../.env.test").expect("Failed to load the .env.test file");

    let madara_url = get_env_var_or_panic("MADARA_RPC_URL");
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse(madara_url.as_str().to_string().as_str()).expect("Failed to parse URL"),
    ));

    let mut job_handler = MockJob::new();

    let mut job_item = build_job_item(JobType::DataSubmission, JobStatus::Created, 1);

    if job_type == "process-job" {
        // CHANGE
        job_handler.expect_process_job().times(1).returning(move |_, _| Ok("0xbeef".to_string()));
    } else {
        job_item = build_job_item(JobType::DataSubmission, JobStatus::PendingVerification, 2);
        job_handler.expect_verify_job().times(1).returning(move |_, _| Ok(JobVerificationStatus::Verified));
    }

    job_handler.expect_verification_polling_delay_seconds().return_const(1u64);

    let services = TestConfigBuilder::new()
        .configure_database(ConfigType::Actual)
        .configure_queue_client(ConfigType::Actual)
        .configure_starknet_client(provider.into())
        .build()
        .await;

    services.config.database().create_job(job_item.clone()).await.unwrap();

    let host = get_env_var_or_default("HOST", "0.0.0.0");
    let port = get_env_var_or_default("PORT", "3000").parse::<u16>().expect("PORT must be a u16");
    let address = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(address.clone()).await.expect("Failed to get listener");
    let addr = listener.local_addr().expect("Unable to bind address to listener.");

    let job_routes = job_routes(services.config.clone());
    let app_routes = app_router();

    let app = Router::new().merge(app_routes).merge(job_routes).fallback(handler_404);

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("Failed to start axum server");
    });

    let job_id = job_item.clone().id;

    let job_handler: Arc<Box<dyn Job>> = Arc::new(Box::new(job_handler));
    let ctx = mock_factory::get_job_handler_context();
    ctx.expect().times(1).with(eq(JobType::DataSubmission)).returning(move |_| Arc::clone(&job_handler));

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

    // assertions
    if let Some(job_fetched) = services.config.database().get_job_by_id(job_id).await.unwrap() {
        assert_eq!(response.status(), 200);
        assert_eq!(job_fetched.id, job_item.id);
        if job_type == "process-job" {
            assert_eq!(job_fetched.version, 2);
        } else {
            assert_eq!(job_fetched.version, 1);
        }
    } else {
        panic!("Could not get job from database")
    }
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
