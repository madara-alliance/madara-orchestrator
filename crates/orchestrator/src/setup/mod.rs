use std::process::Command;
use std::sync::Arc;

use aws_config::environment::EnvironmentVariableCredentialsProvider;
use aws_config::{from_env, Region, SdkConfig};
use aws_credential_types::provider::ProvideCredentials;

use crate::alerts::aws_sns::AWSSNS;
use crate::alerts::Alerts;
use crate::cli::alert::AlertParams;
use crate::cli::aws_config::AWSConfigParams;
use crate::cli::queue::QueueParams;
use crate::cli::storage::StorageParams;
use crate::cli::RunCmd;
use crate::config::{get_aws_config, ProviderConfig};
use crate::cron::event_bridge::AWSEventBridge;
use crate::cron::{Cron, CronParams};
use crate::data_storage::aws_s3::AWSS3;
use crate::data_storage::DataStorage;
use crate::queue::sqs::SqsQueue;
use crate::queue::QueueProvider as _;

#[derive(Clone)]
pub enum SetupConfig {
    AWS(SdkConfig),
}

pub enum ConfigType {
    AWS(AWSConfigParams),
}

async fn setup_config_from_params(client_type: ConfigType) -> SetupConfig {
    match client_type {
        ConfigType::AWS(aws_config) => {
            let region_provider = Region::new(aws_config.aws_region);
            let creds = EnvironmentVariableCredentialsProvider::new().provide_credentials().await.unwrap();
            SetupConfig::AWS(from_env().region(region_provider).credentials_provider(creds).load().await)
        }
    }
}

// TODO : move this to main.rs after moving to clap.
pub async fn setup_cloud(run_cmd: &RunCmd) -> color_eyre::Result<()> {
    println!("Setting up cloud.");
    let aws_config = run_cmd.validate_aws_config_params().expect("Failed to validate AWS config params");
    let provider_config = Arc::new(ProviderConfig::AWS(Box::new(get_aws_config(&aws_config).await)));

    println!("Setting up data storage.");
    let data_storage_params = run_cmd.validate_storage_params().expect("Failed to validate storage params");

    match data_storage_params {
        StorageParams::AWSS3(aws_s3_params) => {
            let s3 = Box::new(AWSS3::new_with_params(&aws_s3_params, provider_config.clone()).await);
            s3.setup(&StorageParams::AWSS3(aws_s3_params.clone())).await?
        }
    }
    println!("Data storage setup completed ✅");

    println!("Setting up queues");
    let queue_params = run_cmd.validate_queue_params().expect("Failed to validate queue params");
    match queue_params {
        QueueParams::AWSSQS(aws_sqs_params) => {
            let config = setup_config_from_params(ConfigType::AWS(aws_config.clone())).await;
            let sqs = Box::new(SqsQueue::new_with_params(aws_sqs_params));
            sqs.setup(config).await?
        }
    }
    println!("Queues setup completed ✅");

    println!("Setting up cron");
    let cron_params = run_cmd.validate_cron_params().expect("Failed to validate cron params");
    match cron_params {
        CronParams::EventBridge(aws_event_bridge_params) => {
            let config = setup_config_from_params(ConfigType::AWS(aws_config)).await;
            let event_bridge = Box::new(AWSEventBridge::new_with_params(&aws_event_bridge_params));
            event_bridge.setup(config).await?
        }
    }
    println!("Cron setup completed ✅");

    println!("Setting up alerts.");
    let alert_params = run_cmd.validate_alert_params().expect("Failed to validate alert params");
    match alert_params {
        AlertParams::AWSSNS(aws_sns_params) => {
            let sns = Box::new(AWSSNS::new_with_params(&aws_sns_params, provider_config).await);
            sns.setup(AlertParams::AWSSNS(aws_sns_params.clone())).await?
        }
    }
    println!("Alerts setup completed ✅");

    Ok(())
}

pub async fn setup_db() -> color_eyre::Result<()> {
    // We run the js script in the folder root:
    tracing::info!("Setting up database.");

    Command::new("node").arg("migrate-mongo-config.js").output()?;

    tracing::info!("Database setup completed ✅");

    Ok(())
}
