use std::time::Duration;

use async_trait::async_trait;
use lazy_static::lazy_static;

use crate::queue::job_queue::{WorkerTriggerMessage, WorkerTriggerType};
use crate::setup::SetupConfig;

pub mod event_bridge;

lazy_static! {
    pub static ref CRON_DURATION: Duration = Duration::from_mins(1);
    pub static ref TARGET_QUEUE_NAME: String = String::from("madara_orchestrator_worker_trigger_queue");
    pub static ref WORKER_TRIGGERS: Vec<WorkerTriggerType> = vec![
        WorkerTriggerType::Snos,
        WorkerTriggerType::Proving,
        WorkerTriggerType::DataSubmission,
        WorkerTriggerType::UpdateState
    ];
}

#[async_trait]
pub trait Cron {
    async fn setup_cron(
        &self,
        config: &SetupConfig,
        cron_time: Duration,
        target_queue_name: String,
        message: String,
        worker_trigger_type: WorkerTriggerType,
    ) -> color_eyre::Result<()>;
    async fn setup(&self, config: SetupConfig) -> color_eyre::Result<()> {
        for triggers in WORKER_TRIGGERS.iter() {
            self.setup_cron(
                &config,
                *CRON_DURATION,
                TARGET_QUEUE_NAME.clone(),
                get_worker_trigger_message(triggers.clone())?,
                triggers.clone(),
            )
            .await?;
        }
        Ok(())
    }
}

fn get_worker_trigger_message(worker_trigger_type: WorkerTriggerType) -> color_eyre::Result<String> {
    let message = WorkerTriggerMessage { worker: worker_trigger_type };
    Ok(serde_json::to_string(&message)?)
}
