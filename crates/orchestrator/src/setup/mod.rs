mod worker_triggers;

use std::process::Command;
use std::sync::Arc;

use aws_config::environment::EnvironmentVariableCredentialsProvider;
use aws_config::{from_env, Region, SdkConfig};
use aws_credential_types::provider::ProvideCredentials;
use utils::env_utils::get_env_var_or_panic;
use utils::settings::env::EnvSettingsProvider;
use utils::settings::Settings;

use crate::alerts::aws_sns::AWSSNS;
use crate::alerts::Alerts;
use crate::config::{get_aws_config, ProviderConfig};
use crate::data_storage::aws_s3::AWSS3;
use crate::data_storage::DataStorage;
use crate::queue::job_queue::WorkerTriggerType;
use crate::queue::QueueProvider;
use crate::setup::worker_triggers::event_bridge::setup_event_bridge_for_trigger_type;

#[derive(Clone)]
pub enum SetupConfig {
    AWS(SdkConfig),
}

pub enum ConfigType {
    AWS,
}

const WORKER_TRIGGER_RULE: &str = "worker_trigger_scheduled";
const WORKER_TRIGGER_QUEUE_NAME: &str = "madara_orchestrator_worker_trigger_queue";
const QUEUE_VISIBILITY_TIMEOUT: u32 = 1800;
const QUEUE_MAX_RECEIVE_COUNT: u32 = 5;

async fn setup_config(client_type: ConfigType) -> SetupConfig {
    match client_type {
        ConfigType::AWS => {
            let region_provider = Region::new(get_env_var_or_panic("AWS_REGION"));
            let creds = EnvironmentVariableCredentialsProvider::new().provide_credentials().await.unwrap();
            SetupConfig::AWS(from_env().region(region_provider).credentials_provider(creds).load().await)
        }
    }
}

pub async fn setup_cloud() -> color_eyre::Result<()> {
    log::info!("Setting up cloud.");
    let settings_provider = EnvSettingsProvider {};
    let provider_config = Arc::new(ProviderConfig::AWS(Box::new(get_aws_config(&settings_provider).await)));

    log::info!("Setting up data storage.");
    match get_env_var_or_panic("DATA_STORAGE").as_str() {
        "s3" => {
            let s3 = Box::new(AWSS3::new_with_settings(&settings_provider, provider_config.clone()).await);
            s3.setup(&settings_provider.get_settings_or_panic("AWS_S3_BUCKET_NAME")).await?
        }
        _ => panic!("Unsupported Storage Client"),
    }
    log::info!("Data storage setup completed ✅");

    log::info!("Setting up queues");
    match get_env_var_or_panic("QUEUE_PROVIDER").as_str() {
        "sqs" => {
            let config = setup_config(ConfigType::AWS).await;
            let sqs = Box::new(crate::queue::sqs::SqsQueue {});
            sqs.setup(config, QUEUE_VISIBILITY_TIMEOUT, QUEUE_MAX_RECEIVE_COUNT).await?
        }
        _ => panic!("Unsupported Queue Client"),
    }
    log::info!("Queues setup completed ✅");

    log::info!("Setting up event bridge");
    match get_env_var_or_panic("EVENT_BRIDGE_PROVIDER").as_str() {
        "event_bridge" => {
            let config = setup_config(ConfigType::AWS).await;
            setup_event_bridge_for_trigger_type(
                WorkerTriggerType::Snos,
                &config,
                WORKER_TRIGGER_RULE,
                WORKER_TRIGGER_QUEUE_NAME,
            )
            .await?;
            setup_event_bridge_for_trigger_type(
                WorkerTriggerType::Proving,
                &config,
                WORKER_TRIGGER_RULE,
                WORKER_TRIGGER_QUEUE_NAME,
            )
            .await?;
            setup_event_bridge_for_trigger_type(
                WorkerTriggerType::DataSubmission,
                &config,
                WORKER_TRIGGER_RULE,
                WORKER_TRIGGER_QUEUE_NAME,
            )
            .await?;
            setup_event_bridge_for_trigger_type(
                WorkerTriggerType::UpdateState,
                &config,
                WORKER_TRIGGER_RULE,
                WORKER_TRIGGER_QUEUE_NAME,
            )
            .await?;
        }
        _ => panic!("Unsupported Event Bridge Client"),
    }

    log::info!("Setting up alerts.");
    match get_env_var_or_panic("ALERTS").as_str() {
        "sns" => {
            let sns = Box::new(AWSSNS::new_with_settings(&settings_provider, provider_config).await);
            sns.setup().await?
        }
        _ => panic!("Unsupported Alert Client"),
    }
    log::info!("Alerts setup completed ✅");

    Ok(())
}

pub async fn setup_db() -> color_eyre::Result<()> {
    // We run the js script in the folder root:
    log::info!("Setting up database.");

    Command::new("node").arg("migrate-mongo-config.js").output()?;

    log::info!("Database setup completed ✅");

    Ok(())
}
