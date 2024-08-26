use std::str::FromStr;

use url::Url;

#[allow(dead_code)]
pub struct MongoDbServer {
    // container: ContainerAsync<GenericImage>,
    endpoint: Url,
}

impl MongoDbServer {
    pub async fn run() -> Self {
        // let host_port = get_free_port();
        //
        // let container = GenericImage::new(MONGODB_IMAGE_NAME, MONGODB_IMAGE_TAG)
        //     .with_wait_for(WaitFor::message_on_stdout("Waiting for connections"))
        //     .with_mapped_port(host_port, ContainerPort::Tcp(MONGODB_DEFAULT_PORT))
        //     .start()
        //     .await
        //     .expect("Failed to create docker container");
        Self { endpoint: Url::from_str("mongodb://localhost:27017").unwrap() }
    }

    pub fn endpoint(&self) -> Url {
        Url::from_str("mongodb://localhost:27017").unwrap()
    }
}
