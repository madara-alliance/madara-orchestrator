use std::clone::Clone;

use base64::engine::general_purpose;
use base64::Engine;
use reqwest::{Certificate, ClientBuilder, Identity};
use url::Url;
use utils::env_utils::get_env_var_or_panic;
use utils::settings::Settings;
use uuid::Uuid;

use crate::error::SharpError;
use crate::types::{SharpAddJobResponse, SharpGetStatusResponse};

/// SHARP API async wrapper
pub struct SharpClient {
    base_url: Url,
    client: reqwest::Client,
}

impl SharpClient {
    /// We need to set up the client with the provided certificates.
    /// We need to have three secrets :
    /// - base64(SHARP_USER_CRT)
    /// - base64(SHARP_USER_KEY)
    /// - base64(SHARP_SERVER_CRT)
    ///
    /// You can run this command in terminal to convert a file output into base64
    /// and then copy it and paste it into .env file :
    ///
    /// `cat <file_name> | base64`
    pub fn new_with_settings(url: Url, settings: &impl Settings) -> Self {
        // Getting the cert files from the .env and then decoding it from base64

        let cert = general_purpose::STANDARD
            .decode(settings.get_settings_or_panic("SHARP_USER_CRT"))
            .expect("Failed to decode certificate");
        let key = general_purpose::STANDARD
            .decode(settings.get_settings_or_panic("SHARP_USER_KEY"))
            .expect("Failed to decode sharp user key");
        let server_cert = general_purpose::STANDARD
            .decode(settings.get_settings_or_panic("SHARP_SERVER_CRT"))
            .expect("Failed to decode sharp server certificate");

        // Adding Customer ID to the url

        let mut url_mut = url.clone();
        let customer_id = settings.get_settings_or_panic("SHARP_CUSTOMER_ID");
        url_mut.query_pairs_mut().append_pair("customer_id", customer_id.as_str());

        Self {
            base_url: url_mut,
            client: ClientBuilder::new()
                .identity(
                    Identity::from_pkcs8_pem(&cert, &key)
                        .expect("Failed to build the identity from certificate and key"),
                )
                .add_root_certificate(
                    Certificate::from_pem(server_cert.as_slice()).expect("Failed to add root certificate"),
                )
                .build()
                .expect("Failed to build the reqwest client with provided configs"),
        }
    }

    pub async fn add_job(&self, encoded_pie: &str) -> Result<(SharpAddJobResponse, Uuid), SharpError> {
        let mut base_url = self.base_url.clone();

        base_url.path_segments_mut().map_err(|_| SharpError::PathSegmentMutFailOnUrl)?.push("add_job");

        let cairo_key = Uuid::new_v4();
        let cairo_key_string = cairo_key.to_string();
        let proof_layout = get_env_var_or_panic("SHARP_PROOF_LAYOUT");

        // Params for sending the PIE file to the prover
        // for temporary reference you can check this doc :
        // https://docs.google.com/document/d/1-9ggQoYmjqAtLBGNNR2Z5eLreBmlckGYjbVl0khtpU0
        let params = vec![
            ("cairo_job_key", cairo_key_string.as_str()),
            ("offchain_proof", "true"),
            ("proof_layout", proof_layout.as_str()),
        ];

        // Adding params to the URL
        add_params_to_url(&mut base_url, params);

        let res =
            self.client.post(base_url).body(encoded_pie.to_string()).send().await.map_err(SharpError::AddJobFailure)?;

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

        base_url.path_segments_mut().map_err(|_| SharpError::PathSegmentMutFailOnUrl)?.push("get_status");
        let cairo_key_string = job_key.to_string();

        // Params for getting the prover job status
        // for temporary reference you can check this doc :
        // https://docs.google.com/document/d/1-9ggQoYmjqAtLBGNNR2Z5eLreBmlckGYjbVl0khtpU0
        let params = vec![("cairo_job_key", cairo_key_string.as_str())];

        // Adding params to the url
        add_params_to_url(&mut base_url, params);

        let res = self.client.post(base_url).send().await.map_err(SharpError::GetJobStatusFailure)?;

        match res.status() {
            reqwest::StatusCode::OK => res.json().await.map_err(SharpError::GetJobStatusFailure),
            code => Err(SharpError::SharpService(code)),
        }
    }
}

fn add_params_to_url(url: &mut Url, params: Vec<(&str, &str)>) {
    let mut pairs = url.query_pairs_mut();
    for (key, value) in params {
        pairs.append_pair(key, value);
    }
}
