use std::clone::Clone;
use std::path::Path;

use cairo_vm::types::layout_name::LayoutName;
use reqwest::multipart::Form;
use reqwest::{multipart, Body, ClientBuilder};
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use url::Url;
use utils::env_utils::get_env_var_or_panic;

use crate::config::SettlementLayer;
use crate::error::AtlanticError;
use crate::types::{AtlanticAddJobResponse, AtlanticGetStatusResponse};

#[derive(Debug, strum_macros::EnumString)]
enum ProverType {
    #[strum(serialize = "starkware")]
    Starkware,
    #[strum(serialize = "herodotus")]
    HeroDotus,
}

/// SHARP API async wrapper
#[derive(Debug)]
pub struct AtlanticClient {
    base_url: Url,
    client: reqwest::Client,
    settlement_layer: SettlementLayer,
    api_key: String,
    mock_fact_hash: bool,
    prover_type: ProverType,
}

impl AtlanticClient {
    /// We need to set up the client with the API_KEY.
    pub fn new_with_settings(mut url: Url, settlement_layer: SettlementLayer) -> Self {
        let api_key = get_env_var_or_panic("ATLANTIC_API_KEY");
        let mock_fact_hash =
            get_env_var_or_panic("MOCK_FACT_HASH").parse::<bool>().expect("MOCK_FACT_HASH must be a boolean");
        let prover_type =
            get_env_var_or_panic("PROVER_TYPE").parse::<ProverType>().expect("PROVER_TYPE must be a valid prover type");

        {
            let mut pairs = url.query_pairs_mut();

            let query_params = vec![("apiKey", api_key.as_str())];
            for (key, value) in query_params {
                pairs.append_pair(key, value);
            }
        }

        Self {
            base_url: url.clone(),
            client: ClientBuilder::new().build().unwrap(),
            settlement_layer,
            api_key,
            mock_fact_hash,
            prover_type,
        }
    }

    pub async fn add_job(
        &self,
        pie_file: &Path,
        proof_layout: LayoutName,
    ) -> Result<AtlanticAddJobResponse, AtlanticError> {
        match self.settlement_layer {
            SettlementLayer::Ethereum => submit_l2_proving_job(self, pie_file, proof_layout).await,
            SettlementLayer::Starknet => submit_l3_proving_job(self, pie_file, proof_layout).await,
        }
    }

    pub async fn get_job_status(&self, job_key: &str) -> Result<AtlanticGetStatusResponse, AtlanticError> {
        let mut base_url = self.base_url.clone();

        base_url
            .path_segments_mut()
            .map_err(|_| AtlanticError::PathSegmentMutFailOnUrl)?
            .push("sharp-query")
            .push(job_key);
        let res = self.client.get(base_url).send().await.map_err(AtlanticError::GetJobStatusFailure)?;
        log::trace!("Task status from atlantic {:?}", res);

        if res.status().is_success() {
            res.json().await.map_err(AtlanticError::GetJobStatusFailure)
        } else {
            Err(AtlanticError::SharpService(res.status()))
        }
    }
}

async fn submit_l2_proving_job(
    atlantic_client: &AtlanticClient,
    pie_file: &Path,
    proof_layout: LayoutName,
) -> Result<AtlanticAddJobResponse, AtlanticError> {
    let mut base_url = atlantic_client.base_url.clone();
    base_url
        .path_segments_mut()
        .map_err(|_| AtlanticError::PathSegmentMutFailOnUrl)?
        .push("l1")
        .push("submit-sharp-query")
        .push("proof_generation_verification");

    let api_key = get_env_var_or_panic("ATLANTIC_API_KEY");
    let mock_fact_hash = get_env_var_or_panic("MOCK_FACT_HASH");

    let query_params = vec![("apiKey", api_key.as_str())];

    let pie_file = File::open(pie_file).await.map_err(AtlanticError::FileReadError)?;
    let stream = FramedRead::new(pie_file, BytesCodec::new());
    let file_body = Body::wrap_stream(stream);

    // make form part of file
    let pie_file_part = multipart::Part::stream(file_body).file_name("pie.zip");

    let form = Form::new()
        .part("pieFile", pie_file_part)
        .text("layout", proof_layout.to_str())
        .text("mockFactHash", mock_fact_hash);

    // Adding params to the URL
    add_params_to_url(&mut base_url, query_params);
    let res =
        atlantic_client.client.post(base_url).multipart(form).send().await.map_err(AtlanticError::AddJobFailure)?;
    if res.status().is_success() {
        let result: AtlanticAddJobResponse = res.json().await.map_err(AtlanticError::AddJobFailure)?;
        Ok(result)
    } else {
        log::error!("Failed to add job to Atlantic: {:?}", res);
        Err(AtlanticError::SharpService(res.status()))
    }
}

#[allow(unused)]
async fn submit_l3_proving_job(
    atlantic_client: &AtlanticClient,
    pie_file: &Path,
    proof_layout: LayoutName,
) -> Result<AtlanticAddJobResponse, AtlanticError> {
    let mut base_url = atlantic_client.base_url.clone();

    base_url
        .path_segments_mut()
        .map_err(|_| AtlanticError::PathSegmentMutFailOnUrl)?
        .push("l2")
        .push("submit-sharp-query")
        .push("from-proof-generation-to-proof-verification");

    let api_key = get_env_var_or_panic("ATLANTIC_API_KEY");
    let proof_layout = get_env_var_or_panic("SNOS_PROOF_LAYOUT");
    let mock_fact_hash = get_env_var_or_panic("MOCK_FACT_HASH");
    let prover = get_env_var_or_panic("PROVER_TYPE");

    let query_params = vec![("apiKey", api_key.as_str())];

    let pie_file = File::open(pie_file).await.map_err(AtlanticError::FileReadError)?;
    let stream = FramedRead::new(pie_file, BytesCodec::new());
    let file_body = Body::wrap_stream(stream);

    // make form part of file
    let pie_file_part = multipart::Part::stream(file_body).file_name("pie.zip");

    let form = Form::new()
        .part("pieFile", pie_file_part)
        .text("layout", proof_layout)
        .text("prover", prover)
        .text("mockFactHash", mock_fact_hash);

    log::trace!("form {:?}", form);
    // Adding params to the URL
    add_params_to_url(&mut base_url, query_params);

    let multipart_request = atlantic_client.client.post(base_url).multipart(form);
    log::debug!("The multipart request is: {:?}", multipart_request);

    let res = multipart_request.send().await.map_err(AtlanticError::AddJobFailure)?;
    if res.status().is_success() {
        log::debug!("Successfully submitted task to atlantic: {:?}", res);
        let result: AtlanticAddJobResponse = res.json().await.map_err(AtlanticError::AddJobFailure)?;
        Ok(result)
    } else {
        log::error!("Failed to add job to Atlantic: {:?}", res);
        Err(AtlanticError::SharpService(res.status()))
    }
}

fn add_params_to_url(url: &mut Url, params: Vec<(&str, &str)>) {
    let mut pairs = url.query_pairs_mut();
    for (key, value) in params {
        pairs.append_pair(key, value);
    }
}
