pub mod client;
pub mod config;
pub mod error;
mod types;

use std::str::FromStr;

use alloy::primitives::B256;
use async_trait::async_trait;
use gps_fact_checker::FactChecker;
use prover_client_interface::{ProverClient, ProverClientError, Task, TaskStatus};
use tempfile::NamedTempFile;
use utils::settings::Settings;

use crate::client::AtlanticClient;
use crate::config::AtlanticConfig;
use crate::types::SharpQueryStatus;

pub const ATLANTIC_SETTINGS_NAME: &str = "atlantic";

/// Atlantic is a SHARP wrapper service hosted by Herodotus.
pub struct AtlanticProverService {
    pub atlantic_client: AtlanticClient,
    pub fact_checker: FactChecker,
}

#[async_trait]
impl ProverClient for AtlanticProverService {
    #[tracing::instrument(skip(self, task))]
    async fn submit_task(&self, task: Task) -> Result<String, ProverClientError> {
        tracing::info!(
            log_type = "starting",
            category = "submit_task",
            function_type = "cairo_pie",
            "Submitting Cairo PIE task."
        );
        match task {
            Task::CairoPie(cairo_pie) => {
                let temp_file =
                    NamedTempFile::new().map_err(|e| ProverClientError::FailedToCreateTempFile(e.to_string()))?;
                let pie_file_path = temp_file.path();
                cairo_pie
                    .write_zip_file(pie_file_path)
                    .map_err(|e| ProverClientError::FailedToWriteFile(e.to_string()))?;

                // sleep for 2 seconds to make sure the job is submitted
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                let atlantic_job_response = self.atlantic_client.add_job(pie_file_path).await?;
                log::debug!("Successfully submitted task to atlantic: {:?}", atlantic_job_response);
                // The temporary file will be automatically deleted when `temp_file` goes out of scope
                Ok(atlantic_job_response.sharp_query_id)
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn get_task_status(&self, job_key: &str, fact: &str) -> Result<TaskStatus, ProverClientError> {
        let res = self.atlantic_client.get_job_status(job_key).await?;

        match res.sharp_query.status {
            SharpQueryStatus::InProgress => Ok(TaskStatus::Processing),
            SharpQueryStatus::Done => {
                let fact = B256::from_str(fact).map_err(|e| ProverClientError::FailedToConvertFact(e.to_string()))?;
                if self.fact_checker.is_valid(&fact).await? {
                    Ok(TaskStatus::Succeeded)
                } else {
                    Ok(TaskStatus::Failed(format!("Fact {} is not valid or not registered", hex::encode(fact))))
                }
            }
            SharpQueryStatus::Failed => {
                Ok(TaskStatus::Failed("Task failed while processing on Atlantic side".to_string()))
            }
        }
    }
}

impl AtlanticProverService {
    pub fn new(atlantic_client: AtlanticClient, fact_checker: FactChecker) -> Self {
        Self { atlantic_client, fact_checker }
    }

    pub fn new_with_settings(settings: &impl Settings) -> Self {
        let atlantic_config = AtlanticConfig::new_with_settings(settings)
            .expect("Not able to create Atlantic Prover Service from given settings.");
        let atlantic_client =
            AtlanticClient::new_with_settings(atlantic_config.service_url, atlantic_config.settlement_layer);
        let fact_checker = FactChecker::new(atlantic_config.rpc_node_url, atlantic_config.verifier_address);
        log::debug!("Atlantic Client instantiated: {:?}", atlantic_client);
        log::debug!("Fact checker instantiated: {:?}", atlantic_client);

        Self::new(atlantic_client, fact_checker)
    }

    pub fn with_test_settings(settings: &impl Settings, port: u16) -> Self {
        let atlantic_config = AtlanticConfig::new_with_settings(settings)
            .expect("Not able to create SharpProverService from given settings.");
        let atlantic_client = AtlanticClient::new_with_settings(
            format!("http://127.0.0.1:{}", port).parse().unwrap(),
            atlantic_config.settlement_layer,
        );
        let fact_checker = FactChecker::new(atlantic_config.rpc_node_url, atlantic_config.verifier_address);
        Self::new(atlantic_client, fact_checker)
    }
}
