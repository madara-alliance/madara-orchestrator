use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use opentelemetry::KeyValue;
use tracing::{error, info, instrument};
use uuid::Uuid;

use super::error::JobRouteError;
use super::types::{ApiResponse, JobId, JobRouteResult};
use crate::config::Config;
use crate::jobs::{process_job, retry_job, verify_job};
use crate::metrics::ORCHESTRATOR_METRICS;

#[instrument(skip(config), fields(job_id = %id))]
async fn handle_process_job_request(
    Path(JobId { id }): Path<JobId>,
    State(config): State<Arc<Config>>,
) -> JobRouteResult {
    let job_id = Uuid::parse_str(&id).map_err(|_| JobRouteError::InvalidId(id.clone()))?;

    match process_job(job_id, config).await {
        Ok(_) => {
            info!("Job processed successfully");
            ORCHESTRATOR_METRICS.successful_job_operations.add(1.0, &[KeyValue::new("operation_type", "process_job")]);

            Ok(Json(ApiResponse::success()).into_response())
        }
        Err(e) => {
            error!(error = %e, "Failed to process job");
            ORCHESTRATOR_METRICS.failed_job_operations.add(1.0, &[KeyValue::new("operation_type", "process_job")]);
            Err(JobRouteError::ProcessingError(e.to_string()))
        }
    }
}

#[instrument(skip(config), fields(job_id = %id))]
async fn handle_verify_job_request(
    Path(JobId { id }): Path<JobId>,
    State(config): State<Arc<Config>>,
) -> JobRouteResult {
    let job_id = Uuid::parse_str(&id).map_err(|_| JobRouteError::InvalidId(id.clone()))?;

    match verify_job(job_id, config).await {
        Ok(_) => {
            info!("Job verified successfully");
            ORCHESTRATOR_METRICS.successful_job_operations.add(1.0, &[KeyValue::new("operation_type", "verify_job")]);

            Ok(Json(ApiResponse::success()).into_response())
        }
        Err(e) => {
            error!(error = %e, "Failed to verify job");
            ORCHESTRATOR_METRICS.failed_job_operations.add(1.0, &[KeyValue::new("operation_type", "verify_job")]);
            Err(JobRouteError::ProcessingError(e.to_string()))
        }
    }
}

#[instrument(skip(config), fields(job_id = %id))]
async fn handle_retry_job_request(
    Path(JobId { id }): Path<JobId>,
    State(config): State<Arc<Config>>,
) -> JobRouteResult {
    let job_id = Uuid::parse_str(&id).map_err(|_| JobRouteError::InvalidId(id.clone()))?;
    println!("retry_job_request: {:?}", job_id);

    match retry_job(job_id, config).await {
        Ok(_) => {
            info!("Job retry initiated successfully");
            ORCHESTRATOR_METRICS.successful_job_operations.add(1.0, &[KeyValue::new("operation_type", "retry_job")]);

            Ok(Json(ApiResponse::success()).into_response())
        }
        Err(e) => {
            error!(error = %e, "Failed to retry job");
            ORCHESTRATOR_METRICS.failed_job_operations.add(1.0, &[KeyValue::new("operation_type", "retry_job")]);
            Err(JobRouteError::ProcessingError(e.to_string()))
        }
    }
}

/// Creates a router for job-related endpoints
pub fn job_router(config: Arc<Config>) -> Router {
    Router::new().nest("/jobs", trigger_router(config.clone()))
}

/// Creates the nested router for job trigger endpoints
fn trigger_router(config: Arc<Config>) -> Router {
    Router::new()
        .route("/:id/process", get(handle_process_job_request))
        .route("/:id/verify", get(handle_verify_job_request))
        .route("/:id/retry", get(handle_retry_job_request))
        .with_state(config)
}
