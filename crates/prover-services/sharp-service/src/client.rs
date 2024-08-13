use std::clone::Clone;
use reqwest::{Certificate, ClientBuilder, Identity};
use std::fs;
use std::path::{PathBuf};
use lazy_static::lazy_static;
use url::Url;
use utils::env_utils::get_env_var_or_panic;
use uuid::Uuid;

use crate::error::SharpError;
use crate::types::{SharpAddJobResponse, SharpGetStatusResponse};

/// SHARP endpoint for Sepolia testnet
pub const DEFAULT_SHARP_URL: &str = "https://sepolia-recursive.public-testnet.provingservice.io/v1/gateway";

lazy_static! {
    // Define a static variable to hold the current directory
    static ref CURRENT_PATH: PathBuf = std::env::current_dir().unwrap();

    // Define static variables for the certificate paths
    static ref USER_CRT_PATH: PathBuf = CURRENT_PATH.join("user.crt");
    static ref USER_KEY_PATH: PathBuf = CURRENT_PATH.join("user.key");
    static ref SERVER_CRT_PATH: PathBuf = CURRENT_PATH.join("server.crt");
}

/// SHARP API async wrapper
pub struct SharpClient {
    base_url: Url,
    client: reqwest::Client,
}

impl SharpClient {
    /// We need to set up the client with the provided certificates.
    /// We need to have three files : 
    /// - user.crt
    /// - user.key
    /// - server.crt
    pub fn new(url: Url) -> Self {
        let cert = fs::read(USER_CRT_PATH.as_path()).expect("Unable to read user.crt.");
        let key = fs::read(USER_KEY_PATH.as_path()).expect("Unable to read user.key.");
        let server_cert = fs::read(SERVER_CRT_PATH.as_path()).expect("Unable to read server.crt");

        // Constructing identity from crt and key for user
        let mut identity = cert.clone();
        identity.extend_from_slice(&key);
        
        Self {
            base_url: url,
            client: ClientBuilder::new()
                .identity(Identity::from_pem(identity.as_slice()).unwrap())
                .add_root_certificate(Certificate::from_pem(server_cert.as_slice()).unwrap())
                .build()
                .unwrap(),
        }
    }

    pub async fn add_job(&self, encoded_pie: &str) -> Result<(SharpAddJobResponse, Uuid), SharpError> {
        let mut base_url = self.base_url.clone();

        // Making the URL from base url and returning the cairo key constructed
        let cairo_key = get_full_url_with_body_for_add_job(&mut base_url, encoded_pie);

        let res = self
            .client
            .post(base_url)
            .body(encoded_pie.to_string())
            .send()
            .await
            .map_err(|e| SharpError::AddJobFailure(e))?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let result: SharpAddJobResponse = res.json().await.map_err(SharpError::AddJobFailure)?;
                Ok((result, cairo_key))
            }
            code => Err(SharpError::SharpService(code)),
        }
    }

    pub async fn get_job_status(&self, job_key: &Uuid) -> Result<SharpGetStatusResponse, SharpError> {
        let mut base_url = self.base_url.clone();
        get_full_url_with_body_for_get_job_status(&mut base_url, job_key);
        let res = self.client.post(base_url).send().await.map_err(SharpError::GetJobStatusFailure)?;

        match res.status() {
            reqwest::StatusCode::OK => res.json().await.map_err(SharpError::GetJobStatusFailure),
            code => Err(SharpError::SharpService(code)),
        }
    }
}

/// To construct the url for adding the job to the sharp service.
fn get_full_url_with_body_for_add_job(url: &mut Url, encoded_pie: &str) -> Uuid {
    let mut url = url.join("add_job").unwrap();
    let customer_id = get_env_var_or_panic("SHARP_CUSTOMER_ID");
    let cairo_key = Uuid::new_v4();
    let cairo_key_string = cairo_key.to_string();

    // Params for sending the PIE file to the prover
    let params = vec![
        ("customer_id", customer_id.as_str()),
        ("cairo_job_key", &cairo_key_string),
        ("offchain_proof", "true"),
        ("proof_layout", "small"),
        ("encoded_pie", &encoded_pie),
    ];

    let mut pairs = url.query_pairs_mut();
    for (key, value) in params {
        pairs.append_pair(key, value);
    }

    cairo_key
}

/// To construct the url for getting the job status from the sharp service.
fn get_full_url_with_body_for_get_job_status(url: &mut Url, job_key: &Uuid) {
    let mut url = url.join("get_status").unwrap();
    let customer_id = get_env_var_or_panic("SHARP_CUSTOMER_ID");
    let cairo_key_string = job_key.to_string();

    // Params for sending the PIE file to the prover
    let params = vec![
        ("customer_id", customer_id.as_str()),
        ("cairo_job_key", &cairo_key_string),
    ];

    let mut pairs = url.query_pairs_mut();
    for (key, value) in params {
        pairs.append_pair(key, value);
    }
}

impl Default for SharpClient {
    fn default() -> Self {
        Self::new(DEFAULT_SHARP_URL.parse().unwrap())
    }
}
