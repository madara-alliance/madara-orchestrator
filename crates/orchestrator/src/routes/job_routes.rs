use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use uuid::Uuid;

use crate::config::Config;
use crate::jobs::{process_job, verify_job, JobError};

#[derive(Deserialize)]
struct JobParams {
    id: String,
}

struct JobResult(Result<(), JobError>);

impl IntoResponse for JobResult {
    fn into_response(self) -> Response {
        match self.0 {
            Ok(_) => (StatusCode::OK, "Job processing completed.").into_response(),
            Err(e) => match e {
                JobError::InvalidId { id } => {
                    (StatusCode::BAD_REQUEST, (JobError::InvalidId { id }).to_string()).into_response()
                }
                other_error => (StatusCode::INTERNAL_SERVER_ERROR, other_error.to_string()).into_response(),
            },
        }
    }
}

async fn handle_process_job_request(
    Query(params): Query<JobParams>,
    State(config): State<Arc<Config>>,
) -> impl IntoResponse {
    let id = match Uuid::parse_str(&params.id) {
        Ok(id) => id,
        Err(_) => return JobResult(Err(JobError::InvalidId { id: params.id })).into_response(),
    };

    JobResult(process_job(id, config).await).into_response()
}

async fn handle_verify_job_request(
    Query(params): Query<JobParams>,
    State(config): State<Arc<Config>>,
) -> impl IntoResponse {
    let id = match Uuid::parse_str(&params.id) {
        Ok(id) => id,
        Err(_) => return JobResult(Err(JobError::InvalidId { id: params.id })).into_response(),
    };

    JobResult(verify_job(id, config).await).into_response()
}

pub fn job_routes(config: Arc<Config>) -> Router {
    Router::new().nest("/trigger", trigger_routes(config.clone()))
}

fn trigger_routes(config: Arc<Config>) -> Router {
    Router::new()
        .route("/process-job", get(handle_process_job_request))
        .route("/verify-job", get(handle_verify_job_request))
        .with_state(config)
}
