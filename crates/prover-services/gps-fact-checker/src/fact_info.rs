//! Fact info structure and helpers.
//!
//! Port of https://github.com/starkware-libs/cairo-lang/blob/master/src/starkware/cairo/bootloaders/generate_fact.py

use alloy::primitives::{keccak256, B256};
use aws_config::meta::region::RegionProviderChain;
use aws_config::SdkConfig;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::Client;
use cairo_vm::program_hash::compute_program_hash_chain;
use cairo_vm::types::builtin_name::BuiltinName;
use cairo_vm::types::relocatable::MaybeRelocatable;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
use cairo_vm::Felt252;
use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;
use utils::env_utils::get_env_var_or_panic;
use utils::settings::env::EnvSettingsProvider;
use utils::settings::Settings;

use super::error::FactCheckerError;
use super::fact_node::generate_merkle_root;
use super::fact_topology::{get_fact_topology, FactTopology};

/// Default bootloader program version.
///
/// https://github.com/starkware-libs/cairo-lang/blob/efa9648f57568aad8f8a13fbf027d2de7c63c2c0/src/starkware/cairo/bootloaders/hash_program.py#L11
pub const BOOTLOADER_VERSION: usize = 0;

pub struct FactInfo {
    pub program_output: Vec<Felt252>,
    pub fact_topology: FactTopology,
    pub fact: B256,
}

#[derive(Serialize, Deserialize)]
struct ProgramData(Vec<[u8; 32]>);

pub async fn get_fact_info(
    cairo_pie: &CairoPie,
    program_hash: Option<FieldElement>,
) -> Result<FactInfo, FactCheckerError> {
    let program_output = get_program_output(cairo_pie)?;

    let mut program_output_u8_vec = Vec::new();
    for i in &program_output {
        program_output_u8_vec.push(i.to_bytes_be());
    }

    // TODO :
    // Remove the logic for storing here find something more generic
    store_program_output(program_output_u8_vec).await;

    let fact_topology = get_fact_topology(cairo_pie, program_output.len())?;
    let program_hash = match program_hash {
        Some(hash) => hash,
        None => compute_program_hash_chain(&cairo_pie.metadata.program, BOOTLOADER_VERSION)?,
    };
    let output_root = generate_merkle_root(&program_output, &fact_topology)?;
    let fact = keccak256([program_hash.to_bytes_be(), *output_root.node_hash].concat());
    Ok(FactInfo { program_output, fact_topology, fact })
}

pub fn get_program_output(cairo_pie: &CairoPie) -> Result<Vec<Felt252>, FactCheckerError> {
    let segment_info = cairo_pie
        .metadata
        .builtin_segments
        .get(&BuiltinName::output)
        .ok_or(FactCheckerError::OutputBuiltinNoSegmentInfo)?;

    let mut output = vec![Felt252::from(0); segment_info.size];
    let mut insertion_count = 0;
    let cairo_program_memory = &cairo_pie.memory.0;

    for ((index, offset), value) in cairo_program_memory.iter() {
        if *index == segment_info.index as usize {
            match value {
                MaybeRelocatable::Int(felt) => {
                    output[*offset] = *felt;
                    insertion_count += 1;
                }
                MaybeRelocatable::RelocatableValue(_) => {
                    return Err(FactCheckerError::OutputSegmentUnexpectedRelocatable(*offset));
                }
            }
        }
    }

    if insertion_count != segment_info.size {
        return Err(FactCheckerError::InvalidSegment);
    }

    Ok(output)
}

/// This is a duplicate code not able to import it from orchestrator
/// as it will become a cyclic dependency.
///
/// To build a `SdkConfig` for AWS provider.
pub async fn aws_config(settings_provider: &impl Settings) -> SdkConfig {
    let region = settings_provider.get_settings_or_panic("AWS_REGION");
    let region_provider = RegionProviderChain::first_try(Region::new(region)).or_default_provider();
    let credentials = Credentials::from_keys(
        settings_provider.get_settings_or_panic("AWS_ACCESS_KEY_ID"),
        settings_provider.get_settings_or_panic("AWS_SECRET_ACCESS_KEY"),
        None,
    );
    aws_config::from_env()
        .credentials_provider(credentials)
        .region(region_provider)
        .endpoint_url(settings_provider.get_settings_or_panic("AWS_ENDPOINT_URL"))
        .load()
        .await
}

// TODO : generalise this (or use from config)
async fn store_program_output(program_output: Vec<[u8; 32]>) {
    let aws_config = aws_config(&EnvSettingsProvider {}).await;
    // Building AWS S3 config
    let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&aws_config);
    // this is necessary for it to work with localstack in test cases
    s3_config_builder.set_force_path_style(Some(true));
    let client = Client::from_conf(s3_config_builder.build());

    let encoded_data = bincode::serialize(&program_output).unwrap();

    client
        .put_object()
        .bucket(get_env_var_or_panic("AWS_S3_BUCKET_NAME"))
        .key(get_env_var_or_panic("L2_BLOCK_TO_TEST") + "/program_output.txt")
        .body(encoded_data.into())
        .send()
        .await
        .unwrap();
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use cairo_vm::vm::runners::cairo_pie::CairoPie;

    use super::get_fact_info;

    #[tokio::test]
    async fn test_fact_info() {
        dotenvy::from_filename("../.env.test").expect("Failed to load the .env.test file");
        // Generated using the get_fact.py script
        let expected_fact = "0xca15503f02f8406b599cb220879e842394f5cf2cef753f3ee430647b5981b782";
        let cairo_pie_path: PathBuf =
            [env!("CARGO_MANIFEST_DIR"), "tests", "artifacts", "fibonacci.zip"].iter().collect();
        let cairo_pie = CairoPie::read_zip_file(&cairo_pie_path).unwrap();
        let fact_info = get_fact_info(&cairo_pie, None).await.unwrap();
        assert_eq!(expected_fact, fact_info.fact.to_string());
    }
}
