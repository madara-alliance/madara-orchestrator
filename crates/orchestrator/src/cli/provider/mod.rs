pub mod aws;

use aws::AWSConfigValidatedArgs;

#[derive(Debug, Clone)]
pub enum ProviderValidatedArgs {
    AWS(AWSConfigValidatedArgs),
}
