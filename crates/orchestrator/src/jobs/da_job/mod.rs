use crate::config::Config;
// use crate::jobs::constants::{BLOB_LEN, BLS_MODULUS, GENERATOR};
use crate::jobs::types::{JobItem, JobStatus, JobType, JobVerificationStatus};
use crate::jobs::Job;
use async_trait::async_trait;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use lazy_static::lazy_static;
use num_bigint::{BigUint, ToBigUint};
use num_traits::Num;
use num_traits::{One, Zero};
use std::ops::{Add, Mul, Rem};
use std::str::FromStr;

use starknet::core::types::{BlockId, FieldElement, MaybePendingStateUpdate, StateUpdate, StorageEntry};
use starknet::providers::Provider;
use std::collections::HashMap;
use tracing::log;
use uuid::Uuid;

// @note: can be remo
pub const BLOB_LEN: usize = 4096;
// lazy_static! {
//     /// EIP-4844 BLS12-381 modulus.
//     ///
//     /// As defined in https://eips.ethereum.org/EIPS/eip-4844

//     /// Generator of the group of evaluation points (EIP-4844 parameter).
//     pub static ref GENERATOR: BigUint = BigUint::from_str(
//         "39033254847818212395286706435128746857159659164139250548781411570340225835782",
//     )
//     .unwrap();

//     pub static ref BLS_MODULUS: BigUint = BigUint::from_str(
//         "52435875175126190479447740508185965837690552500527637822603658699938581184513",
//     )
//     .unwrap();
//     pub static ref TWO: BigUint = 2u32.to_biguint().unwrap();
// }

pub struct DaJob;

#[async_trait]
impl Job for DaJob {
    async fn create_job(&self, _config: &Config, internal_id: String) -> Result<JobItem> {
        Ok(JobItem {
            id: Uuid::new_v4(),
            internal_id,
            job_type: JobType::DataSubmission,
            status: JobStatus::Created,
            external_id: String::new().into(),
            metadata: HashMap::new(),
            version: 0,
        })
    }

    async fn process_job(&self, config: &Config, job: &JobItem) -> Result<String> {
        let block_no = job.internal_id.parse::<u64>()?;
        let state_update = config.starknet_client().get_state_update(BlockId::Number(block_no)).await?;

        let state_update = match state_update {
            MaybePendingStateUpdate::PendingUpdate(_) => {
                log::error!("Cannot process block {} for job id {} as it's still in pending state", block_no, job.id);
                return Err(eyre!(
                    "Cannot process block {} for job id {} as it's still in pending state",
                    block_no,
                    job.id
                ));
            }
            MaybePendingStateUpdate::Update(state_update) => state_update,
        };
        // constructing the data from the rpc
        let blob_data = state_update_to_blob_data(block_no, state_update);
        // transforming the data so that we can apply FFT on this.
        // @note: we can skip this step if in the above step we return vec<BigUint> directly
        let blob_data_biguint = convert_to_biguint(blob_data.clone());
        // data transformation on the data
        let transformed_data = fft_transformation(blob_data_biguint);

        let max_bytes_per_blob = config.da_client().max_bytes_per_blob().await;
        let max_blob_per_txn = config.da_client().max_blob_per_txn().await;

        // converting BigUints to Vec<u8>, one Vec<u8> represents one blob data
        let blob_array = data_to_blobs(max_bytes_per_blob, transformed_data);
        let current_blob_length: u64 = blob_array.len().try_into().unwrap();

        // there is a limit on number of blobs per txn, checking that here
        if current_blob_length > max_blob_per_txn {
            return Err(eyre!(
                "Cannot process block {} for job id {} as number of blobs allowd are {} but it contains {} blobs",
                block_no,
                job.id,
                max_blob_per_txn,
                current_blob_length
            ));
        }

        // making the txn to the DA layer
        let external_id = config.da_client().publish_state_diff(blob_array).await?;

        Ok(external_id)
    }

    async fn verify_job(&self, config: &Config, job: &JobItem) -> Result<JobVerificationStatus> {
        Ok(config.da_client().verify_inclusion(job.external_id.unwrap_string()?).await?.into())
    }

    fn max_process_attempts(&self) -> u64 {
        1
    }

    fn max_verification_attempts(&self) -> u64 {
        3
    }

    fn verification_polling_delay_seconds(&self) -> u64 {
        60
    }
}

fn fft_transformation(elements: Vec<BigUint>) -> Vec<BigUint> {
    // @note: this should be in constants ideally
    let BLS_MODULUS: BigUint =
        BigUint::from_str("52435875175126190479447740508185965837690552500527637822603658699938581184513").unwrap();
    let GENERATOR: BigUint =
        BigUint::from_str("39033254847818212395286706435128746857159659164139250548781411570340225835782").unwrap();

    let xs: Vec<BigUint> = (0..BLOB_LEN)
        .map(|i| {
            let bin = format!("{:012b}", i);
            let bin_rev = bin.chars().rev().collect::<String>();
            GENERATOR.modpow(&BigUint::from_str_radix(&bin_rev, 2).unwrap(), &BLS_MODULUS)
        })
        .collect();
    let n = elements.len();
    let mut transform: Vec<BigUint> = vec![BigUint::zero(); n];

    for i in 0..n {
        for j in (0..n).rev() {
            transform[i] = (transform[i].clone().mul(&xs[i]).add(&elements[j])).rem(&BLS_MODULUS);
        }
    }
    transform
}

fn convert_to_biguint(elements: Vec<FieldElement>) -> Vec<BigUint> {
    let mut biguint_vec = Vec::with_capacity(4096);

    // Iterate over the first 4096 elements of the input vector or until we reach 4096 elements
    for i in 0..4096 {
        if let Some(element) = elements.get(i) {
            // Convert FieldElement to [u8; 32]
            let bytes: [u8; 32] = element.to_bytes_be();

            // Convert [u8; 32] to BigUint
            let biguint = BigUint::from_bytes_be(&bytes);

            biguint_vec.push(biguint);
        } else {
            // If we run out of elements, push a zero BigUint
            biguint_vec.push(BigUint::zero());
        }
    }

    biguint_vec
}

fn data_to_blobs(blob_size: u64, block_data: Vec<BigUint>) -> Vec<Vec<u8>> {
    // Validate blob size
    if blob_size < 32 {
        panic!("Blob size must be at least 32 bytes to accommodate a single FieldElement/BigUint");
    }

    let mut blobs: Vec<Vec<u8>> = Vec::new();

    // Convert all FieldElements to bytes first
    let mut bytes: Vec<u8> = block_data.iter().flat_map(|element| element.to_bytes_be().to_vec()).collect();

    // Process bytes in chunks of blob_size
    while bytes.len() >= blob_size as usize {
        let chunk = bytes.drain(..blob_size as usize).collect();
        blobs.push(chunk);
    }

    // Handle any remaining bytes (not a complete blob)
    if !bytes.is_empty() {
        let remaining_bytes = bytes.len();
        let mut last_blob = bytes;
        last_blob.resize(blob_size as usize, 0); // Pad with zeros
        blobs.push(last_blob);
        println!("Warning: Remaining {} bytes not forming a complete blob were padded", remaining_bytes);
    }

    return blobs;
}

fn state_update_to_blob_data(block_no: u64, state_update: StateUpdate) -> Vec<FieldElement> {
    let state_diff = state_update.state_diff;
    let mut blob_data: Vec<FieldElement> = vec![
        // TODO: confirm first three fields
        FieldElement::from(state_diff.storage_diffs.len()),
        // @note: won't need this if while producing the block we are attaching the block number
        // and the block hash
        FieldElement::ONE,
        FieldElement::ONE,
        FieldElement::from(block_no),
        state_update.block_hash,
    ];

    let storage_diffs: HashMap<FieldElement, &Vec<StorageEntry>> =
        state_diff.storage_diffs.iter().map(|item| (item.address, &item.storage_entries)).collect();
    let declared_classes: HashMap<FieldElement, FieldElement> =
        state_diff.declared_classes.iter().map(|item| (item.class_hash, item.compiled_class_hash)).collect();
    let deployed_contracts: HashMap<FieldElement, FieldElement> =
        state_diff.deployed_contracts.iter().map(|item| (item.address, item.class_hash)).collect();
    let replaced_classes: HashMap<FieldElement, FieldElement> =
        state_diff.replaced_classes.iter().map(|item| (item.contract_address, item.class_hash)).collect();
    let mut nonces: HashMap<FieldElement, FieldElement> =
        state_diff.nonces.iter().map(|item| (item.contract_address, item.nonce)).collect();

    // Loop over storage diffs
    for (addr, writes) in storage_diffs {
        let class_flag = deployed_contracts.get(&addr).or_else(|| replaced_classes.get(&addr));

        let nonce = nonces.remove(&addr);
        let da_word = da_word(class_flag.is_some(), nonce, writes.len() as u64);
        // @note: it can be improved if the first push to the data is of block number and hash
        // @note: ONE address is special address which for now has 1 value and that is current
        //        block number and hash
        // @note: ONE special address can be used to mark the range of block, if in future
        //        the team wants to submit multiple blocks in a sinle blob etc.
        if (addr == FieldElement::ONE && da_word == FieldElement::ONE) {
            continue;
        }
        blob_data.push(addr);
        blob_data.push(da_word);

        if let Some(class_hash) = class_flag {
            blob_data.push(*class_hash);
        }

        for entry in writes {
            blob_data.push(entry.key);
            blob_data.push(entry.value);
        }
    }

    // Handle declared classes
    blob_data.push(FieldElement::from(declared_classes.len()));

    for (class_hash, compiled_class_hash) in &declared_classes {
        blob_data.push(*class_hash);
        blob_data.push(*compiled_class_hash);
    }

    blob_data
}

/// DA word encoding:
/// |---padding---|---class flag---|---new nonce---|---num changes---|
///     127 bits        1 bit           64 bits          64 bits
fn da_word(class_flag: bool, nonce_change: Option<FieldElement>, num_changes: u64) -> FieldElement {
    // padding of 127 bits
    let mut binary_string = "0".repeat(127);

    // class flag of one bit
    if (class_flag) {
        binary_string += "1"
    } else {
        binary_string += "0"
    }

    // checking for nonce here
    if let Some(new_nonce) = nonce_change {
        let bytes: [u8; 32] = nonce_change.unwrap().to_bytes_be();
        let biguint = BigUint::from_bytes_be(&bytes);
        let binary_string_local = format!("{:b}", biguint);
        let padded_binary_string = format!("{:0>64}", binary_string_local);
        binary_string += &padded_binary_string;
    } else {
        let mut binary_string_local = "0".repeat(64);
        binary_string += &binary_string_local;
    }

    let binary_representation = format!("{:b}", num_changes);
    let padded_binary_string = format!("{:0>64}", binary_representation);
    binary_string += &padded_binary_string;

    let biguint = BigUint::from_str_radix(binary_string.as_str(), 2).expect("Invalid binary string");

    // Now convert the BigUint to a decimal string
    let decimal_string = biguint.to_str_radix(10);

    let word = FieldElement::from_dec_str(&decimal_string).expect("issue while converting to fieldElement");

    word
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(false, 1, 1, "18446744073709551617")]
    #[case(false, 1, 0, "18446744073709551616")]
    #[case(false, 0, 6, "6")]
    #[case(true, 1, 0, "340282366920938463481821351505477763072")]
    fn da_word_works(
        #[case] class_flag: bool,
        #[case] new_nonce: u64,
        #[case] num_changes: u64,
        #[case] expected: String,
    ) {
        let new_nonce = if new_nonce > 0 { Some(FieldElement::from(new_nonce)) } else { None };
        let da_word = da_word(class_flag, new_nonce, num_changes);
        let expected = FieldElement::from_dec_str(expected.as_str()).unwrap();
        assert_eq!(da_word, expected);
    }
}
