mod dummy_state;

use std::collections::HashMap;
use std::num::NonZeroU128;

use async_trait::async_trait;
use blockifier::block::{pre_process_block, BlockInfo, BlockNumberHashPair, GasPrices};
use blockifier::context::{ChainInfo, FeeTokenAddresses};
use blockifier::versioned_constants::VersionedConstants;
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::Felt252;
use color_eyre::Result;
use num::FromPrimitive;
use snos::execution::helper::ExecutionHelperWrapper;
use snos::io::input::StarknetOsInput;
use snos::run_os;
use starknet_api::block::{BlockHash, BlockNumber, BlockTimestamp};
use starknet_api::core::{ChainId, ContractAddress, PatriciaKey};
use starknet_api::hash::StarkFelt;
use starknet_core::types::FieldElement;
use uuid::Uuid;

use crate::config::Config;
use crate::jobs::snos_job::dummy_state::DummyState;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;

pub struct SnosJob;

#[async_trait]
impl Job for SnosJob {
    async fn create_job(
        &self,
        _config: &Config,
        internal_id: String,
        metadata: HashMap<String, String>,
    ) -> Result<JobItem> {
        Ok(JobItem {
            id: Uuid::new_v4(),
            internal_id,
            job_type: JobType::SnosRun,
            status: JobStatus::Created,
            external_id: String::new().into(),
            metadata,
            version: 0,
        })
    }

    async fn process_job(&self, _config: &Config, _job: &mut JobItem) -> Result<String> {
        // 1. Fetch SNOS input data from Madara
        // TODO: JSON RPC call to `getSnosInput` for a specific block
        let snos_input = StarknetOsInput::load(std::path::Path::new("snos_input.txt")).unwrap();

        // 2. Build the required inputs for snos::run_os
        // TODO import BlockifierStateAdapter from Deoxys RPC and use it here
        let mut state = DummyState {};

        // TODO: build the BlockNumberHashPair, should be in the metadata?
        let old_block_number_and_hash =
            BlockNumberHashPair { number: BlockNumber(42), hash: BlockHash(StarkFelt::from_u128(4242)) };

        // TODO: retrieve the actual gas prices - from deoxys..?
        let block_info = BlockInfo {
            // TODO: get the block number from SnosInput block_hash
            block_number: BlockNumber(69420),
            // TODO: get the block timestamp from SnosInput block_hash
            block_timestamp: BlockTimestamp(69420420),
            // TODO: should be a constant?
            sequencer_address: ContractAddress(PatriciaKey::from(42_u32)),
            // TODO: retrieve them from Deoxys?
            gas_prices: GasPrices {
                eth_l1_gas_price: NonZeroU128::new(42).unwrap(),
                strk_l1_gas_price: NonZeroU128::new(69).unwrap(),
                eth_l1_data_gas_price: NonZeroU128::new(420).unwrap(),
                strk_l1_data_gas_price: NonZeroU128::new(4269420).unwrap(),
            },
            // TODO: where do we know that? In our configuration?
            use_kzg_da: false,
        };

        let chain_info = ChainInfo {
            // TODO: retrieve the chain_id from our configuration?
            chain_id: ChainId(String::from("0x69420")),
            // TODO: retrieve the fee token addresses from our configuration or deoxys?
            fee_token_addresses: FeeTokenAddresses {
                strk_fee_token_address: ContractAddress(PatriciaKey::from(42_u32)),
                eth_fee_token_address: ContractAddress(PatriciaKey::from(42_u32)),
            },
        };

        // TODO: check this, lot of fields
        let versioned_constants = VersionedConstants::default();

        let old_block_number = old_block_number_and_hash.number.0;
        let old_block_hash = old_block_number_and_hash.hash.0;
        let block_context = pre_process_block(
            // TODO: what should be the state reader here? do we build our own & call deoxys?
            // See: deoxys/crates/client/exec/src/blockifier_state_adapter.rs
            &mut state,
            Some(old_block_number_and_hash),
            block_info,
            chain_info,
            versioned_constants,
        )
        .unwrap(); // TODO: handle the result instead of unsafe unwrap.

        let execution_helper = ExecutionHelperWrapper::new(
            // TODO: build contract_storage_map from snosinput?
            HashMap::default(),
            // TODO: vec of TransactionExecutionInfo
            vec![],
            &block_context,
            // TODO: should be old_block_number_and_hash
            (
                Felt252::from_u64(old_block_number).unwrap(),
                Felt252::from_hex_unchecked(&FieldElement::from(old_block_hash).to_string()),
            ),
        );

        // 3. Import SNOS in Rust and execute it with the input data
        let (_cairo_pie, _snos_output) = run_os(
            // TODO: what is this?
            String::from("PATH/TO/THE/OS"),
            // TODO: what to choose?
            LayoutName::plain,
            snos_input,
            block_context,
            execution_helper,
            // TODO: unsafe unwrap
        )
        .unwrap();

        // 3. Store the received PIE in DB
        // TODO: do we want to store the SnosOutput also?
        todo!()
    }

    async fn verify_job(&self, _config: &Config, _job: &mut JobItem) -> Result<JobVerificationStatus> {
        // No need for verification as of now. If we later on decide to outsource SNOS run
        // to another servicehow a, verify_job can be used to poll on the status of the job
        Ok(JobVerificationStatus::Verified)
    }

    fn max_process_attempts(&self) -> u64 {
        1
    }

    fn max_verification_attempts(&self) -> u64 {
        10
    }

    fn verification_polling_delay_seconds(&self) -> u64 {
        // TODO: adapt this value - what is an average run time for SNOS?
        60
    }
}
