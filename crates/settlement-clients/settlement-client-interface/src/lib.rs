use async_trait::async_trait;
use color_eyre::eyre::Result;
use mockall::automock;
use mockall::predicate::*;

pub const SETTLEMENT_SETTINGS_NAME: &str = "settlement_settings";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementVerificationStatus {
    Pending,
    Verified,
    Rejected(String),
}

/// Trait for every new Settlement Layer to implement
#[automock]
#[async_trait]
pub trait SettlementClient: Send + Sync {
    /// Should register the proof on the base layer and return an external id
    /// which can be used to track the status.
    async fn register_proof(&self, proof: Vec<u8>) -> Result<String>;

    /// Should be used to update state on core contract when DA is done in calldata
    async fn update_state_calldata(
        &self,
        program_output: Vec<Vec<u8>>,
        onchain_data_hash: Vec<u8>,
        onchain_data_size: usize,
    ) -> Result<String>;

    /// Should be used to update state on core contract when DA is in blobs/alt DA
    async fn update_state_blobs(&self, program_output: Vec<Vec<u8>>, kzg_proof: Vec<u8>) -> Result<String>;

    /// Should verify the inclusion of the state diff in the Settlement layer and return the status
    async fn verify_inclusion(&self, external_id: &str) -> Result<SettlementVerificationStatus>;

    async fn get_last_settled_block(&self) -> Result<u64>;
}

/// Trait for every new SettlementConfig to implement
pub trait SettlementConfig {
    /// Should create a new instance of the SettlementConfig from the environment variables
    fn new_from_env() -> Self;
}
