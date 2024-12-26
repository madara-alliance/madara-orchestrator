use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use opentelemetry::KeyValue;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ApiResponse;
use crate::config::Config;
use crate::jobs::types::{JobItemUpdates, JobStatus};
use crate::jobs::{process_job, verify_job, JobError};
use crate::metrics::ORCHESTRATOR_METRICS;

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
            return ApiResponse::<JobApiResponse>::error((JobError::InvalidId { id }).to_string()).into_response();
        }
    };

    // Process job
    match process_job(job_id, config).await {
        Ok(_) => {
            let response = JobApiResponse { job_id: job_id.to_string(), status: "completed".to_string() };
            ApiResponse::success(response).into_response()
        }
        Err(e) => {
            ORCHESTRATOR_METRICS.failed_job_operations.add(1.0, &[KeyValue::new("operation_type", "process_job")]);
            ApiResponse::<JobApiResponse>::error(e.to_string()).into_response()
        }
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
            return ApiResponse::<JobApiResponse>::error((JobError::InvalidId { id }).to_string()).into_response();
        }
    };

    // Verify job
    match verify_job(job_id, config).await {
        Ok(_) => {
            let response = JobApiResponse { job_id: job_id.to_string(), status: "verified".to_string() };
            ApiResponse::success(response).into_response()
        }
        Err(e) => {
            ORCHESTRATOR_METRICS.failed_job_operations.add(1.0, &[KeyValue::new("operation_type", "verify_job")]);
            ApiResponse::<JobApiResponse>::error(e.to_string()).into_response()
        }
    }
}

async fn handle_retry_job_request(
    Path(JobId { id }): Path<JobId>,
    State(config): State<Arc<Config>>,
) -> impl IntoResponse {
    let job_id = match Uuid::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return ApiResponse::<JobApiResponse>::error((JobError::InvalidId { id }).to_string()).into_response();
        }
    };

    // Get the job and verify it's in a failed state
    let job = match config.database().get_job_by_id(job_id).await {
        Ok(Some(job)) => job,
        Ok(None) => {
            return ApiResponse::<JobApiResponse>::error(JobError::JobNotFound { id: job_id }.to_string())
                .into_response();
        }
        Err(e) => {
            return ApiResponse::<JobApiResponse>::error(e.to_string()).into_response();
        }
    };

    // Check if job is in a failed state
    if job.status != JobStatus::Failed {
        return ApiResponse::<JobApiResponse>::error(format!(
            "Job {} cannot be retried: current status is {:?}",
            id, job.status
        ))
        .into_response();
    }

    // Update the job status to RetryAttempt
    match config.database().update_job(&job, JobItemUpdates::new().update_status(JobStatus::RetryAttempt).build()).await
    {
        Ok(_) => {
            // Process the job after successful status update
            match process_job(job_id, config).await {
                Ok(_) => {
                    let response =
                        JobApiResponse { job_id: job_id.to_string(), status: "retry_processing".to_string() };
                    ApiResponse::success(response).into_response()
                }
                Err(e) => {
                    ORCHESTRATOR_METRICS
                        .failed_job_operations
                        .add(1.0, &[KeyValue::new("operation_type", "retry_job")]);
                    ApiResponse::<JobApiResponse>::error(e.to_string()).into_response()
                }
            }
        }
        Err(e) => {
            ORCHESTRATOR_METRICS.failed_job_operations.add(1.0, &[KeyValue::new("operation_type", "retry_job")]);
            ApiResponse::<JobApiResponse>::error(e.to_string()).into_response()
        }
    }
}

pub fn job_router(config: Arc<Config>) -> Router {
    Router::new().nest("/jobs", trigger_router(config.clone()))
}

fn trigger_router(config: Arc<Config>) -> Router {
    Router::new()
        .route("/:id/process", get(handle_process_job_request))
        .route("/:id/verify", get(handle_verify_job_request))
        .route("/:id/retry", get(handle_retry_job_request))
        .with_state(config)
}
