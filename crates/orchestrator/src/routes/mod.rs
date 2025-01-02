use std::net::SocketAddr;
use std::sync::Arc;

use app_routes::{app_router, handler_404};
use axum::Router;
use job_routes::job_router;

use crate::config::Config;

pub mod app_routes;
pub mod error;
pub mod job_routes;
pub mod types;

pub use error::JobRouteError;

#[derive(Debug, Clone)]
pub struct ServerParams {
    pub host: String,
    pub port: u16,
}

pub async fn setup_server(config: Arc<Config>) -> SocketAddr {
    let (api_server_url, listener) = get_server_url(config.server_config()).await;

    let job_routes = job_router(config.clone());
    let app_routes = app_router();
    let app = Router::new().merge(app_routes).merge(job_routes).fallback(handler_404);

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("Failed to start axum server");
    });

    api_server_url
}

pub async fn get_server_url(server_params: &ServerParams) -> (SocketAddr, tokio::net::TcpListener) {
    let address = format!("{}:{}", server_params.host, server_params.port);
    let listener = tokio::net::TcpListener::bind(address.clone()).await.expect("Failed to get listener");
    let api_server_url = listener.local_addr().expect("Unable to bind address to listener.");

    (api_server_url, listener)
}
