use std::fmt::Write;
use std::io::{BufRead, Cursor, Read};
use std::str::FromStr;

use crate::config::config;
use crate::constants::{BLOB_DATA_FILE_NAME, PROGRAM_OUTPUT_FILE_NAME};
use alloy::primitives::U256;
use color_eyre::eyre::eyre;
use num_bigint::BigUint;

/// Fetching the blob data (stored in remote storage during DA job) for a particular block
pub async fn fetch_blob_data_for_block(block_number: u64) -> color_eyre::Result<Vec<Vec<u8>>> {
    let config = config().await;
    let storage_client = config.storage();
    let key = block_number.to_string() + "/" + BLOB_DATA_FILE_NAME;
    let blob_data = storage_client.get_data(&key).await?;
    Ok(vec![blob_data.to_vec()])
}

/// Fetching the blob data (stored in remote storage during DA job) for a particular block
pub async fn fetch_program_data_for_block(block_number: u64) -> color_eyre::Result<Vec<[u8; 32]>> {
    let config = config().await;
    let storage_client = config.storage();
    let key = block_number.to_string() + "/" + PROGRAM_OUTPUT_FILE_NAME;
    let blob_data = storage_client.get_data(&key).await?;
    let transformed_blob_vec_u8 = bytes_to_vec_u8(blob_data.as_ref());
    Ok(transformed_blob_vec_u8)
}

fn bytes_to_vec_u8(bytes: &[u8]) -> Vec<[u8; 32]> {
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
        // writes a byte value as a two-digit hexadecimal number (padded with a leading zero if necessary) to the specified output.
        let _ = write!(output, "{byte:02x}");
        output
    });
    format!("{:0<64}", hex)
}

#[cfg(test)]
mod test {
    use crate::config::config;
    use crate::constants::BLOB_DATA_FILE_NAME;
    use crate::jobs::da_job::fft_transformation;
    use crate::jobs::state_update_job::utils::{biguint_to_32_bytes, biguint_vec_to_u8_vec, hex_string_to_u8_vec};
    use crate::tests::config::TestConfigBuilder;
    use majin_blob_core::blob;
    use num_bigint::BigUint;
    use rstest::rstest;
    use std::fs;

    #[rstest]
    #[tokio::test]
    async fn test_fetch_blob_data_for_block() -> color_eyre::Result<()> {
        dotenvy::from_filename("../.env.test").expect("Failed to load the .env file");

        TestConfigBuilder::new().build().await;

        let res = biguint_to_32_bytes(
            &BigUint::parse_bytes(b"32114628705813240320780031224394235025697957640420683367072248844003647429056", 10)
                .unwrap(),
        );
        println!("{:?}", res);

        let blob_data = fs::read_to_string(
            "/Users/ocdbytes/Karnot/madara-orchestrator/crates/orchestrator/src/tests/jobs/da_job/test_data/test_blob/671070.txt"
        )
            .unwrap();
        let blob_data_vec = hex_string_to_u8_vec(&blob_data).unwrap();

        // let fetch_from_s3 = fetch_blob_data_for_block(671070).await.unwrap();

        let original_blob_data = majin_blob_types::serde::parse_file_to_blob_data("/Users/ocdbytes/Karnot/madara-orchestrator/crates/orchestrator/src/tests/jobs/da_job/test_data/test_blob/671070.txt");
        let recovered_blob_data = blob::recover(original_blob_data.clone());
        // println!("recovered_blob_data : {:?}", recovered_blob_data.len());
        let fft_blob = fft_transformation(recovered_blob_data);
        // println!("{:?}", fft_blob);
        let fft_blob_vec_u8 = biguint_vec_to_u8_vec(fft_blob.as_slice());

        let key = "671070/".to_string() + BLOB_DATA_FILE_NAME;
        let config = config().await;
        let storage_client = config.storage();

        storage_client.put_data(fft_blob_vec_u8.clone().into(), &key).await.unwrap();

        let blob_data = storage_client.get_data(&key).await?;
        let blob_vec_data = blob_data.to_vec();

        // 131072
        // 32114628705813240320780031224394235025697957640420683367072248844003647429056
        // 7258306880938333000768807635232118825372104144766057789408335876908913440392
        assert_eq!(blob_data_vec, blob_vec_data);

        Ok(())
    }
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
