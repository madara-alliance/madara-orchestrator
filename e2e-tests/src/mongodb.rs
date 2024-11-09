use std::str::FromStr;

use orchestrator::database::mongodb::MongoDBValidatedArgs;
use url::Url;
#[allow(dead_code)]
pub struct MongoDbServer {
    endpoint: Url,
}

impl MongoDbServer {
    pub fn run(mongodb_params: MongoDBValidatedArgs) -> Self {
        Self { endpoint: Url::from_str(&mongodb_params.connection_url).unwrap() }
    }

    pub fn endpoint(&self) -> Url {
        self.endpoint.clone()
    }
}
