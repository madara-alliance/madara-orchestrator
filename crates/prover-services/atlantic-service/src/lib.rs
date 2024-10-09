pub mod client;
use std::path::Path;
pub mod config;
pub mod error;
mod types;

use std::str::FromStr;

use alloy::primitives::B256;
use async_trait::async_trait;
use gps_fact_checker::FactChecker;
use prover_client_interface::{ProverClient, ProverClientError, Task, TaskStatus};
use utils::settings::Settings;
use uuid::Uuid;

use crate::client::AtlanticClient;
use crate::config::AtlanticConfig;
use crate::types::AtlanticJobStatus;

pub const ATLANTIC_SETTINGS_NAME: &str = "atlantic";

/// Atlantic is a SHARP wrapper service hosted by Herodotus.
pub struct AtlanticProverService {
    atlantic_client: AtlanticClient,
    fact_checker: FactChecker,
}

#[async_trait]
impl ProverClient for AtlanticProverService {
    #[tracing::instrument(skip(self, task))]
    async fn submit_task(&self, task: Task) -> Result<String, ProverClientError> {
        match task {
            Task::CairoPie(_) => {
                unimplemented!();
            }
            Task::CairoPieFilePath(cair_pie_file) => {
                let cairo_pie_path = Path::new(&cair_pie_file);
                let atlantic_job_reponse = self.atlantic_client.add_job(cairo_pie_path).await?;
                Ok(atlantic_job_reponse.sharp_query_id)
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn get_task_status(&self, job_key: &str, fact: &str) -> Result<TaskStatus, ProverClientError> {
        let job_key = Uuid::from_str(job_key)
            .map_err(|e| ProverClientError::InvalidJobKey(format!("Failed to convert {} to UUID {}", job_key, e)))?;
        let res = self.atlantic_client.get_job_status(&job_key).await?;

        match res.status {
            AtlanticJobStatus::RECIEVED => Ok(TaskStatus::Processing),
            AtlanticJobStatus::DONE => {
                let fact = B256::from_str(fact).map_err(|e| ProverClientError::FailedToConvertFact(e.to_string()))?;
                if self.fact_checker.is_valid(&fact).await? {
                    Ok(TaskStatus::Succeeded)
                } else {
                    Ok(TaskStatus::Failed(format!("Fact {} is not valid or not registered", hex::encode(fact))))
                }
            }
            AtlanticJobStatus::FAILED => Ok(TaskStatus::Failed(res.error_log.unwrap_or_default())),
        }
    }
}

impl AtlanticProverService {
    pub fn new(atlantic_client: AtlanticClient, fact_checker: FactChecker) -> Self {
        Self { atlantic_client, fact_checker }
    }

    pub fn new_with_settings(settings: &impl Settings) -> Self {
        let atlantic_config = AtlanticConfig::new_with_settings(settings)
            .expect("Not able to create SharpProverService from given settings.");
        let atlantic_client = AtlanticClient::new_with_settings(atlantic_config.service_url);
        let fact_checker = FactChecker::new(atlantic_config.rpc_node_url, atlantic_config.verifier_address);
        log::debug!("Atlantic Client instantiated: {:?}", atlantic_client);
        log::debug!("Fact checker instantiated: {:?}", atlantic_client);

        Self::new(atlantic_client, fact_checker)
    }

    pub fn with_test_settings(settings: &impl Settings, port: u16) -> Self {
        let atlantic_config = AtlanticConfig::new_with_settings(settings)
            .expect("Not able to create SharpProverService from given settings.");
        let atlantic_client = AtlanticClient::new_with_settings(format!("http://127.0.0.1:{}", port).parse().unwrap());
        let fact_checker = FactChecker::new(atlantic_config.rpc_node_url, atlantic_config.verifier_address);
        Self::new(atlantic_client, fact_checker)
    }
}
