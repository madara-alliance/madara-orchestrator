use serde::{Deserialize, Serialize};
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlanticAddJobResponse {
    pub atlantic_query_id: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AtlanticGetProofResponse {
    pub code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlanticQuery {
    pub id: String,
    pub external_id: Option<String>,
    pub transaction_id: String,
    pub status: AtlanticQueryStatus,
    pub step: AtlanticQueryStep,
    pub program_hash: String,
    pub integrity_fact_hash: String,
    pub sharp_fact_hash: String,
    pub layout: String,
    pub is_fact_mocked: Option<bool>,
    pub chain: String,
    pub job_size: String,
    pub declared_job_size: String,
    pub cairo_vm: String,
    pub cairo_version: String,
    pub steps: Vec<AtlanticQueryStep>,
    pub result: String,
    pub network: String,
    pub error_reason: Option<String>,
    pub submitted_by_client: String,
    pub project_id: String,
    pub created_at: String,
    pub completed_at: String,
    pub client: AtlanticClient,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlanticClient {
    pub client_id: String,
    pub name: String,
    pub email: String,
    pub is_email_verified: bool,
    pub image: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlanticGetStatusResponse {
    pub atlantic_query: AtlanticQuery,
    pub metadata_urls: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AtlanticQueryStatus {
    Received,
    InProgress,
    Done,
    Failed,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AtlanticQueryStep {
    TraceGeneration,
    ProofGeneration,
    ProofVerification,
    ProofVerificationOnL1,
    ProofVerificationOnL2,
    ProofGenerationAndVerification,
}
