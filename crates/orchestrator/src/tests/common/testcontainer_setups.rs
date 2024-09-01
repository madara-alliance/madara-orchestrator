use testcontainers::{
    core::{CmdWaitFor, ExecCommand, WaitFor},
    Image,
};

const DEFAULT_WAIT: u64 = 3000;
/// LocalStack using TestContainers ////
#[derive(Default, Debug, Clone)]
pub struct LocalStack {
    _priv: (),
}

impl Image for LocalStack {
    /// Informs docker which Image to load.
    fn name(&self) -> &str {
        "localstack/localstack"
    }

    /// Informs docker which version of image to load.
    fn tag(&self) -> &str {
        "latest"
    }

    /// Waits for these conditions to be met before interacting with the Image.
    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready."), WaitFor::millis(DEFAULT_WAIT)]
    }

    // fn env_vars(
    //         &self,
    //     ) -> impl IntoIterator<Item = (impl Into<std::borrow::Cow<'_, str>>, impl Into<std::borrow::Cow<'_, str>>)> {

    //         let mut env_vars = HashMap::new();
    //         env_vars.insert("LS_LOG".to_owned(), "debug".to_owned());
    //         env_vars
    // }
}

/// Mongo using TestContainers ////
#[derive(Debug, Clone)]
enum InstanceKind {
    Standalone,
    ReplSet,
}

impl Default for InstanceKind {
    fn default() -> Self {
        Self::Standalone
    }
}

#[derive(Default, Debug, Clone)]
pub struct Mongo {
    kind: InstanceKind,
}

impl Mongo {
    pub fn new() -> Self {
        Self { kind: InstanceKind::Standalone }
    }
    pub fn repl_set() -> Self {
        Self { kind: InstanceKind::ReplSet }
    }
}

impl Image for Mongo {
    /// Informs docker which Image to load.
    fn name(&self) -> &str {
        "mongo"
    }

    /// Informs docker which version of image to load.
    fn tag(&self) -> &str {
        "latest"
    }

    /// Waits for these conditions to be met before interacting with the Image.
    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Waiting for connections"), WaitFor::millis(DEFAULT_WAIT)]
    }

    fn cmd(&self) -> impl IntoIterator<Item = impl Into<std::borrow::Cow<'_, str>>> {
        match self.kind {
            InstanceKind::Standalone => Vec::<String>::new(),
            InstanceKind::ReplSet => vec!["--replSet".to_string(), "rs".to_string()],
        }
    }

    fn exec_after_start(
        &self,
        _: testcontainers::core::ContainerState,
    ) -> Result<Vec<ExecCommand>, testcontainers::TestcontainersError> {
        match self.kind {
            InstanceKind::Standalone => Ok(Default::default()),
            InstanceKind::ReplSet => Ok(vec![ExecCommand::new(vec![
                "mongosh".to_string(),
                "--quiet".to_string(),
                "--eval".to_string(),
                "'rs.initiate()'".to_string(),
            ])
            .with_cmd_ready_condition(CmdWaitFor::message_on_stdout("Using a default configuration for the set"))
            .with_container_ready_conditions(vec![WaitFor::message_on_stdout(
                "Rebuilding PrimaryOnlyService due to stepUp",
            )])]),
        }
    }
}
