use testcontainers::{core::WaitFor, Image};

const NAME: &str = "localstack/localstack";
const TAG: &str = "latest";

#[derive(Default, Debug, Clone)]
pub struct LocalStack {
    _priv: (),
}

impl Image for LocalStack {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready.")]
    }
}
