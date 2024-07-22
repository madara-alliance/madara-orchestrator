use crate::config::config;
use crate::constants::{BLOB_DATA_FILE_NAME, X_0_FILE_NAME};
use crate::jobs::state_update_job::CURRENT_PATH;
use alloy::eips::eip4844::BYTES_PER_BLOB;
use c_kzg::{Blob, Bytes32, KzgCommitment, KzgProof, KzgSettings};
use color_eyre::eyre::eyre;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref KZG_SETTINGS: KzgSettings = KzgSettings::load_trusted_setup_file(
        CURRENT_PATH.join("src/jobs/state_update_job/trusted_setup.txt").as_path()
    )
    .expect("Error loading trusted setup file");
}

/// Build KZG proof for a given block
/// For test only
pub async fn build_kzg_proof(blob_data: Vec<u8>, x_0_value: Bytes32) -> color_eyre::Result<(Vec<u8>, KzgProof)> {
    let fixed_size_blob: [u8; BYTES_PER_BLOB] = blob_data.as_slice().try_into()?;

    let blob = Blob::new(fixed_size_blob);
    let commitment = KzgCommitment::blob_to_kzg_commitment(&blob, &KZG_SETTINGS)?;
    let (kzg_proof, y_0_value) = KzgProof::compute_kzg_proof(&blob, &x_0_value, &KZG_SETTINGS)?;

    // Verifying the proof for double check
    let eval = KzgProof::verify_kzg_proof(
        &commitment.to_bytes(),
        &x_0_value,
        &y_0_value,
        &kzg_proof.to_bytes(),
        &KZG_SETTINGS,
    )?;

    if !eval {
        Err(eyre!("ERROR : Assertion failed, not able to verify the proof."))
    } else {
        Ok((blob_data, kzg_proof))
    }
}

/// Fetching the blob data (stored in s3 during DA job) for a particular block
pub async fn fetch_blob_data_for_block(block_number: u64) -> color_eyre::Result<Vec<Vec<u8>>> {
    let config = config().await;
    let storage_client = config.storage();
    let key = block_number.to_string() + "/" + BLOB_DATA_FILE_NAME;
    let blob_data = storage_client.get_data(&key).await?;
    let blob_vec_data: Vec<Vec<u8>> =
        bincode::deserialize(&blob_data).expect("Not able to convert Vec<u8> to Vec<Vec<u8>> during deserialization.");
    Ok(blob_vec_data)
}

pub async fn fetch_x_0_value_from_os_output(block_number: u64) -> color_eyre::Result<Bytes32> {
    let config = config().await;
    let storage_client = config.storage();
    let key = block_number.to_string() + "/" + X_0_FILE_NAME;
    let x_0_point = storage_client.get_data(&key).await?;
    let x_0_point_string =
        std::str::from_utf8(x_0_point.as_ref()).expect("Not able to convert the x_0 point into string");
    Ok(Bytes32::from_hex(x_0_point_string)?)
}

// Util Functions
// ===============

/// Util function to convert hex string data into Vec<u8>
pub fn hex_string_to_u8_vec(hex_str: &str) -> color_eyre::Result<Vec<u8>, String> {
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
    use crate::config::config_force_init;
    use crate::data_storage::MockDataStorage;
    use crate::jobs::state_update_job::kzg::{
        build_kzg_proof, fetch_blob_data_for_block, fetch_x_0_value_from_os_output, hex_string_to_u8_vec,
        BLOB_DATA_FILE_NAME, X_0_FILE_NAME,
    };
    use crate::jobs::state_update_job::CURRENT_PATH;
    use crate::tests::common::init_config;
    use bytes::Bytes;
    use c_kzg::Bytes48;
    use mockall::predicate::eq;
    use rstest::rstest;

    #[rstest]
    #[case(630872)]
    #[tokio::test]
    async fn test_build_kzg_proof(#[case] block_number: u64) {
        let mut storage_client = MockDataStorage::new();

        // Mocking Data Storage Client
        let blob_data_key = block_number.to_string() + "/" + BLOB_DATA_FILE_NAME;
        let x_0_key = block_number.to_string() + "/" + X_0_FILE_NAME;

        let blob_data = std::fs::read_to_string(
            CURRENT_PATH.join(format!("src/tests/jobs/state_update_job/test_data/{}/{}", block_number, BLOB_DATA_FILE_NAME)),
        )
        .expect("Failed to read the blob data txt file");
        let x_0 = std::fs::read_to_string(
            CURRENT_PATH.join(format!("src/tests/jobs/state_update_job/test_data/{}/{}", block_number, X_0_FILE_NAME)),
        )
        .expect("Failed to read the x_0 txt file");

        let blob_data_vec = vec![hex_string_to_u8_vec(&blob_data).unwrap()];
        let blob_serialized = bincode::serialize(&blob_data_vec).unwrap();
        storage_client
            .expect_get_data()
            .with(eq(blob_data_key))
            .returning(move |_| Ok(Bytes::from(blob_serialized.clone())));
        storage_client.expect_get_data().with(eq(x_0_key)).returning(move |_| Ok(Bytes::from(x_0.clone())));

        let config = init_config(None, None, None, None, None, None, Some(storage_client)).await;
        config_force_init(config).await;

        // testing the data in transaction :
        // https://etherscan.io/tx/0x6b9fc547764a5d6e4451b5236b92e74c70800250f00fc1974fc0a75a459dc12e
        let blob_data = fetch_blob_data_for_block(block_number).await.unwrap();
        let x_0_value = fetch_x_0_value_from_os_output(block_number).await.unwrap();

        let (_, kzg_proof) = build_kzg_proof(blob_data[0].clone(), x_0_value).await.unwrap();
        let proof = kzg_proof.to_bytes();
        let original_proof_from_l1 = Bytes48::from_hex(
            "a168b317e7c44691ee1932bd12fc6ac22182277e8fc5cd4cd21adc0831c33b1359aa5171bba529c69dcfe6224b220f8f",
        )
        .unwrap();

        assert_eq!(proof, original_proof_from_l1);
    }
}
