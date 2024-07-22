mod dummy_state;

use std::collections::HashMap;
use std::num::NonZeroU128;

use async_trait::async_trait;
use blockifier::block::{pre_process_block, BlockInfo, BlockNumberHashPair, GasPrices};
use blockifier::context::{ChainInfo, FeeTokenAddresses};
use blockifier::versioned_constants::VersionedConstants;
use cairo_vm::types::layout_name::LayoutName;
use cairo_vm::Felt252;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use num::FromPrimitive;
use snos::execution::helper::ExecutionHelperWrapper;
use snos::io::input::StarknetOsInput;
use snos::run_os;
use starknet_api::block::{BlockHash, BlockNumber, BlockTimestamp};
use starknet_api::hash::StarkFelt;
use starknet_core::types::FieldElement;
use uuid::Uuid;

use utils::conversions::try_non_zero_u128_from_u128;
use utils::time::get_current_timestamp_in_secs;

use crate::config::Config;
use crate::jobs::snos_job::dummy_state::DummyState;
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;
use crate::rpc::{
    l1::{EthFeeHistory, L1HttpRpcRequests},
    madara::MadaraHttpRpcClient,
};

use super::constants::JOB_METADATA_SNOS_BLOCK;

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

    async fn process_job(&self, config: &Config, job: &mut JobItem) -> Result<String> {
        // 0. Get block number from metadata
        let block_number = self.get_block_number_from_metadata(job)?;

        // 1. Fetch SNOS input data from Madara
        let snos_input: StarknetOsInput = self.get_snos_input_from_madara(config, &block_number).await?;

        // 2. Build the required inputs for snos::run_os
        // TODO: import BlockifierStateAdapter from Madara RPC and use it here.
        // Currently not possible because of dependencies versions conflicts between
        // SNOS, cairo-vm and madara.
        let mut state = DummyState {};

        let block_number_and_hash = BlockNumberHashPair {
            number: block_number,
            hash: BlockHash(StarkFelt::from(
                FieldElement::from_bytes_be(&snos_input.block_hash.clone().to_bytes_be())
                    .expect("Could not convert Felt to FieldElement"),
            )),
        };

        let block_info = BlockInfo {
            block_number,
            // TODO: Assert that we really want current_timestamp?
            block_timestamp: BlockTimestamp(get_current_timestamp_in_secs()),
            sequencer_address: snos_input.general_config.sequencer_address,
            gas_prices: self.get_gas_prices_from_l1(config).await?,
            use_kzg_da: snos_input.general_config.use_kzg_da,
        };

        let chain_info = ChainInfo {
            chain_id: snos_input.general_config.starknet_os_config.chain_id.clone(),
            fee_token_addresses: FeeTokenAddresses {
                eth_fee_token_address: snos_input.general_config.starknet_os_config.fee_token_address,
                // TODO: assert that the STRK fee token address is [deprecated_fee_token_address]
                strk_fee_token_address: snos_input.general_config.starknet_os_config.deprecated_fee_token_address,
            },
        };

        let block_context = match pre_process_block(
            &mut state,
            Some(block_number_and_hash),
            block_info,
            chain_info,
            VersionedConstants::latest_constants().clone(),
        ) {
            Ok(block_context) => block_context,
            Err(e) => return Err(eyre!("pre_process_block failed for block #{}: {}", block_number, e)),
        };

        // TODO: contract_storage_map should be retrieved from where?
        let contract_storage_map = HashMap::default();
        let execution_helper = ExecutionHelperWrapper::new(
            contract_storage_map,
            vec![], // TODO: vec of TransactionExecutionInfo, how to get it?
            &block_context,
            (Felt252::from_u64(block_number.0).unwrap(), snos_input.block_hash),
        );

        // 3. Import SNOS in Rust and execute it with the input data
        let (_cairo_pie, _snos_output) = match run_os(
            // TODO: what is this path?
            String::from("PATH/TO/THE/OS"),
            // TODO: which layout should we choose?
            LayoutName::plain,
            snos_input,
            block_context,
            execution_helper,
        ) {
            Ok((cairo_pie, snos_output)) => (cairo_pie, snos_output),
            Err(e) => return Err(eyre!("Could not run SNOS for block #{}: {}", block_number, e)),
        };

        // 3. Store the received PIE in DB
        // TODO: Store the PIE & the SnosOutput once S3 is implemented
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
        1
    }

    fn verification_polling_delay_seconds(&self) -> u64 {
        1
    }
}

impl SnosJob {
    /// Get the block number that needs to be run with SNOS for the current
    /// job.
    fn get_block_number_from_metadata(&self, job: &JobItem) -> Result<BlockNumber> {
        let block_number: u64 = job
            .metadata
            .get(JOB_METADATA_SNOS_BLOCK)
            .ok_or_else(|| eyre!("Block number to run with SNOS must be specified (snos job #{})", job.internal_id))?
            .parse()?;
        Ok(BlockNumber(block_number))
    }

    /// Retrieves the [StarknetOsInput] for the provided block number from Madara.
    async fn get_snos_input_from_madara(&self, config: &Config, block_number: &BlockNumber) -> Result<StarknetOsInput> {
        let http_rpc_client = config.http_rpc_client();
        let snos_input = http_rpc_client.get_snos_input(block_number).await?;
        Ok(snos_input)
    }

    /// Retrieves the ETH & STRK gas prices and returns them in a [GasPrices].
    /// TODO: We only retrieve the ETH gas price for now. For STRK, we need to implement
    /// a logic to fetch the live price of ETH <=> STRK from an Oracle.
    async fn get_gas_prices_from_l1(&self, config: &Config) -> Result<GasPrices> {
        let http_rpc_client = config.http_rpc_client();
        let fee_history = http_rpc_client.get_eth_fee_history().await?;

        let (eth_l1_gas_price, eth_l1_data_gas_price) = self.compute_eth_gas_prices_from_history(fee_history)?;

        let gas_prices = GasPrices {
            eth_l1_gas_price,
            eth_l1_data_gas_price,
            // TODO: Logic for fetching from an Oracle
            strk_l1_gas_price: try_non_zero_u128_from_u128(1)?,
            strk_l1_data_gas_price: try_non_zero_u128_from_u128(1)?,
        };
        Ok(gas_prices)
    }

    /// Compute the l1_gas_price and l1_data_gas_price from the [EthFeeHistory].
    /// Source: https://github.com/keep-starknet-strange/madara/blob/7b405924b5859fdfa24ec33f152e56a97a047e31/crates/client/l1-gas-price/src/worker.rs#L31
    fn compute_eth_gas_prices_from_history(&self, fee_history: EthFeeHistory) -> Result<(NonZeroU128, NonZeroU128)> {
        let (_, blob_fee_history_one_hour) =
            fee_history.base_fee_per_blob_gas.split_at(fee_history.base_fee_per_blob_gas.len().max(300) - 300);

        let avg_blob_base_fee = blob_fee_history_one_hour
            .iter()
            .map(|hex_str| u128::from_str_radix(&hex_str[2..], 16).unwrap())
            .sum::<u128>()
            / blob_fee_history_one_hour.len() as u128;

        let eth_gas_price = u128::from_str_radix(
            fee_history
                .base_fee_per_gas
                .last()
                .ok_or(eyre!("Failed to get last element of `base_fee_per_gas`"))?
                .trim_start_matches("0x"),
            16,
        )?;

        Ok((try_non_zero_u128_from_u128(eth_gas_price)?, try_non_zero_u128_from_u128(avg_blob_base_fee)?))
    }
}
