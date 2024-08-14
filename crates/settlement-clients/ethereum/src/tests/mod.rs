use alloy::{node_bindings::Anvil, sol};
use utils::env_utils::get_env_var_or_panic;
use std::io::BufRead;
use std::{
    fs::{self, File},
    io::BufReader,
    str::FromStr,
};
use url::Url;

use alloy::primitives::U256;
use alloy::providers::{ext::AnvilApi, ProviderBuilder};
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



// TODO: Checking send_transaction
// Create a dummy contract and deploy on anvil with same methodId as core contract
// Make an env variable that will tell if we are testing impersonation or not
// Check against the env variable, if we are not impersonation then we should talk to the dummy address

sol! {
    #[allow(missing_docs)]
    #[sol(rpc, bytecode="6080604052348015600e575f80fd5b506101c18061001c5f395ff3fe608060405234801561000f575f80fd5b5060043610610029575f3560e01c8063b72d42a11461002d575b5f80fd5b6100476004803603810190610042919061010d565b610049565b005b50505050565b5f80fd5b5f80fd5b5f80fd5b5f80fd5b5f80fd5b5f8083601f84011261007857610077610057565b5b8235905067ffffffffffffffff8111156100955761009461005b565b5b6020830191508360208202830111156100b1576100b061005f565b5b9250929050565b5f8083601f8401126100cd576100cc610057565b5b8235905067ffffffffffffffff8111156100ea576100e961005b565b5b6020830191508360018202830111156101065761010561005f565b5b9250929050565b5f805f80604085870312156101255761012461004f565b5b5f85013567ffffffffffffffff81111561014257610141610053565b5b61014e87828801610063565b9450945050602085013567ffffffffffffffff81111561017157610170610053565b5b61017d878288016100b8565b92509250509295919450925056fea2646970667358221220fa7488d5a2a9e6c21e6f46145a831b0f04fdebab83868dc2b996c17f8cba4d8064736f6c634300081a0033")]
    contract DummyCoreContract {
        function updateStateKzgDA(uint256[] calldata programOutput, bytes calldata kzgProof)  external {
        }
    }
}

#[rstest]
#[tokio::test]
#[case::basic(20468828)]
async fn update_state_blob_works(#[case] block_no: u64) {
    use std::time::Duration;

    use alloy::providers::Provider;
    use alloy_primitives::FixedBytes;
    use tokio::time::sleep;

    // Load ENV vars
    use crate::conversion::to_padded_hex;
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

    let impersonate_acount = get_env_var_or_panic("TEST_IMPERSONATE_OPERATOR");

    if impersonate_acount == "0".to_string(){
        let contract = DummyCoreContract::deploy(&provider).await.expect("Unable to deploy address");
        println!("Deployed contract at address: {}", contract.address());
    } else {
        provider
            .anvil_impersonate_account(
                Address::from_str("0x2C169DFe5fBbA12957Bdd0Ba47d9CEDbFE260CA7").expect("Could not impersonate account."),
            )
            .await
            .expect("Unable to impersonate account.");
    }

    let nonce = ethereum_settlement_client.get_nonce().await.expect("Unable to fetch nonce");

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
                let v_0 = U256::from_str(trimmed).expect("Unable to convert line").to_be_bytes_vec();
                let v_1 = v_0.as_slice();
                let v_2 = to_padded_hex(v_1);
                // let v_3 = v_2.replace("0x", "");
                let v_4 = hex_string_to_u8_vec(&v_2).expect("unable to convert");
                let v_5: [u8; 32] = v_4.try_into().expect("Vector length must be 32");
                program_output.push(v_5)
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
        .update_state_with_blobs(program_output, blob_data_vec, nonce)
        .await
        .expect("Could not go through update_state_with_blobs.");

    // Asserting, Expected to receive transaction hash.
    assert!(!update_state_result.is_empty(), "No transaction Hash received.");

    // Call the contract, retrieve the latest stateBlockNumber.
    let latest_block_number = contract.stateBlockNumber().call().await.unwrap();

    if impersonate_acount == "1".to_string() {
        println!("PREVIOUS BLOCK NUMBER {}", prev_block_number._0);
        println!("CURRENT BLOCK HASH {}", latest_block_number._0);
        assert_eq!(prev_block_number._0.as_u32() + 1, latest_block_number._0.as_u32());
    } else {
        sleep(Duration::from_secs(2)).await;
        let txn = provider.get_transaction_by_hash(FixedBytes::from_str(update_state_result.as_str()).expect("couln't convert"))
        .await
        .expect("did not get txn from hash");
        println!("{:?}", txn);
        sleep(Duration::from_secs(2)).await;

        assert!(!txn.is_none());

      
    }

}

#[rstest]
#[tokio::test]
#[case::basic(20468828)]
async fn creating_input_data_works(#[case] block_no: u64) {
    use c_kzg::Bytes32;

    use crate::conversion::{get_input_data_for_eip_4844, to_padded_hex};

    let current_path = std::env::current_dir().unwrap().to_str().unwrap().to_string();

    let program_output_file_path =
        format!("{}{}{}{}", current_path.clone(), "/src/test_data/program_output/", block_no, ".txt");

    let mut program_output: Vec<[u8; 32]> = Vec::new();
    let file = File::open(program_output_file_path).expect("Failed to read program output file");
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.expect("can't read line");
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let v_0 = U256::from_str(trimmed).expect("Unable to convert line").to_be_bytes_vec();
            let v_1 = v_0.as_slice();
            let v_2 = to_padded_hex(v_1);
            // let v_3 = v_2.replace("0x", "");
            let v_4 = hex_string_to_u8_vec(&v_2).expect("unable to convert");
            let v_5: [u8; 32] = v_4.try_into().expect("Vector length must be 32");
            program_output.push(v_5)
        }
    }

    let x_0_value_bytes32 = Bytes32::from(program_output[8]);

    // Blob Data
    let blob_data_file_path = format!("{}{}{}{}", current_path.clone(), "/src/test_data/blob_data/", block_no, ".txt");
    println!("{}", blob_data_file_path);
    let blob_data = fs::read_to_string(blob_data_file_path).expect("Failed to read the blob data txt file");
    let blob_data_vec = vec![hex_string_to_u8_vec(&blob_data).unwrap()];

    let kzg_proof = EthereumSettlementClient::build_proof(blob_data_vec, x_0_value_bytes32)
        .expect("Unable to build KZG proof for given params.")
        .to_owned();

    let input_bytes = get_input_data_for_eip_4844(program_output, kzg_proof).expect("unable to create input data");
    let expected = "0xb72d42a100000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000340000000000000000000000000000000000000000000000000000000000000001706ac7b2661801b4c0733da6ed1d2910b3b97259534ca95a63940932513111fba028bccc051eaae1b9a69b53e64a68021233b4dee2030aeda4be886324b3fbb3e00000000000000000000000000000000000000000000000000000000000a29b8070626a88de6a77855ecd683757207cdd18ba56553dca6c0c98ec523b827bee005ba2078240f1585f96424c2d1ee48211da3b3f9177bf2b9880b4fc91d59e9a2000000000000000000000000000000000000000000000000000000000000000100000000000000002b4e335bc41dc46c71f29928a5094a8c96a0c3536cabe53e0000000000000000810abb1929a0d45cdd62a20f9ccfd5807502334e7deb35d404c86d8b63a5741770fefca2f9b8efb7e663d89097edb3c60595b236f6e78e6f000000000000000000000000000000004a4b8a979fefc4d6b82e030fb082ca98000000000000000000000000000000004e8371c6774260e87b92447d4a2b0e170000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000a000000000000000000000000bf67f59d2988a46fbff7ed79a621778a3cd3985b0088eedbe2fe3918b69ccb411713b7fa72079d4eddf291103ccbe41e78a9615c0000000000000000000000000000000000000000000000000000000000194fe601b64b1b3b690b43b9b514fb81377518f4039cd3e4f4914d8a6bdf01d679fb1900000000000000000000000000000000000000000000000000000000000000050000000000000000000000007f39c581f595b53c5cb19bd0b3f8da6c935e2ca000000000000000000000000012ccc443d39da45e5f640b3e71f0c7502152dbac01d4988e248d342439aa025b302e1f07595f6a5c810dcce23e7379e48f05d4cf000000000000000000000000000000000000000000000007f189b5374ad2a00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000030ab015987628cffee3ef99b9768ef8ca12c6244525f0cd10310046eaa21291b5aca164d044c5b4ad7212c767b165ed5e300000000000000000000000000000000";
    assert_eq!(input_bytes, expected);
}
