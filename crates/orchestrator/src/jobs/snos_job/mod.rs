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
use starknet_api::hash::StarkFelt;
use starknet_core::types::FieldElement;
use uuid::Uuid;

use utils::time::get_current_timestamp_in_secs;

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
        // 0. Get block number from metadata
        let block_number = BlockNumber(42_u64);

        // 1. Fetch SNOS input data from Madara
        let snos_input: StarknetOsInput = self.get_snos_input_from_madara(&block_number)?;

        // 2. Build the required inputs for snos::run_os
        // TODO: import BlockifierStateAdapter from Madara RPC and use it here
        let mut state = DummyState {};

        let block_number_and_hash = BlockNumberHashPair {
            number: block_number,
            // NOTE: 😹😹😹😹😹
            hash: BlockHash(StarkFelt::from(
                FieldElement::from_bytes_be(&snos_input.block_hash.clone().to_bytes_be())
                    .expect("Could not convert Felt to FieldElement 😹"),
            )),
        };

        let block_info = BlockInfo {
            block_number,
            // TODO: Assert that we really want current_timestamp?
            block_timestamp: BlockTimestamp(get_current_timestamp_in_secs()),
            sequencer_address: snos_input.general_config.sequencer_address,
            // TODO: retrieve them from Madara? & assert that they're in gas_price_bounds?
            gas_prices: GasPrices {
                eth_l1_gas_price: NonZeroU128::new(0).unwrap(),
                strk_l1_gas_price: NonZeroU128::new(0).unwrap(),
                eth_l1_data_gas_price: NonZeroU128::new(0).unwrap(),
                strk_l1_data_gas_price: NonZeroU128::new(0).unwrap(),
            },
            use_kzg_da: snos_input.general_config.use_kzg_da,
        };

        let chain_info = ChainInfo {
            chain_id: snos_input.general_config.starknet_os_config.chain_id.clone(),
            fee_token_addresses: FeeTokenAddresses {
                eth_fee_token_address: snos_input.general_config.starknet_os_config.fee_token_address,
                // TODO: assert that the STRK fee token address is deprecated_fee_token_address
                strk_fee_token_address: snos_input.general_config.starknet_os_config.deprecated_fee_token_address,
            },
        };

        let block_context = pre_process_block(
            &mut state,
            Some(block_number_and_hash),
            block_info,
            chain_info,
            VersionedConstants::latest_constants().clone(),
        )
        // TODO: Handle result instead of unsafe unwrap
        .unwrap();

        // TODO: contract_storage_map should be retrieved from the state?
        let contract_storage_map = HashMap::default();
        let execution_helper = ExecutionHelperWrapper::new(
            contract_storage_map,
            vec![], // TODO: vec of TransactionExecutionInfo, how to get it?
            &block_context,
            (Felt252::from_u64(block_number.0).unwrap(), snos_input.block_hash),
        );

        // 3. Import SNOS in Rust and execute it with the input data
        let (_cairo_pie, _snos_output) = run_os(
            // TODO: what is this path?
            String::from("PATH/TO/THE/OS"),
            // TODO: which layout should we choose?
            LayoutName::plain,
            snos_input,
            block_context,
            execution_helper,
        )
        // TODO: Handle result instead of unsafe unwrap
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
        // TODO: isn't 10 a lot?
        10
    }

    fn verification_polling_delay_seconds(&self) -> u64 {
        // TODO: what is an average run time for SNOS?
        60
    }
}

impl SnosJob {
    fn get_snos_input_from_madara(&self, _block_number: &BlockNumber) -> Result<StarknetOsInput> {
        // TODO: JSON RPC call to `getSnosInput` for a specific block
        let snos_input = StarknetOsInput::load(std::path::Path::new("i_do_not_exist_😹.txt")).unwrap();
        Ok(snos_input)
    }
}
