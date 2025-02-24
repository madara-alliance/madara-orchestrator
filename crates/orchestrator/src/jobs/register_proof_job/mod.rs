use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{SubsecRound, Utc};
use color_eyre::eyre::{eyre, WrapErr};
use color_eyre::Result;
use prover_client_interface::{Task, TaskStatus};
use swiftness_proof_parser::{parse, StarkProof};
use uuid::Uuid;

use super::{Job, JobError, OtherError};
use crate::config::{self, Config};
use crate::constants::{PROOF_FILE_NAME, PROOF_PART2_FILE_NAME};
use crate::jobs::constants::JOB_METADATA_SNOS_FACT;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};

pub struct RegisterProofJob;

#[async_trait]
impl Job for RegisterProofJob {
    #[tracing::instrument(fields(category = "proof_registry"), skip(self, _config, metadata), ret, err)]
    async fn create_job(
        &self,
        _config: Arc<Config>,
        internal_id: String,
        metadata: HashMap<String, String>,
    ) -> Result<JobItem, JobError> {
        tracing::info!(log_type = "starting", category = "proof_registry", function_type = "create_job",  block_no = %internal_id, "Proof registration job creation started.");
        let job_item = JobItem {
            id: Uuid::new_v4(),
            internal_id: internal_id.clone(),
            job_type: JobType::ProofRegistration,
            status: JobStatus::Created,
            external_id: String::new().into(),
            // metadata must contain the blocks that have been included inside this proof
            // this will allow state update jobs to be created for each block
            metadata,
            version: 0,
            created_at: Utc::now().round_subsecs(0),
            updated_at: Utc::now().round_subsecs(0),
        };
        tracing::info!(log_type = "completed", category = "proof_registry", function_type = "create_job",  block_no = %internal_id,  "Proof registration job created.");
        Ok(job_item)
    }

    #[tracing::instrument(fields(category = "proof_registry"), skip(self, config), ret, err)]
    async fn process_job(&self, config: Arc<Config>, job: &mut JobItem) -> Result<String, JobError> {
        let internal_id = job.internal_id.clone();
        tracing::info!(
            log_type = "starting",
            category = "proof_registry",
            function_type = "process_job",
            job_id = ?job.id,
            block_no = %internal_id,
            "Proof registration job processing started."
        );

        // Get proof from storage
        let proof_key = format!("{internal_id}/{PROOF_FILE_NAME}");
        tracing::debug!(job_id = %job.internal_id, %proof_key, "Fetching proof file");

        let proof_file = config.storage().get_data(&proof_key).await.map_err(|e| {
            tracing::error!(job_id = %job.internal_id, error = %e, "Failed to fetch proof file");
            JobError::Other(OtherError(e))
        })?;

        let proof = String::from_utf8(proof_file.to_vec()).map_err(|e| {
            tracing::error!(job_id = %job.internal_id, error = %e, "Failed to parse proof file as UTF-8");
            JobError::Other(OtherError(eyre!("{}", e)))
        })?;

        let _: StarkProof = parse(proof.clone()).map_err(|e| {
            tracing::error!(job_id = %job.internal_id, error = %e, "Failed to parse proof file as UTF-8");
            JobError::Other(OtherError(eyre!("{}", e)))
        })?;

        // save the proof to a file
        let mut file = File::create("proof2.json").unwrap();
        file.write_all(proof.as_bytes()).unwrap();

        // Format proof for submission
        let formatted_proof = format!("{{\n\t\"proof\": {}\n}}", proof);

        let task_id = job.internal_id.clone();

        // Submit proof for L2 verification
        let external_id = config
            .prover_client()
            .submit_l2_query(&task_id, &formatted_proof)
            .await
            .wrap_err("Prover Client Error".to_string())
            .map_err(|e| {
                tracing::error!(job_id = %job.internal_id, error = %e, "Failed to submit proof for L2 verification");
                JobError::Other(OtherError(e))
            })?;

        tracing::info!(
            log_type = "completed",
            category = "proof_registry",
            function_type = "process_job",
            job_id = ?job.id,
            block_no = %internal_id,
            %external_id,
            "Proof registration job processed successfully."
        );
        Ok(external_id)
    }

    #[tracing::instrument(fields(category = "proof_registry"), skip(self, config), ret, err)]
    async fn verify_job(&self, config: Arc<Config>, job: &mut JobItem) -> Result<JobVerificationStatus, JobError> {
        let internal_id = job.internal_id.clone();
        tracing::info!(
            log_type = "starting",
            category = "proof_registry",
            function_type = "verify_job",
            job_id = ?job.id,
            block_no = %internal_id,
            "Proof registration job verification started."
        );

        let task_id: String = job
            .external_id
            .unwrap_string()
            .map_err(|e| {
                tracing::error!(job_id = %job.internal_id, error = %e, "Failed to unwrap external_id");
                JobError::Other(OtherError(e))
            })?
            .into();

        let fact = job.metadata.get(JOB_METADATA_SNOS_FACT).ok_or_else(|| {
            tracing::error!(job_id = %job.internal_id, "Fact not available in job metadata");
            OtherError(eyre!("Fact not available in job"))
        })?;

        tracing::debug!(job_id = %job.internal_id, %task_id, "Getting task status from prover client");
        let task_status = config
            .prover_client()
            .get_task_status(&task_id, fact)
            .await
            .wrap_err("Prover Client Error".to_string())
            .map_err(|e| {
                tracing::error!(job_id = %job.internal_id, error = %e, "Failed to get task status from prover client");
                JobError::Other(OtherError(e))
            })?;

        match task_status {
            TaskStatus::Processing => {
                tracing::info!(
                    log_type = "pending",
                    category = "proof_registry",
                    function_type = "verify_job",
                    job_id = ?job.id,
                    block_no = %internal_id,
                    "Proof registration job verification pending."
                );
                Ok(JobVerificationStatus::Pending)
            }
            TaskStatus::Succeeded => {
                // TODO: call isValid on the contract over here to cross-verify whether the proof was registered on
                // chain or not
                let fetched_proof = config.prover_client().get_proof(&task_id, fact).await
                .wrap_err("Prover Client Error".to_string())
                .map_err(|e| {
                    tracing::error!(job_id = %job.internal_id, error = %e, "Failed to get task status from prover client");
                    JobError::Other(OtherError(e))
                })?;

                let proof_key = format!("{internal_id}/{PROOF_PART2_FILE_NAME}");
                config.storage().put_data(bytes::Bytes::from(fetched_proof.into_bytes()), &proof_key).await.map_err(
                    |e| {
                        tracing::error!(job_id = %job.internal_id, error = %e, "Failed to store proof in S3");
                        JobError::Other(OtherError(e))
                    },
                )?;
                tracing::info!(
                    log_type = "completed",
                    category = "proof_registry",
                    function_type = "verify_job",
                    job_id = ?job.id,
                    block_no = %internal_id,
                    "Proof registration job verification completed."
                );
                Ok(JobVerificationStatus::Verified)
            }
            TaskStatus::Failed(err) => {
                tracing::info!(
                    log_type = "failed",
                    category = "proof_registry",
                    function_type = "verify_job",
                    job_id = ?job.id,
                    block_no = %internal_id,
                    "Proof registration job verification failed."
                );
                Ok(JobVerificationStatus::Rejected(format!(
                    "Proof registration job #{} failed with error: {}",
                    job.internal_id, err
                )))
            }
        }
    }

    fn max_process_attempts(&self) -> u64 {
        2
    }

    fn max_verification_attempts(&self) -> u64 {
        300
    }

    fn verification_polling_delay_seconds(&self) -> u64 {
        300
    }

    fn job_processing_lock(
        &self,
        _config: Arc<Config>,
    ) -> std::option::Option<std::sync::Arc<config::JobProcessingState>> {
        None
    }
}
