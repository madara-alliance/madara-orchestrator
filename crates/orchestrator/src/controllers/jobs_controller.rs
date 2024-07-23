use std::collections::HashMap;

use axum::extract::Json;
use serde::Deserialize;

use crate::controllers::errors::AppError;
use crate::jobs::types::JobType;

/// Client request to create a job
#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    /// Job type
    pub job_type: JobType,
    /// Internal id must be a way to identify the job. For example
    /// block_no, transaction_hash etc. The (job_type, internal_id)
    /// pair must be unique.
    pub internal_id: String,
}

/// Create a job
pub async fn create_job(Json(payload): Json<CreateJobRequest>) -> Result<Json<()>, AppError> {
    crate::jobs::create_job(payload.job_type, payload.internal_id, HashMap::new()).await?;
    Ok(Json::from(()))
}
