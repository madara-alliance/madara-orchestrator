use std::str::FromStr;

use orchestrator::database::mongodb::config::MongoDBParams;
use url::Url;
#[allow(dead_code)]
pub struct MongoDbServer {
    endpoint: Url,
}

impl MongoDbServer {
    pub fn run(mongodb_params: MongoDBParams) -> Self {
        Self { endpoint: Url::from_str(&mongodb_params.connection_url).unwrap() }
    }

    pub fn endpoint(&self) -> Url {
        self.endpoint.clone()
    }
}
