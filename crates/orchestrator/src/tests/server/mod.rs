pub mod job_routes;
use std::io::Read;
use std::net::SocketAddr;

use axum::http::StatusCode;
use dotenvy::dotenv;
use hyper::body::Buf;
use hyper::{Body, Request};
use rstest::*;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use url::Url;
use utils::env_utils::{get_env_var_or_default, get_env_var_or_panic};

use crate::queue::init_consumers;
use crate::routes::app_routes::app_router;
use crate::tests::config::TestConfigBuilder;

#[fixture]
pub async fn setup_app_server() -> SocketAddr {
    dotenv().ok();
    let madara_url = get_env_var_or_panic("MADARA_RPC_URL");
    let provider = JsonRpcClient::new(HttpTransport::new(
        Url::parse(madara_url.as_str().to_string().as_str()).expect("Failed to parse URL"),
    ));

    TestConfigBuilder::new().configure_starknet_client(provider.into()).build().await;

    let host = get_env_var_or_default("HOST", "127.0.0.1");
    let port = get_env_var_or_default("PORT", "3000").parse::<u16>().expect("PORT must be a u16");
    let address = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(address.clone()).await.expect("Failed to get listener");
    let addr = listener.local_addr().expect("Unable to bind address to listener.");
    let app = app_router();

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("Failed to start axum server");
    });

    addr
}

#[rstest]
#[tokio::test]
async fn test_health_endpoint(#[future] setup_app_server: SocketAddr) {
    let addr = setup_app_server.await;

    let client = hyper::Client::new();
    let response = client
        .request(Request::builder().uri(format!("http://{}/health", addr)).body(Body::empty()).unwrap())
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
