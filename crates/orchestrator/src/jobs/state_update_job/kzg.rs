use crate::data_storage::aws_s3::config::AWSS3Config;
use crate::data_storage::aws_s3::AWSS3;
use crate::data_storage::{DataStorage, DataStorageConfig};
use crate::jobs::state_update_job::CURRENT_PATH;
use alloy::eips::eip4844::BYTES_PER_BLOB;
use c_kzg::{Blob, Bytes32, KzgCommitment, KzgProof, KzgSettings};

/// Build KZG proof for a given block
pub async fn build_kzg_proof(block_number: u64, fetch_from_tests: Option<bool>) -> color_eyre::Result<KzgProof> {
    let blob_data = fetch_blob_data_for_block(block_number, fetch_from_tests).await?;
    let mut fixed_size_blob: [u8; BYTES_PER_BLOB] = [0; BYTES_PER_BLOB];
    fixed_size_blob.copy_from_slice(blob_data.as_slice());

    let x_0_value = fetch_x_0_value_from_os_output(block_number, fetch_from_tests).await?;

    // trusted setup ceremony
    let trusted_setup_path = CURRENT_PATH.join("src/jobs/state_update_job/trusted_setup.txt");
    let trusted_setup =
        KzgSettings::load_trusted_setup_file(trusted_setup_path.as_path()).expect("Error loading trusted setup file");

    let blob = Blob::new(fixed_size_blob);
    let commitment = KzgCommitment::blob_to_kzg_commitment(&blob, &trusted_setup)?;
    let (kzg_proof, y_0_value) = KzgProof::compute_kzg_proof(&blob, &x_0_value, &trusted_setup)?;

    // Verifying the proof for double check
    let eval = KzgProof::verify_kzg_proof(
        &commitment.to_bytes(),
        &x_0_value,
        &y_0_value,
        &kzg_proof.to_bytes(),
        &trusted_setup,
    )?;
    assert!(eval);

    Ok(kzg_proof)
}

/// Fetching the blob data (stored in s3 during DA job) for a particular block
pub async fn fetch_blob_data_for_block(
    block_number: u64,
    fetch_from_tests: Option<bool>,
) -> color_eyre::Result<Vec<u8>> {
    let fetch_from_tests = fetch_from_tests.unwrap_or(true);
    let blob_data: Vec<u8> = match fetch_from_tests {
        true => {
            let blob_data_path =
                CURRENT_PATH.join(format!("src/jobs/state_update_job/test_data/{}/blob_data.txt", block_number));
            let data = std::fs::read_to_string(blob_data_path).expect("Failed to read the blob data txt file");
            hex_string_to_u8_vec(&data).unwrap()
        }
        false => {
            let s3_client = AWSS3::new(AWSS3Config::new_from_env()).await;
            let key = block_number.to_string() + "/blob_data.txt";
            let blob_data = s3_client.get_data(&key).await?;
            blob_data.to_vec()
        }
    };

    Ok(blob_data)
}

pub async fn fetch_x_0_value_from_os_output(
    block_number: u64,
    fetch_from_tests: Option<bool>,
) -> color_eyre::Result<Bytes32> {
    let fetch_from_tests = fetch_from_tests.unwrap_or(true);
    let x_0 = match fetch_from_tests {
        true => {
            let x_0_path = CURRENT_PATH.join(format!("src/jobs/state_update_job/test_data/{}/x_0.txt", block_number));
            let data = std::fs::read_to_string(x_0_path)?;
            Bytes32::from_hex(&data).unwrap()
        }
        false => unimplemented!(),
    };

    Ok(x_0)
}

// Util Functions
// ===============

/// Util function to convert hex string data into Vec<u8>
fn hex_string_to_u8_vec(hex_str: &str) -> color_eyre::Result<Vec<u8>, String> {
    // Remove any spaces or non-hex characters from the input string
    let cleaned_str: String = hex_str.chars().filter(|c| c.is_ascii_hexdigit()).collect();

    // Convert the cleaned hex string to a Vec<u8>
    let mut result = Vec::new();
    for chunk in cleaned_str.as_bytes().chunks(2) {
        if let Ok(byte_val) = u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16) {
            result.push(byte_val);
        } else {
            return Err(format!("Error parsing hex string: {}", cleaned_str));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::jobs::state_update_job::kzg::build_kzg_proof;
    use c_kzg::Bytes48;
    use rstest::rstest;

    #[rstest]
    #[case(630872)]
    #[tokio::test]
    async fn test_build_kzg_proof(#[case] block_number: u64) {
        // testing the data in transaction :
        // https://etherscan.io/tx/0x6b9fc547764a5d6e4451b5236b92e74c70800250f00fc1974fc0a75a459dc12e
        let kzg_proof = build_kzg_proof(block_number, Some(true)).await.unwrap().to_bytes();
        let original_proof_from_l1 = Bytes48::from_hex(
            "a168b317e7c44691ee1932bd12fc6ac22182277e8fc5cd4cd21adc0831c33b1359aa5171bba529c69dcfe6224b220f8f",
        )
        .unwrap();

        assert_eq!(kzg_proof, original_proof_from_l1);
    }
}
