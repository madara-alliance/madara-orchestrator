use alloy::{node_bindings::Anvil, sol};
use std::io::BufRead;
use std::{
    fs::{self, File},
    io::BufReader,
    str::FromStr,
};
use url::Url;

use alloy::providers::{ext::AnvilApi, ProviderBuilder};
use alloy::{primitives::U256, providers::Provider};
use alloy_primitives::Address;
use color_eyre::eyre::eyre;
use rstest::*;

use settlement_client_interface::SettlementClient;
use utils::settings::default::DefaultSettingsProvider;

use crate::EthereumSettlementClient;

fn hex_string_to_u8_vec(hex_str: &str) -> color_eyre::Result<Vec<u8>> {
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

// Codegen from ABI file to interact with the contract.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    STARKNET_CORE_CONTRACT,
    "src/test_data/ABI/starknet_core_contract.json"
);

// TODO: betterment of file routes

#[rstest]
#[tokio::test]
#[case::basic(20468828)]
async fn update_state_blob_works(#[case] block_no: u64) {
    // Load ENV vars
    dotenvy::from_filename("../.env.test").expect("Could not load .env.test file.");
    let current_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();

    // Setup Anvil
    let _anvil = Anvil::new()
        .port(3000_u16)
        .fork("https://eth.llamarpc.com")
        .fork_block_number(block_no - 1)
        .try_spawn()
        .expect("Could not spawn Anvil.");

    // Setup Provider
    let provider =
        ProviderBuilder::new().on_http(Url::from_str("http://localhost:3000").expect("Could not create provider."));

    // Setup EthereumSettlementClient
    let settings_provider: DefaultSettingsProvider = DefaultSettingsProvider {};
    let ethereum_settlement_client = EthereumSettlementClient::with_test_settings(&settings_provider, provider.clone());

    // Setup operator account impersonation
    provider
        .anvil_impersonate_account(
            Address::from_str("0x2C169DFe5fBbA12957Bdd0Ba47d9CEDbFE260CA7").expect("Could not impersonate account."),
        )
        .await
        .expect("sdcjb");

    // Create a contract instance.
    let contract = STARKNET_CORE_CONTRACT::new(
        Address::from_str("0xc662c410c0ecf747543f5ba90660f6abebd9c8c4").expect("sd"),
        provider.clone(),
    );

    // Call the contract, retrieve the current stateBlockNumber.
    let prev_block_number = contract.stateBlockNumber().call().await.unwrap();

    // Program Output
    let program_output_file_path =
        format!("{}{}{}{}", current_path.clone(), "/src/test_data/program_output/", block_no, ".txt");

    let mut program_output: Vec<[u8; 32]> = Vec::new();
    {
        let file = File::open(program_output_file_path).expect("Failed to read program output file");
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.expect("can't read line");
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let line_u8_32: [u8; 32] = U256::from_str(trimmed).expect("unable to convert line").to_le_bytes();
                program_output.push(line_u8_32);
            }
        }
    }

    // Blob Data
    let blob_data_file_path = format!("{}{}{}{}", current_path.clone(), "/src/test_data/blob_data/", block_no, ".txt");
    println!("{}", blob_data_file_path);
    let blob_data = fs::read_to_string(blob_data_file_path).expect("Failed to read the blob data txt file");
    let blob_data_vec = vec![hex_string_to_u8_vec(&blob_data).unwrap()];

    // Calling update_state_with_blobs
    let update_state_result = ethereum_settlement_client
        .update_state_with_blobs(program_output, blob_data_vec)
        .await
        .expect("Could not go through update_state_with_blobs.");

    // Asserting, Expected to receive transaction hash.
    assert!(!update_state_result.is_empty(), "No transaction Hash received.");

    // Call the contract, retrieve the latest stateBlockNumber.
    let latest_block_number = contract.stateBlockNumber().call().await.unwrap();

    println!("PREVIOUS BLOCK NUMBER {}", prev_block_number._0);
    println!("CURRENT BLOCK HASH {}", latest_block_number._0);

    assert_eq!(prev_block_number._0.as_u32() + 1, latest_block_number._0.as_u32());
}
