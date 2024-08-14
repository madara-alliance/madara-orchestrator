use alloy::primitives::U256;
use alloy::providers::{ext::AnvilApi, ProviderBuilder};
use alloy::{node_bindings::Anvil, sol};
use alloy_primitives::Address;
use color_eyre::eyre::eyre;
use rstest::*;
use settlement_client_interface::SettlementVerificationStatus;
use std::env;
use std::io::BufRead;
use std::path::PathBuf;
use std::time::Duration;
use std::{
    fs::{self, File},
    io::BufReader,
    str::FromStr,
};
use tokio::time::sleep;
use utils::env_utils::get_env_var_or_panic;

use settlement_client_interface::SettlementClient;
use utils::settings::default::DefaultSettingsProvider;

use crate::conversion::to_padded_hex;
use crate::EthereumSettlementClient;
use alloy::providers::Provider;
use alloy_primitives::FixedBytes;

// Using the Pipe trait to write chained operations easier
trait Pipe: Sized {
    fn pipe<T, F: FnOnce(Self) -> T>(self, f: F) -> T {
        f(self)
    }
}

// Implement Pipe for all types
impl<S> Pipe for S {}

// TODO: betterment of file routes

use lazy_static::lazy_static;

lazy_static! {
    static ref ENV_FILE_PATH: PathBuf = PathBuf::from(".env.test");
    static ref CURRENT_PATH: String = env::current_dir()
        .expect("Failed to get current directory")
        .to_str()
        .expect("Path contains invalid Unicode")
        .to_string();
    static ref PORT: u16 = 3000_u16;
    static ref ETH_RPC: String = "https://eth.llamarpc.com".to_string();
    static ref SHOULD_IMPERSONATE_ACCOUNT: bool = get_env_var_or_panic("TEST_IMPERSONATE_OPERATOR") == *"1";
    static ref TEST_DUMMY_CONTRACT_ADDRESS: String = get_env_var_or_panic("TEST_DUMMY_CONTRACT_ADDRESS");
    static ref STARKNET_OPERATOR_ADDRESS: Address =
        Address::from_str("0x2C169DFe5fBbA12957Bdd0Ba47d9CEDbFE260CA7").expect("Could not impersonate account.");
    static ref STARKNET_CORE_CONTRACT_ADDRESS: Address =
        Address::from_str("0xc662c410c0ecf747543f5ba90660f6abebd9c8c4").expect("Could not impersonate account.");
}

pub struct TestFixture {
    pub ethereum_settlement_client: EthereumSettlementClient,
    pub provider: alloy::providers::RootProvider<alloy::transports::http::Http<reqwest::Client>>,
}

fn ethereum_test_fixture(block_no: u64) -> TestFixture {
    // Load ENV vars
    dotenvy::from_filename(&*ENV_FILE_PATH).expect("Could not load .env.test file.");

    // Setup Anvil
    let anvil = Anvil::new()
        .port(*PORT)
        .fork(&*ETH_RPC)
        .fork_block_number(block_no - 1)
        .try_spawn()
        .expect("Could not spawn Anvil.");

    // Setup Provider
    let provider = ProviderBuilder::new().on_http(anvil.endpoint_url());

    // Setup EthereumSettlementClient
    let settings_provider: DefaultSettingsProvider = DefaultSettingsProvider {};
    let ethereum_settlement_client = EthereumSettlementClient::with_test_settings(&settings_provider, provider.clone());

    TestFixture { ethereum_settlement_client, provider }
}

#[rstest]
#[tokio::test]
#[case::basic(20468828)]
async fn update_state_blob_with_dummy_contract_works(#[case] block_no: u64) {
    env::set_var("TEST_IMPERSONATE_OPERATOR", "0");

    let TestFixture { ethereum_settlement_client, provider } = ethereum_test_fixture(block_no);

    // Deploying a dummy contract
    let contract = DummyCoreContract::deploy(&provider).await.expect("Unable to deploy address");
    assert_eq!(
        contract.address().to_string(),
        *TEST_DUMMY_CONTRACT_ADDRESS,
        "Dummy Contract got deployed on unexpected address"
    );

    // Getting latest nonce after deployment
    let nonce = ethereum_settlement_client.get_nonce().await.expect("Unable to fetch nonce");

    // generating program output and blob vector
    let program_output = get_program_output(block_no);
    let blob_data_vec = get_blob_data(block_no);

    // Calling update_state_with_blobs
    let update_state_result = ethereum_settlement_client
        .update_state_with_blobs(program_output, blob_data_vec, nonce)
        .await
        .expect("Could not go through update_state_with_blobs.");

    // Asserting, Expected to receive transaction hash.
    assert!(!update_state_result.is_empty(), "No transaction Hash received.");

    let txn = provider
        .get_transaction_by_hash(FixedBytes::from_str(update_state_result.as_str()).expect("Unable to convert txn"))
        .await
        .expect("did not get txn from hash")
        .unwrap();

    assert_eq!(txn.hash.to_string(), update_state_result.to_string());
    assert!(txn.signature.is_some());
    assert_eq!(txn.to.unwrap().to_string(), *TEST_DUMMY_CONTRACT_ADDRESS);

    // Testing verify_tx_inclusion
    sleep(Duration::from_secs(2)).await;
    ethereum_settlement_client
        .wait_for_tx_finality(update_state_result.as_str())
        .await
        .expect("Could not wait for txn finality.");
    let verified_inclusion = ethereum_settlement_client
        .verify_tx_inclusion(update_state_result.as_str())
        .await
        .expect("Could not verify inclusion.");
    assert_eq!(verified_inclusion, SettlementVerificationStatus::Verified);
}

#[rstest]
#[tokio::test]
#[case::basic(20468828)]
async fn update_state_blob_with_impersonation_works(#[case] block_no: u64) {
    let TestFixture { ethereum_settlement_client, provider } = ethereum_test_fixture(block_no);

    provider.anvil_impersonate_account(*STARKNET_OPERATOR_ADDRESS).await.expect("Unable to impersonate account.");

    let nonce = ethereum_settlement_client.get_nonce().await.expect("Unable to fetch nonce");

    // Create a contract instance.
    let contract = STARKNET_CORE_CONTRACT::new(*STARKNET_CORE_CONTRACT_ADDRESS, provider.clone());

    // Call the contract, retrieve the current stateBlockNumber.
    let prev_block_number = contract.stateBlockNumber().call().await.unwrap();

    // generating program output and blob vector
    let program_output = get_program_output(block_no);
    let blob_data_vec = get_blob_data(block_no);

    // Calling update_state_with_blobs
    let update_state_result = ethereum_settlement_client
        .update_state_with_blobs(program_output, blob_data_vec, nonce)
        .await
        .expect("Could not go through update_state_with_blobs.");

    // Asserting, Expected to receive transaction hash.
    assert!(!update_state_result.is_empty(), "No transaction Hash received.");

    // Call the contract, retrieve the latest stateBlockNumber.
    let latest_block_number = contract.stateBlockNumber().call().await.unwrap();

    println!("PREVIOUS BLOCK NUMBER {}", prev_block_number._0);
    println!("CURRENT BLOCK HASH {}", latest_block_number._0);
    assert_eq!(prev_block_number._0.as_u32() + 1, latest_block_number._0.as_u32());

    // Testing verify_tx_inclusion
    sleep(Duration::from_secs(2)).await;
    ethereum_settlement_client
        .wait_for_tx_finality(update_state_result.as_str())
        .await
        .expect("Could not wait for txn finality.");
    let verified_inclusion = ethereum_settlement_client
        .verify_tx_inclusion(update_state_result.as_str())
        .await
        .expect("Could not verify inclusion.");
    assert_eq!(verified_inclusion, SettlementVerificationStatus::Verified);
}

#[rstest]
#[tokio::test]
#[case::typical(20468828, 666039)]
async fn get_last_settled_block_typical_works(#[case] block_no: u64, #[case] expected: u64) {
    let TestFixture { ethereum_settlement_client, provider: _ } = ethereum_test_fixture(block_no);

    let result = ethereum_settlement_client.get_last_settled_block().await.expect("Could not get last settled block.");
    assert_eq!(expected, result);
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

// SOLIDITY FUNCTIONS NEEDED
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    STARKNET_CORE_CONTRACT,
    "src/test_data/ABI/starknet_core_contract.json"
);

sol! {
    #[allow(missing_docs)]
    #[sol(rpc, bytecode="6080604052348015600e575f80fd5b506101c18061001c5f395ff3fe608060405234801561000f575f80fd5b5060043610610029575f3560e01c8063b72d42a11461002d575b5f80fd5b6100476004803603810190610042919061010d565b610049565b005b50505050565b5f80fd5b5f80fd5b5f80fd5b5f80fd5b5f80fd5b5f8083601f84011261007857610077610057565b5b8235905067ffffffffffffffff8111156100955761009461005b565b5b6020830191508360208202830111156100b1576100b061005f565b5b9250929050565b5f8083601f8401126100cd576100cc610057565b5b8235905067ffffffffffffffff8111156100ea576100e961005b565b5b6020830191508360018202830111156101065761010561005f565b5b9250929050565b5f805f80604085870312156101255761012461004f565b5b5f85013567ffffffffffffffff81111561014257610141610053565b5b61014e87828801610063565b9450945050602085013567ffffffffffffffff81111561017157610170610053565b5b61017d878288016100b8565b92509250509295919450925056fea2646970667358221220fa7488d5a2a9e6c21e6f46145a831b0f04fdebab83868dc2b996c17f8cba4d8064736f6c634300081a0033")]
    contract DummyCoreContract {
        function updateStateKzgDA(uint256[] calldata programOutput, bytes calldata kzgProof)  external {
        }
    }
}

// UTILITY FUNCTIONS NEEDED

fn get_program_output(block_no: u64) -> Vec<[u8; 32]> {
    // Program Output
    let program_output_file_path =
        format!("{}{}{}{}", *CURRENT_PATH, "/src/test_data/program_output/", block_no, ".txt");

    let mut program_output: Vec<[u8; 32]> = Vec::new();
    let file = File::open(program_output_file_path).expect("Failed to read program output file");
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line.expect("can't read line");
        let trimmed = line.trim();
        assert!(!trimmed.is_empty());

        let result: [u8; 32] = U256::from_str(trimmed)
            .expect("Unable to convert line")
            .to_be_bytes_vec()
            .as_slice()
            .pipe(to_padded_hex)
            .pipe(|hex| hex_string_to_u8_vec(&hex).expect("unable to convert"))
            .try_into()
            .expect("Vector length must be 32");

        program_output.push(result)
    }
    program_output
}

fn get_blob_data(block_no: u64) -> Vec<Vec<u8>> {
    // Blob Data
    let blob_data_file_path = format!("{}{}{}{}", *CURRENT_PATH, "/src/test_data/blob_data/", block_no, ".txt");
    let blob_data = fs::read_to_string(blob_data_file_path).expect("Failed to read the blob data txt file");
    let blob_data_vec = vec![hex_string_to_u8_vec(&blob_data).unwrap()];
    blob_data_vec
}

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
