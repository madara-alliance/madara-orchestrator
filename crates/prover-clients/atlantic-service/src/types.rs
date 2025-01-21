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
pub struct AtlanticGetStatusResponse {
    pub atlantic_query: AtlanticQuery,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlanticQuery {
    pub id: String,
    pub external_id: String,
    pub submitted_by_client: String,
    pub status: AtlanticQueryStatus,
    pub step: Option<AtlanticQueryStep>,
    pub program_hash: Option<String>,
    pub layout: Option<String>,
    pub program_fact_hash: Option<String>,
    pub price: String,
    pub trace_credits_used: u64,
    pub is_fact_mocked: Option<bool>,
    pub chain: Option<String>,
    pub prover: String,
    pub steps: Vec<AtlanticQueryStep>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AtlanticQueryStatus {
    InProgress,
    Done,
    Failed,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AtlanticQueryStep {
    TraceGeneration,
    ProofGeneration,
    FactHashGeneration,
    FactHashRegistration,
    ProofVerification,
}
