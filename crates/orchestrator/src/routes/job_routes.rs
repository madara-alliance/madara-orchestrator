use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;
use crate::jobs::{process_job, verify_job, JobError};
use super::ApiResponse;

#[derive(Deserialize)]
struct JobId {
    id: String,
}

#[derive(Serialize)]
struct JobApiResponse {
    job_id: String,
    status: String,
}


async fn handle_process_job_request(
    Path(JobId { id }): Path<JobId>,
    State(config): State<Arc<Config>>,
) -> impl IntoResponse {
    // Parse UUID
    let job_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return ApiResponse::<JobApiResponse>::error(
                (JobError::InvalidId { id }).to_string()
            ).into_response()
        }
    };

    // Process job
    match process_job(job_id, config).await {
        Ok(_) => {
            let response = JobApiResponse {
                job_id: job_id.to_string(),
                status: "completed".to_string(),
            };
            ApiResponse::success(response).into_response()
        },
        Err(e) => ApiResponse::<JobApiResponse>::error(e.to_string()).into_response()
    }
}

async fn handle_verify_job_request(
    Path(JobId { id }): Path<JobId>,
    State(config): State<Arc<Config>>,
) -> impl IntoResponse {
    // Parse UUID
    let job_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return ApiResponse::<JobApiResponse>::error(
                (JobError::InvalidId { id }).to_string()
            ).into_response()
        }
    };

    // Verify job
    match verify_job(job_id, config).await {
        Ok(_) => {
            let response = JobApiResponse {
                job_id: job_id.to_string(),
                status: "verified".to_string(),
            };
            ApiResponse::success(response).into_response()
        },
        Err(e) => ApiResponse::<JobApiResponse>::error(e.to_string()).into_response()
    }
}
pub fn job_routes(config: Arc<Config>) -> Router {
    Router::new().nest("/jobs", trigger_routes(config.clone()))
}

fn trigger_routes(config: Arc<Config>) -> Router {
    Router::new()
        .route("/:id/process", get(handle_process_job_request))
        .route("/:id/verify", get(handle_verify_job_request))
        .with_state(config)
}
