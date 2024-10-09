use std::clone::Clone;
use std::path::Path;

use reqwest::ClientBuilder;
use reqwest::multipart::{Form, Part};
use url::Url;
use utils::env_utils::get_env_var_or_panic;
use uuid::Uuid;

use crate::error::AtlanticError;
use crate::types::{AtlanticAddJobResponse, AtlanticGetStatusResponse};

/// SHARP API async wrapper
#[derive(Debug)]
pub struct AtlanticClient {
    base_url: Url,
    client: reqwest::Client,
}

impl AtlanticClient {
    /// We need to set up the client with the API_KEY.
    pub fn new_with_settings(url: Url) -> Self {
        Self { base_url: url, client: ClientBuilder::new().build().unwrap() }
    }

    pub async fn add_job(&self, pie_file: &Path) -> Result<AtlanticAddJobResponse, AtlanticError> {
        let mut base_url = self.base_url.clone();
        log::trace!("Atlantic base_url {:?}", base_url);

        base_url
            .path_segments_mut()
            .map_err(|_| AtlanticError::PathSegmentMutFailOnUrl)?
            .push("submit-sharp-query")
            .push("from-proof_generation-to-proof_verification");
        println!("base_url {:?}", base_url);

        let api_key = get_env_var_or_panic("ATLANTIC_API_KEY");
        let proof_layout = get_env_var_or_panic("SHARP_PROOF_LAYOUT");

        let query_params = vec![("apiKey", api_key.as_str())];

        // Open the file
        let file_contents = tokio::fs::read(pie_file).await.map_err(AtlanticError::FileReadError)?;
        let file_part = Part::bytes(file_contents);

        let form = Form::new().part("pieFile", file_part).text("layout", proof_layout).text("isOffchainProof", "true");

        // Adding params to the URL
        add_params_to_url(&mut base_url, query_params);

        let res = self.client.post(base_url).multipart(form).send().await.map_err(AtlanticError::AddJobFailure)?;

        match res.status() {
            reqwest::StatusCode::OK => {
                let result: AtlanticAddJobResponse = res.json().await.map_err(AtlanticError::AddJobFailure)?;
                Ok(result)
            }
            code => Err(AtlanticError::SharpService(code)),
        }
    }

    pub async fn get_job_status(&self, job_key: &Uuid) -> Result<AtlanticGetStatusResponse, AtlanticError> {
        let mut base_url = self.base_url.clone();

        base_url.path_segments_mut().map_err(|_| AtlanticError::PathSegmentMutFailOnUrl)?.push("get_status");
        let cairo_key_string = job_key.to_string();

        // Params for getting the prover job status
        // for temporary reference you can check this doc :
        // https://docs.google.com/document/d/1-9ggQoYmjqAtLBGNNR2Z5eLreBmlckGYjbVl0khtpU0
        let params = vec![("cairo_job_key", cairo_key_string.as_str())];

        // Adding params to the url
        add_params_to_url(&mut base_url, params);

        let res = self.client.post(base_url).send().await.map_err(AtlanticError::GetJobStatusFailure)?;

        match res.status() {
            reqwest::StatusCode::OK => res.json().await.map_err(AtlanticError::GetJobStatusFailure),
            code => Err(AtlanticError::SharpService(code)),
        }
    }
}

fn add_params_to_url(url: &mut Url, params: Vec<(&str, &str)>) {
    let mut pairs = url.query_pairs_mut();
    for (key, value) in params {
        pairs.append_pair(key, value);
    }
}
