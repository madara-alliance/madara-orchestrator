use std::sync::Arc;

use async_trait::async_trait;
use opentelemetry::KeyValue;

use crate::config::Config;
use crate::jobs::create_job;
use crate::jobs::types::{JobStatus, JobType};
use crate::metrics::ORCHESTRATOR_METRICS;
use crate::workers::Worker;

pub struct ProvingWorker;

#[async_trait]
impl Worker for ProvingWorker {
    /// 1. Fetch all successful SNOS job runs that don't have a proving job
    /// 2. Create a proving job for each SNOS job run
    async fn run_worker(&self, config: Arc<Config>) -> color_eyre::Result<()> {
        tracing::trace!(log_type = "starting", category = "ProvingWorker", "ProvingWorker started.");

        let successful_snos_jobs = config
            .database()
            .get_jobs_without_successor(JobType::SnosRun, JobStatus::Completed, JobType::ProofCreation)
            .await?;

        tracing::info!(
            "Found {} successful snos jobs without proving jobs",
            successful_snos_jobs.len()
        );

        // get the max number of proof registration jobs that can be currently created because of max capacity.
        let max_cap = utils::env_utils::get_env_var_or_default("MADARA_ORCHESTRATOR_MAX_PARALLEL_PROVING_JOBS", "8");

        tracing::info!("Max capacity for proving jobs is {}", max_cap);

        tracing::debug!("Found {} successful SNOS jobs without proving jobs", successful_snos_jobs.len());


        let job_type = JobType::ProofCreation;
        let statuses = vec![JobStatus::Created, JobStatus::LockedForProcessing, JobStatus::PendingVerification, JobStatus::PendingRetry];

        let current_jobs = config.database().get_jobs_by_type_and_statuses(job_type, statuses).await?;

        let current_jobs_count = current_jobs.len();
        tracing::info!("Current jobs count: {}", current_jobs_count);

        let max_jobs = max_cap.parse::<usize>().unwrap_or(50);
        let remaining_capacity = max_jobs.saturating_sub(current_jobs_count);

        tracing::info!("max_jobs {}", max_jobs);

        tracing::info!("Remaining capacity for proving jobs is {}", remaining_capacity);

        // get the first remaining_capacity jobs from successful_snos_jobs
        let remaining_jobs = successful_snos_jobs.into_iter().take(remaining_capacity);

        tracing::info!("Creating jobs for {} proving jobs", remaining_jobs.len());

        for job in remaining_jobs {
            tracing::debug!(job_id = %job.internal_id, "Creating proof creation job for SNOS job");
            match create_job(JobType::ProofCreation, job.internal_id.to_string(), job.metadata, config.clone()).await {
                Ok(_) => tracing::info!(block_id = %job.internal_id, "Successfully created new proving job"),
                Err(e) => {
                    tracing::warn!(job_id = %job.internal_id, error = %e, "Failed to create new state transition job");
                    let attributes = [
                        KeyValue::new("operation_job_type", format!("{:?}", JobType::ProofCreation)),
                        KeyValue::new("operation_type", format!("{:?}", "create_job")),
                    ];
                    ORCHESTRATOR_METRICS.failed_job_operations.add(1.0, &attributes);
                }
            }
        }

        tracing::trace!(log_type = "completed", category = "ProvingWorker", "ProvingWorker completed.");
        Ok(())
    }
}
