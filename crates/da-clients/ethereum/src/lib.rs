#![allow(missing_docs)]
#![allow(clippy::missing_docs_in_private_items)]
use std::str::FromStr;

use alloy::network::Ethereum;
use alloy::providers::{ProviderBuilder, RootProvider};
use alloy::rpc::client::RpcClient;
use alloy::transports::http::Http;
use async_trait::async_trait;
use color_eyre::Result;
use config::EthereumDaConfig;
use da_client_interface::{DaClient, DaVerificationStatus};
use mockall::automock;
use mockall::predicate::*;
use reqwest::Client;
use url::Url;
pub mod config;
pub struct EthereumDaClient {
    #[allow(dead_code)]
    provider: RootProvider<Ethereum, Http<Client>>,
}

#[automock]
#[async_trait]
impl DaClient for EthereumDaClient {
    async fn publish_state_diff(&self, _state_diff: Vec<Vec<u8>>, _to: &[u8; 32]) -> Result<String> {
        // Here in case of ethereum we are not publishing the state diff because we are doing it all together in update_state job.
        // So we don't need to send the blob here.
        Ok("NA".to_string())
    }

    async fn verify_inclusion(&self, _external_id: &str) -> Result<DaVerificationStatus> {
        Ok(DaVerificationStatus::Verified)
    }

    async fn max_blob_per_txn(&self) -> u64 {
        6
    }

    async fn max_bytes_per_blob(&self) -> u64 {
        131072
    }
}

impl From<EthereumDaConfig> for EthereumDaClient {
    fn from(config: EthereumDaConfig) -> Self {
        let client =
            RpcClient::new_http(Url::from_str(config.rpc_url.as_str()).expect("Failed to parse ETHEREUM_RPC_URL"));
        let provider = ProviderBuilder::<_, Ethereum>::new().on_client(client);
        EthereumDaClient { provider }
    }
}

#[cfg(test)]
mod tests {
    use alloy::eips::eip4844::BYTES_PER_BLOB;
    use alloy::primitives::FixedBytes;
    use c_kzg::{Blob, KzgCommitment, KzgProof, KzgSettings};
    use std::fs::File;
    use std::io::{self, BufRead};
    use std::path::Path;

    use super::*;

    async fn prepare_sidecar(
        state_diff: &[Vec<u8>],
        trusted_setup: &KzgSettings,
    ) -> Result<(Vec<FixedBytes<131072>>, Vec<FixedBytes<48>>, Vec<FixedBytes<48>>)> {
        let mut sidecar_blobs = vec![];
        let mut sidecar_commitments = vec![];
        let mut sidecar_proofs = vec![];

        for blob_data in state_diff {
            let mut fixed_size_blob: [u8; BYTES_PER_BLOB] = [0; BYTES_PER_BLOB];
            fixed_size_blob.copy_from_slice(blob_data.as_slice());

            let blob = Blob::new(fixed_size_blob);

            let commitment = KzgCommitment::blob_to_kzg_commitment(&blob, trusted_setup)?;
            let proof = KzgProof::compute_blob_kzg_proof(&blob, &commitment.to_bytes(), trusted_setup)?;

            sidecar_blobs.push(FixedBytes::new(fixed_size_blob));
            sidecar_commitments.push(FixedBytes::new(commitment.to_bytes().into_inner()));
            sidecar_proofs.push(FixedBytes::new(proof.to_bytes().into_inner()));
        }

        Ok((sidecar_blobs, sidecar_commitments, sidecar_proofs))
    }

    #[tokio::test]
    async fn test_kzg() {
        let trusted_setup = KzgSettings::load_trusted_setup_file(Path::new("./trusted_setup.txt"))
            .expect("Error loading trusted setup file");

        // hex of the blob data from the block 630872 of L2
        // https://voyager.online/block/0x3333f2f6b32776ac031e7ed373858c656d6d1040e47b73c94e762e6ed4cedf3 (L2)
        // https://etherscan.io/tx/0x6b9fc547764a5d6e4451b5236b92e74c70800250f00fc1974fc0a75a459dc12e (L1)
        let file_path = "./test_utils/hex_block_630872.txt";

        // open the file and store the data as a single string
        let file = File::open(file_path).expect("Unable to load the file for hex");
        let reader = io::BufReader::new(file);
        let mut data = String::new();
        for line in reader.lines().map_while(Result::ok) {
            data.push_str(&line);
        }

        // create vec<u8> from the hex string
        let data_v8 = hex_string_to_u8_vec(&data).expect("error creating hex string from data");

        // creation of sidecar
        let (_sidecar_blobs, sidecar_commitments, sidecar_proofs) =
            prepare_sidecar(&[data_v8], &trusted_setup).await.expect("Error creating the sidecar blobs");

        // blob commitment from L1
        let commitment_vector = hex_string_to_u8_vec(
            "adece1d251a1671e134d57204ef111308818dacf97d2372b28b53f947682de715fd0a75f57496124ec97609a52e8ca52",
        )
        .expect("Error creating the vector of u8 from commitment");
        let commitment_fixedbytes: FixedBytes<48> = FixedBytes::from_slice(&commitment_vector);

        // blob proof from L1
        let proof_vector = hex_string_to_u8_vec(
            "999371598a3807abe20956a5754f9894f2d8fe2a0f8fd49bb13f294282121be1118627f2f9fe4e2ea0b9760addd41a0c",
        )
        .expect("Error creating the vector of u8 from proof");
        let proog_fixedbytes: FixedBytes<48> = FixedBytes::from_slice(&proof_vector);

        // blob commitment and proof should be equal to the blob created by prepare_sidecar
        assert_eq!(sidecar_commitments[0], commitment_fixedbytes);
        assert_eq!(sidecar_proofs[0], proog_fixedbytes);
    }

    fn hex_string_to_u8_vec(hex_str: &str) -> Result<Vec<u8>, String> {
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
        println!("length of vec<u8>: {}", result.len());
        Ok(result)
    }

    fn _vec_u8_to_hex_string(data: &[u8]) -> String {
        let hex_chars: Vec<String> = data.iter().map(|byte| format!("{:02X}", byte)).collect();
        hex_chars.join("")
    }
}
