use serde::{Deserialize, Serialize};
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlanticAddJobResponse {
    pub sharp_query_id: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct AtlanticGetProofResponse {
    pub code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtlanticGetStatusResponse {
    pub sharp_query: SharpQuery,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharpQuery {
    pub id: String,
    pub submitted_by_client: String,
    pub status: SharpQueryStatus,
    pub step: SharpQueryStep,
    pub program_hash: Option<String>,
    pub layout: Option<String>,
    pub program_fact_hash: Option<String>,
    pub is_fact_mocked: bool,
    pub prover: String,
    pub chain: String,
    pub price: i64,
    pub steps: Vec<SharpQueryStep>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SharpQueryStatus {
    InProgress,
    Done,
    Failed,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SharpQueryStep {
    ProofGeneration,
    FactHashGeneration,
    FactHashRegistration,
}