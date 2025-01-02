use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use super::types::ApiResponse;

#[derive(Debug, thiserror::Error)]
pub enum JobRouteError {
    #[error("Invalid job ID: {0}")]
    InvalidId(String),
    #[error("Job not found: {0}")]
    NotFound(String),
    #[error("Job processing error: {0}")]
    ProcessingError(String),
    #[error("Invalid job state: {0}")]
    InvalidJobState(String),
    #[error("Database error")]
    DatabaseError,
}

impl IntoResponse for JobRouteError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            JobRouteError::InvalidId(id) => (StatusCode::BAD_REQUEST, format!("Invalid job ID: {}", id)),
            JobRouteError::NotFound(id) => (StatusCode::NOT_FOUND, format!("Job not found: {}", id)),
            JobRouteError::ProcessingError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Processing error: {}", msg))
            }
            JobRouteError::InvalidJobState(msg) => (StatusCode::CONFLICT, format!("Invalid job state: {}", msg)),
            JobRouteError::DatabaseError => (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred".to_string()),
        };

        (status, Json(ApiResponse::error(message))).into_response()
    }
}
