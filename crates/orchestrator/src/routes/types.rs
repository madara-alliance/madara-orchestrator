use axum::response::Response;
use serde::{Deserialize, Serialize};

use super::error::JobRouteError;

#[derive(Deserialize)]
pub struct JobId {
    pub id: String,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: Option<String>,
}

impl ApiResponse {
    pub fn success() -> Self {
        Self { success: true, message: None }
    }

    pub fn error(message: String) -> Self {
        Self { success: false, message: Some(message) }
    }
}

pub type JobRouteResult = Result<Response, JobRouteError>;
