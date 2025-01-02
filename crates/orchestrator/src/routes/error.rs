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
    #[error("Invalid status: {id}: {job_status}")]
    InvalidStatus { id: String, job_status: String },
}

impl IntoResponse for JobRouteError {
    fn into_response(self) -> Response {
        match self {
            JobRouteError::InvalidId(id) => {
                (StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("Invalid job ID: {}", id)))).into_response()
            }
            JobRouteError::NotFound(id) => {
                (StatusCode::NOT_FOUND, Json(ApiResponse::error(format!("Job not found: {}", id)))).into_response()
            }
            JobRouteError::ProcessingError(msg) => {
                (StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("Processing error: {}", msg))))
                    .into_response()
            }
            JobRouteError::InvalidJobState(msg) => {
                (StatusCode::CONFLICT, Json(ApiResponse::error(format!("Invalid job state: {}", msg)))).into_response()
            }
            JobRouteError::DatabaseError => {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error("Database error occurred".to_string())))
                    .into_response()
            }
            JobRouteError::InvalidStatus { id, job_status } => (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(format!("Cannot retry job {id}: invalid status {job_status}"))),
            )
                .into_response(),
        }
    }
}
