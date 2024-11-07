use std::path::Path;

use cairo_vm::types::layout_name::LayoutName;
use reqwest::Method;
use url::Url;
use utils::env_utils::get_env_var_or_panic;
use utils::http_client::{HttpClient, RequestBuilder};

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

trait ProvingLayer: Send + Sync {
    fn customize_request<'a>(&self, request: RequestBuilder<'a>) -> RequestBuilder<'a>;
}

struct EthereumLayer;
impl ProvingLayer for EthereumLayer {
    fn customize_request<'a>(&self, request: RequestBuilder<'a>) -> RequestBuilder<'a> {
        request.path("/l1/atlantic-query/proof_generation_verification")
    }
}

struct StarknetLayer;
impl ProvingLayer for StarknetLayer {
    fn customize_request<'a>(&self, request: RequestBuilder<'a>) -> RequestBuilder<'a> {
        request.path("/l2/submit-sharp-query/from-proof-generation-to-proof-verification")
    }
}

/// SHARP API async wrapper
pub struct AtlanticClient {
    client: HttpClient,
    proving_layer: Box<dyn ProvingLayer>,
}

impl AtlanticClient {
    /// We need to set up the client with the API_KEY.
    pub fn new_with_settings(url: Url, settlement_layer: SettlementLayer) -> Self {
        let mock_fact_hash = get_env_var_or_panic("MOCK_FACT_HASH");
        let prover_type = get_env_var_or_panic("PROVER_TYPE");

        let client = HttpClient::builder(url.as_str())
            .default_form_data("mockFactHash", &mock_fact_hash)
            .default_form_data("proverType", &prover_type)
            .build()
            .expect("Failed to build HTTP client");

        let proving_layer: Box<dyn ProvingLayer> = match settlement_layer {
            SettlementLayer::Ethereum => Box::new(EthereumLayer),
            SettlementLayer::Starknet => Box::new(StarknetLayer),
        };

        Self { client, proving_layer }
    }

    pub async fn add_job(
        &self,
        pie_file: &Path,
        _proof_layout: LayoutName,
    ) -> Result<AtlanticAddJobResponse, AtlanticError> {
        let api_key = get_env_var_or_panic("ATLANTIC_API_KEY");

        let response = self
            .proving_layer
            .customize_request(
                self.client
                    .request()
                    .method(Method::POST)
                    .query_param("apiKey", &api_key)
                    .form_file("pieFile", pie_file, "pie.zip")
                    .form_text("layout", "dynamic"),
            )
            .send()
            .await
            .map_err(AtlanticError::AddJobFailure)
            .expect("Failed to add job");

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
            .path("atlantic-query")
            .path(job_key)
            .send()
            .await
            .map_err(AtlanticError::GetJobStatusFailure)?;

        if response.status().is_success() {
            response.json().await.map_err(AtlanticError::GetJobStatusFailure)
        } else {
            Err(AtlanticError::SharpService(response.status()))
        }
    }
}
