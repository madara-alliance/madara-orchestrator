use std::path::Path;
use std::time::Duration;

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
        request.path("v1").path("l2/submit-sharp-query/from-proof-generation-to-proof-verification")
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
            .expect("Failed to create HTTP client builder")
            .timeout(Duration::from_secs(1200))  // 20 minute timeout
            .default_form_data("mockFactHash", &mock_fact_hash)
            .default_form_data("proverType", &prover_type)
            .build()
            .expect("Failed to build HTTP client");

        let proving_layer: Box<dyn ProvingLayer> = match atlantic_params.atlantic_settlement_layer.as_str() {
            "ethereum" => Box::new(EthereumLayer),
            "starknet" => Box::new(StarknetLayer),
            _ => panic!("Invalid settlement layer: {}", atlantic_params.atlantic_settlement_layer),
        };

        Self { client, proving_layer }
    }

    pub async fn add_job(
        &self,
        pie_file: &Path,
        proof_layout: LayoutName,
        atlantic_api_key: impl AsRef<str>,
    ) -> Result<AtlanticAddJobResponse, AtlanticError> {
        tracing::info!(">>>>>>>>>>>> Adding job to Atlantic");
        let proof_layout = match proof_layout {
            LayoutName::dynamic => "dynamic",
            _ => proof_layout.to_str(),
        };

        // Log file details
        let file_size = std::fs::metadata(pie_file)?.len();
        tracing::info!("Uploading file of size: {} bytes", file_size);
        tracing::info!("File path: {:?}", pie_file);

        // Log request construction
        tracing::info!("Constructing request with layout: {}", proof_layout);
        
        let request = self
            .proving_layer
            .customize_request(
                self.client.request()
                    .method(Method::POST)
                    .query_param("apiKey", atlantic_api_key.as_ref())
            )
            .form_file("pieFile", pie_file, "pie.zip")?
            .form_text("layout", proof_layout);

        // Log before sending
        tracing::info!("Starting file upload...");
        
        let response = match request.send().await {
            Ok(resp) => {
                tracing::info!(
                    "Received response with status: {} after upload",
                    resp.status()
                );
                resp
            }
            Err(e) => {
                tracing::error!(
                    "Request failed during upload: {:?}. Error type: {}",
                    e,
                    std::any::type_name_of_val(&e)
                );
                return Err(AtlanticError::AddJobFailure(e));
            }
        };

        match response.status().is_success() {
            true => {
                tracing::info!("Successfully uploaded file, parsing response");
                response.json().await.map_err(|e| {
                    tracing::error!("Failed to parse successful response: {:?}", e);
                    AtlanticError::AddJobFailure(e)
                })
            }
            false => {
                let status = response.status();
                // Try to get error body
                let error_body = response.text().await.unwrap_or_default();
                tracing::error!(
                    "Request failed with status: {}, error body: {}",
                    status,
                    error_body
                );
                Err(AtlanticError::SharpService(status))
            }
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

        if response.status().is_success() {
            response.json().await.map_err(AtlanticError::GetJobStatusFailure)
        } else {
            Err(AtlanticError::SharpService(response.status()))
        }
    }
}
