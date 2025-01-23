use std::sync::Arc;

use async_trait::async_trait;
use opentelemetry::KeyValue;

use crate::config::Config;
use crate::jobs::create_job;
use crate::jobs::metadata::{CommonMetadata, JobMetadata, JobSpecificMetadata, ProvingMetadata};
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

        tracing::debug!("Found {} successful SNOS jobs without proving jobs", successful_snos_jobs.len());

        for snos_job in successful_snos_jobs {
            // Extract SNOS metadata
            let snos_metadata = match &snos_job.metadata.specific {
                JobSpecificMetadata::Snos(metadata) => metadata,
                _ => {
                    tracing::error!(job_id = %snos_job.internal_id, "Invalid metadata type for SNOS job");
                    continue;
                }
            };

            // Get SNOS fact early to handle the error case
            let snos_fact = match &snos_metadata.snos_fact {
                Some(fact) => fact.clone(),
                None => {
                    tracing::error!(job_id = %snos_job.internal_id, "SNOS fact not found in metadata");
                    continue;
                }
            };

            // Create proving job metadata
            let proving_metadata = JobMetadata {
                common: CommonMetadata::default(),
                specific: JobSpecificMetadata::Proving(ProvingMetadata {
                    block_number: snos_metadata.block_number,
                    // Get paths from SNOS job
                    cairo_pie_path: snos_metadata.cairo_pie_path.clone(),
                    snos_fact, // Use the extracted fact

                    // Set new paths for proving outputs
                    proof_path: None,
                    verification_key_path: None,
                    // Proving specific settings
                    cross_verify: true,
                    download_proof: false, // Default to false, can be configured
                }),
            };

            tracing::debug!(job_id = %snos_job.internal_id, "Creating proof creation job for SNOS job");
            match create_job(JobType::ProofCreation, snos_job.internal_id.clone(), proving_metadata, config.clone())
                .await
            {
                Ok(_) => tracing::info!(block_id = %snos_job.internal_id, "Successfully created new proving job"),
                Err(e) => {
                    tracing::warn!(job_id = %snos_job.internal_id, error = %e, "Failed to create new proving job");
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
