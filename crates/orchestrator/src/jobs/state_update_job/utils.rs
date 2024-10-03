use std::fmt::Write;
use std::io::{BufRead, Cursor};
use std::str::FromStr;
use std::sync::Arc;

use alloy::primitives::U256;
use cairo_vm::Felt252;
use color_eyre::eyre::eyre;
use num_bigint::BigUint;
use starknet_os::io::output::{ContractChanges, StarknetOsOutput};

use crate::config::Config;
use crate::constants::{BLOB_DATA_FILE_NAME, PROGRAM_OUTPUT_FILE_NAME};

/// Fetching the blob data (stored in remote storage during DA job) for a particular block
pub async fn fetch_blob_data_for_block(block_number: u64, config: Arc<Config>) -> color_eyre::Result<Vec<Vec<u8>>> {
    let storage_client = config.storage();
    let key = block_number.to_string() + "/" + BLOB_DATA_FILE_NAME;
    let blob_data = storage_client.get_data(&key).await?;
    Ok(vec![blob_data.to_vec()])
}

/// Fetching the blob data (stored in remote storage during DA job) for a particular block
pub async fn fetch_program_data_for_block(block_number: u64, config: Arc<Config>) -> color_eyre::Result<Vec<[u8; 32]>> {
    let storage_client = config.storage();
    let key = block_number.to_string() + "/" + PROGRAM_OUTPUT_FILE_NAME;
    let blob_data = storage_client.get_data(&key).await?;
    let transformed_blob_vec_u8 = bytes_to_vec_u8(blob_data.as_ref());
    Ok(transformed_blob_vec_u8)
}

// Util Functions
// ===============

/// Util function to convert hex string data into Vec<u8>
pub fn hex_string_to_u8_vec(hex_str: &str) -> color_eyre::Result<Vec<u8>> {
    // Remove any spaces or non-hex characters from the input string
    let cleaned_str: String = hex_str.chars().filter(|c| c.is_ascii_hexdigit()).collect();

    // Convert the cleaned hex string to a Vec<u8>
    let mut result = Vec::new();
    for chunk in cleaned_str.as_bytes().chunks(2) {
        if let Ok(byte_val) = u8::from_str_radix(std::str::from_utf8(chunk)?, 16) {
            result.push(byte_val);
        } else {
            return Err(eyre!("Error parsing hex string: {}", cleaned_str));
        }
    }

    Ok(result)
}

pub fn bytes_to_vec_u8(bytes: &[u8]) -> Vec<[u8; 32]> {
    let cursor = Cursor::new(bytes);
    let reader = std::io::BufReader::new(cursor);

    let mut program_output: Vec<[u8; 32]> = Vec::new();

    for line in reader.lines() {
        let line = line.expect("can't read line");
        let trimmed = line.trim();
        assert!(!trimmed.is_empty());

        let result = U256::from_str(trimmed).expect("Unable to convert line");
        let res_vec = result.to_be_bytes_vec();
        let hex = to_padded_hex(res_vec.as_slice());
        let vec_hex = hex_string_to_u8_vec(&hex).unwrap();
        program_output.push(vec_hex.try_into().unwrap());
    }

    program_output
}

fn to_padded_hex(slice: &[u8]) -> String {
    assert!(slice.len() <= 32, "Slice length must not exceed 32");
    let hex = slice.iter().fold(String::new(), |mut output, byte| {
        // 0: pads with zeros
        // 2: specifies the minimum width (2 characters)
        // x: formats the number as lowercase hexadecimal
        // writes a byte value as a two-digit hexadecimal number (padded with a leading zero if necessary)
        // to the specified output.
        let _ = write!(output, "{byte:02x}");
        output
    });
    format!("{:0<64}", hex)
}

pub fn biguint_vec_to_u8_vec(nums: &[BigUint]) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();

    for num in nums {
        result.extend_from_slice(biguint_to_32_bytes(num).as_slice());
    }

    result
}

pub fn biguint_to_32_bytes(num: &BigUint) -> [u8; 32] {
    let bytes = num.to_bytes_be();
    let mut result = [0u8; 32];

    if bytes.len() > 32 {
        // If we have more than 32 bytes, take only the last 32
        result.copy_from_slice(&bytes[bytes.len() - 32..]);
    } else {
        // If we have 32 or fewer bytes, pad with zeros at the beginning
        result[32 - bytes.len()..].copy_from_slice(&bytes);
    }

    result
}

// =========================================================
// Starknet OS output encoder function
// =========================================================

pub fn encode_output(output: &StarknetOsOutput) -> Vec<[u8; 32]> {
    let mut encoded = Vec::new();

    // Helper function to convert Felt252 to [u8; 32]
    fn felt_to_bytes(felt: &Felt252) -> [u8; 32] {
        let bytes = felt.to_bytes_be();
        let mut result = [0u8; 32];
        result[32 - bytes.len()..].copy_from_slice(&bytes);
        result
    }

    // Encode header
    encoded.push(felt_to_bytes(&output.initial_root));
    encoded.push(felt_to_bytes(&output.final_root));
    encoded.push(felt_to_bytes(&output.prev_block_number));
    encoded.push(felt_to_bytes(&output.new_block_number));
    encoded.push(felt_to_bytes(&output.prev_block_hash));
    encoded.push(felt_to_bytes(&output.new_block_hash));
    encoded.push(felt_to_bytes(&output.os_program_hash));
    encoded.push(felt_to_bytes(&output.starknet_os_config_hash));
    encoded.push(felt_to_bytes(&output.use_kzg_da));
    encoded.push(felt_to_bytes(&output.full_output));

    // Encode KZG data if use_kzg_da is true
    if output.use_kzg_da == Felt252::ONE {
        // This part is not fully reversible from the given decode function
        // You'll need to add the correct KZG data here
        encoded.push([0u8; 32]); // Placeholder for n_blobs
        encoded.push([0u8; 32]); // Placeholder for other KZG data
        // Add more placeholders for commitments and evaluations if needed
    }

    // Encode messages_to_l1
    encode_variable_length_segment(&mut encoded, &output.messages_to_l1);

    // Encode messages_to_l2
    encode_variable_length_segment(&mut encoded, &output.messages_to_l2);

    // Encode contracts and classes if not using KZG DA
    if output.use_kzg_da == Felt252::ZERO {
        // Encode contracts
        encoded.push(felt_to_bytes(&Felt252::from(output.contracts.len())));
        for contract in &output.contracts {
            encode_contract_changes(&mut encoded, contract);
        }

        // Encode classes
        // encoded.push(felt_to_bytes(&Felt252::from(output.classes.len())));
        // for (class_hash, class_update) in &output.classes {
        //     encoded.push(felt_to_bytes(class_hash));
        //     encoded.push(felt_to_bytes(&class_update.class_hash));
        //     encoded.push(felt_to_bytes(&Felt252::from(class_update.compiled_class_hash.is_some()
        // as u8)));     if let Some(compiled_class_hash) = class_update.compiled_class_hash
        // {         encoded.push(felt_to_bytes(&compiled_class_hash));
        //     }
        // }
    }

    encoded
}

// Helper function to encode variable length segments
fn encode_variable_length_segment(encoded: &mut Vec<[u8; 32]>, segment: &[Felt252]) {
    encoded.push(felt_to_bytes(&Felt252::from(segment.len())));
    for felt in segment {
        encoded.push(felt_to_bytes(felt));
    }
}

// Helper function to encode contract changes
fn encode_contract_changes(encoded: &mut Vec<[u8; 32]>, contract: &ContractChanges) {
    encoded.push(felt_to_bytes(&contract.addr));
    encoded.push(felt_to_bytes(&contract.nonce));
    encoded.push(felt_to_bytes(&Felt252::from(contract.class_hash.is_some() as u8)));
    if let Some(class_hash) = contract.class_hash {
        encoded.push(felt_to_bytes(&class_hash));
    }
    encoded.push(felt_to_bytes(&Felt252::from(contract.storage_changes.len())));
    for (key, value) in &contract.storage_changes {
        encoded.push(felt_to_bytes(key));
        encoded.push(felt_to_bytes(value));
    }
}

// Helper function to convert Felt252 to [u8; 32]
fn felt_to_bytes(felt: &Felt252) -> [u8; 32] {
    let bytes = felt.to_bytes_be();
    let mut result = [0u8; 32];
    result[32 - bytes.len()..].copy_from_slice(&bytes);
    result
}
