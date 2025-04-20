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
        tracing::trace!(
            log_type = "starting",
            category = "ProofRegistrationWorker",
            "ProofRegistrationWorker started."
        );

        let successful_proving_jobs = config
            .database()
            .get_jobs_without_successor(JobType::ProofCreation, JobStatus::Completed, JobType::ProofRegistration)
            .await?;

        tracing::debug!(
            "Found {} successful proving jobs without proof registration jobs",
            successful_proving_jobs.len()
        );

        // get the max number of proof registration jobs that can be currently created because of max capacity.
        let max_cap = utils::env_utils::get_env_var_or_default("MADARA_ORCHESTRATOR_MAX_CONCURRENT_PROOF_REGISTRATION_JOBS", "50");

        let job_type = JobType::ProofRegistration;
        let statuses = vec![JobStatus::Created, JobStatus::LockedForProcessing, JobStatus::PendingVerification, JobStatus::PendingRetry];

        let current_jobs = config.database().get_jobs_by_type_and_statuses(job_type, statuses).await?;

        let current_jobs_count = current_jobs.len();

        let max_jobs = max_cap.parse::<usize>().unwrap_or(50);
        let remaining_capacity = max_jobs.saturating_sub(current_jobs_count);

        // get the first remaining_capacity jobs from successful_proving_jobs
        let remaining_jobs = successful_proving_jobs.into_iter().take(remaining_capacity);

        tracing::info!("Creating jobs for {} proof registration jobs", remaining_jobs.len());

        for job in remaining_jobs {
            tracing::debug!(job_id = %job.internal_id, "Creating proof registration job for proving job");
            match create_job(JobType::ProofRegistration, job.internal_id.to_string(), job.metadata, config.clone())
                .await
            {
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

        tracing::trace!(
            log_type = "completed",
            category = "ProofRegistrationWorker",
            "ProofRegistrationWorker completed."
        );
        Ok(())
    }
}
