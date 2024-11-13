use std::io::{BufRead, BufReader};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;

use strum_macros::Display;
use tokio::net::TcpStream;
use url::Url;

use crate::get_free_port;
use crate::utils::get_repository_root;

const CONNECTION_ATTEMPTS: usize = 720;
const CONNECTION_ATTEMPT_DELAY_MS: u64 = 1000;

#[derive(Debug)]
pub struct Orchestrator {
    process: Child,
    address: String,
}

impl Drop for Orchestrator {
    fn drop(&mut self) {
        let mut kill =
            Command::new("kill").args(["-s", "TERM", &self.process.id().to_string()]).spawn().expect("Failed to kill");
        kill.wait().expect("Failed to kill the process");
    }
}

#[derive(Display, Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorMode {
    #[strum(serialize = "run")]
    Run,
    #[strum(serialize = "setup")]
    Setup,
}

impl Orchestrator {
    pub fn new(mode: OrchestratorMode, mut envs: Vec<(String, String)>) -> Option<Self> {
        let repository_root = &get_repository_root();
        std::env::set_current_dir(repository_root).expect("Failed to change working directory");

        let (mode_str, is_run_mode) = match mode {
            OrchestratorMode::Setup => {
                println!("Running orchestrator in Setup mode");
                (OrchestratorMode::Setup.to_string(), false)
            }
            OrchestratorMode::Run => {
                println!("Running orchestrator in Run mode");
                (OrchestratorMode::Run.to_string(), true)
            }
        };

        // Configure common command arguments
        let mut command = Command::new("cargo");
        command
            .arg("run")
            .arg("--release")
            .arg("--bin")
            .arg("orchestrator")
            .arg("--features")
            .arg("testing")
            .arg(mode_str)
            .arg("--")
            .arg("--aws")
            .arg("--settle-on-ethereum")
            .arg("--aws-s3")
            .arg("--aws-sqs")
            .arg("--aws-sns")
            .arg("--mongodb")
            .arg("--sharp")
            .arg("--da-on-ethereum");

        // Add event bridge arg only for setup mode
        if !is_run_mode {
            command.arg("--aws-event-bridge");
        }

        // Configure run-specific settings
        let address = if is_run_mode {
            let port = get_free_port();
            let addr = format!("127.0.0.1:{}", port);
            envs.push(("MADARA_ORCHESTRATOR_PORT".to_string(), port.to_string()));
            addr
        } else {
            String::new()
        };

        command.current_dir(repository_root).envs(envs).stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut process = command.spawn().expect("Failed to start process");

        if is_run_mode {
            // Set up stdout and stderr handling for run mode
            let stdout = process.stdout.take().expect("Failed to capture stdout");
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                reader.lines().for_each(|line| {
                    if let Ok(line) = line {
                        println!("STDOUT: {}", line);
                    }
                });
            });

            let stderr = process.stderr.take().expect("Failed to capture stderr");
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                reader.lines().for_each(|line| {
                    if let Ok(line) = line {
                        eprintln!("STDERR: {}", line);
                    }
                });
            });

            Some(Self { process, address })
        } else {
            // Handle setup mode
            let status = process.wait().expect("Failed to wait for process");
            if status.success() {
                println!("Setup Orchestrator completed successfully");
            } else if let Some(code) = status.code() {
                println!("Setup Orchestrator failed with exit code: {}", code);
            } else {
                println!("Setup Orchestrator terminated by signal");
            }
            None
        }
    }

    pub fn endpoint(&self) -> Url {
        Url::parse(&format!("http://{}", self.address)).unwrap()
    }

    pub fn has_exited(&mut self) -> Option<ExitStatus> {
        self.process.try_wait().expect("Failed to get orchestrator node exit status")
    }

    pub async fn wait_till_started(&mut self) {
        let mut attempts = CONNECTION_ATTEMPTS;
        loop {
            match TcpStream::connect(&self.address).await {
                Ok(_) => return,
                Err(err) => {
                    if let Some(status) = self.has_exited() {
                        panic!("Orchestrator node exited early with {}", status);
                    }
                    if attempts == 0 {
                        panic!("Failed to connect to {}: {}", self.address, err);
                    }
                }
            };

            attempts -= 1;
            tokio::time::sleep(Duration::from_millis(CONNECTION_ATTEMPT_DELAY_MS)).await;
        }
    }
}
