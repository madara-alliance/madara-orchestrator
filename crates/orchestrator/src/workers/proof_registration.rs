use std::sync::Arc;
use async_trait::async_trait;
use opentelemetry::KeyValue;

use crate::config::Config;
use crate::jobs::create_job;
use crate::jobs::types::{JobStatus, JobType};
use crate::metrics::ORCHESTRATOR_METRICS;
use crate::workers::Worker;

pub struct ProofRegistrationWorker;

#[async_trait]
impl Worker for ProofRegistrationWorker {
    async fn run_worker(&self, config: Arc<Config>) -> color_eyre::Result<()> {
        tracing::trace!(log_type = "starting", category = "ProofRegistrationWorker", "ProofRegistrationWorker started.");

        let successful_proving_jobs = config
            .database()
            .get_jobs_without_successor(JobType::ProofCreation, JobStatus::Completed, JobType::ProofRegistration)
            .await?;

        tracing::debug!("Found {} successful proving jobs without proof registration jobs", successful_proving_jobs.len());

        for job in successful_proving_jobs {
            tracing::debug!(job_id = %job.internal_id, "Creating proof registration job for proving job");
            match create_job(JobType::ProofRegistration, job.internal_id.to_string(), job.metadata, config.clone()).await {
                Ok(_) => tracing::info!(block_id = %job.internal_id, "Successfully created new proof registration job"),
                Err(e) => {
                    tracing::warn!(job_id = %job.internal_id, error = %e, "Failed to create new proof registration job");
                    let attributes = [
                        KeyValue::new("operation_job_type", format!("{:?}", JobType::ProofRegistration)),
                        KeyValue::new("operation_type", format!("{:?}", "create_job")),
                    ];
                    ORCHESTRATOR_METRICS.failed_job_operations.add(1.0, &attributes);
                }
            }
        }

        tracing::trace!(log_type = "completed", category = "ProofRegistrationWorker", "ProofRegistrationWorker completed.");
        Ok(())
    }
}
