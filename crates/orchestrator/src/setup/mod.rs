use std::process::Command;
use std::sync::Arc;

use utils::env_utils::get_env_var_or_panic;
use utils::settings::env::EnvSettingsProvider;
use utils::settings::Settings;

use crate::alerts::aws_sns::AWSSNS;
use crate::alerts::Alerts;
use crate::config::{get_aws_config, ProviderConfig};
use crate::data_storage::aws_s3::AWSS3;
use crate::data_storage::DataStorage;
use crate::queue::QueueProvider;

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

    log::info!("Setting up queues and event bridge.");
    match get_env_var_or_panic("QUEUE_PROVIDER").as_str() {
        "sqs" => {
            let sqs = Box::new(crate::queue::sqs::SqsQueue {});
            sqs.setup().await?
        }
        _ => panic!("Unsupported Queue Client"),
    }
    log::info!("Queues and Event Bridge setup completed ✅");

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
