use async_trait::async_trait;
use color_eyre::Result;

use crate::jobs::types::JobVerificationStatus;

mod model;

pub use self::model::*;

/// Describes the functionalities required by a proof client.
#[async_trait]
pub trait ProofClient {
    /// Requests creation of a proof using the execution information stored in the
    /// provided [`ProofRequest`].
    ///
    /// # Returns
    ///
    /// This function returns an ID that can be used to track the status of the job
    /// and retrieve the proof once it is ready.
    fn create_proof(&self, req: &ProofRequest) -> Result<String>;

    /// Fetches the proof for the job with the given `external_id`.
    ///
    /// # Returns
    ///
    /// This function fetches the state of the proof job and returns it as a
    /// [`JobVerificationStatus`].
    fn fetch_state(&self, external_id: &str) -> Result<JobVerificationStatus>;
}
