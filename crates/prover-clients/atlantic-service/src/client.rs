use std::path::Path;

use cairo_vm::types::layout_name::LayoutName;
use reqwest::Method;
use url::Url;
use utils::http_client::{HttpClient, RequestBuilder};

use crate::error::AtlanticError;
use crate::types::{AtlanticAddJobResponse, AtlanticGetStatusResponse};
use crate::AtlanticValidatedArgs;

#[derive(Debug, strum_macros::EnumString)]
enum ProverType {
    #[strum(serialize = "starkware")]
    Starkware,
    #[strum(serialize = "herodotus")]
    HeroDotus,
}

trait ProvingLayer: Send + Sync {
    fn customize_request<'a>(&self, request: RequestBuilder<'a>) -> RequestBuilder<'a>;
}

struct EthereumLayer;
impl ProvingLayer for EthereumLayer {
    fn customize_request<'a>(&self, request: RequestBuilder<'a>) -> RequestBuilder<'a> {
        request.path("v1").path("l1/atlantic-query/proof-generation-verification")
    }
}

struct StarknetLayer;
impl ProvingLayer for StarknetLayer {
    fn customize_request<'a>(&self, request: RequestBuilder<'a>) -> RequestBuilder<'a> {
        request.path("v1").path("proof-generation")
    }
}

/// SHARP API async wrapper
pub struct AtlanticClient {
    client: HttpClient,
    proving_layer: Box<dyn ProvingLayer>,
}

impl AtlanticClient {
    /// We need to set up the client with the API_KEY.
    pub fn new_with_args(url: Url, atlantic_params: &AtlanticValidatedArgs) -> Self {
        let mock_fact_hash = atlantic_params.atlantic_mock_fact_hash.clone();
        let prover_type = atlantic_params.atlantic_prover_type.clone();

        let client = HttpClient::builder(url.as_str())
            .default_form_data("mockFactHash", &mock_fact_hash)
            .build()
            .expect("Failed to build HTTP client");

        let proving_layer: Box<dyn ProvingLayer> = match atlantic_params.atlantic_settlement_layer.as_str() {
            "ethereum" => Box::new(EthereumLayer),
            "starknet" => Box::new(StarknetLayer),
            _ => panic!("proving layer not correct"),
        };

        Self { client, proving_layer }
    }

    pub async fn add_job(
        &self,
        pie_file: &Path,
        proof_layout: LayoutName,
        atlantic_api_key: String,
    ) -> Result<AtlanticAddJobResponse, AtlanticError> {
        let proof_layout = match proof_layout {
            LayoutName::dynamic => "dynamic",
            _ => proof_layout.to_str(),
        };

        let response = self
            .proving_layer
            .customize_request(
                self.client.request().method(Method::POST).query_param("apiKey", atlantic_api_key.as_ref()),
            )
            .form_file("pieFile", pie_file, "pie.zip")
            .form_text("layout", proof_layout)
            .send()
            .await
            .map_err(AtlanticError::AddJobFailure)?;

        tracing::info!(">>>>>>> response: {:?}", response);
        if response.status().is_success() {
            response.json().await.map_err(AtlanticError::AddJobFailure)
        } else {
            Err(AtlanticError::SharpService(response.status()))
        }
    }

    pub async fn submit_l2_query(&self, task_id: &str, proof: &str, atlantic_api_key: &str) -> Result<AtlanticAddJobResponse, AtlanticError> {
        tracing::info!(">>>>>>> task_id: {:?}", task_id);
        let response = self
            .client
            .request()
            .method(Method::POST)
            .path("v1")
            .path("l2/atlantic-query")
            .query_param("apiKey", atlantic_api_key.as_ref())
            .form_text("programHash", "0x193641eb151b0f41674641089952e60bc3aded26e3cf42793655c562b8c3aa0")
            .form_text("prover", "starkware_sharp")
            .form_text("cairoVersion", "0")
            .form_text("layout", "recursive_with_poseidon")
            .form_file_bytes("inputFile", proof.as_bytes().to_vec(), "proof.json")
            .send()
            .await
            .map_err(AtlanticError::SubmitL2QueryFailure)?;

        tracing::info!(">>>>>>> response: {:?}", response);
        if response.status().is_success() {
            response.json().await.map_err(AtlanticError::AddJobFailure)
        } else {
            Err(AtlanticError::SharpService(response.status()))
        }
    }

    pub async fn get_job_status(&self, job_key: &str) -> Result<AtlanticGetStatusResponse, AtlanticError> {
        let response = self
            .client
            .request()
            .method(Method::GET)
            .path("v1")
            .path("atlantic-query")
            .path(job_key)
            .send()
            .await
            .map_err(AtlanticError::GetJobStatusFailure)?;

        tracing::info!(">>>>>>> response: {:?}", response);

        if response.status().is_success() {
            response.json().await.map_err(AtlanticError::GetJobStatusFailure)
        } else {
            Err(AtlanticError::SharpService(response.status()))
        }
    }
}
