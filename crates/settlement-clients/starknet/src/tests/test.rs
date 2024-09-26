use super::{MadaraCmd, MadaraCmdBuilder};
use crate::StarknetSettlementClient;
use rstest::{fixture, rstest};
use settlement_client_interface::SettlementClient;
use starknet::{
    accounts::{Account, ExecutionEncoding, SingleOwnerAccount},
    contract::ContractFactory,
    core::types::{
        contract::{CompiledClass, SierraClass},
        BlockId, BlockTag, DeclareTransactionResult, Felt,
    },
    macros::felt,
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, Url},
    signers::{LocalWallet, SigningKey},
};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use utils::settings::env::EnvSettingsProvider;
use utils::settings::Settings;

#[allow(unused)]
pub async fn spin_up_madara() -> MadaraCmd {
    env::set_current_dir(PathBuf::from("/Users/bytezorvin/work/karnot/madara/"))
        .expect("Failed to set working directory");
    let output = std::process::Command::new("cargo")
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .expect("Failed to execute command");

    let cargo_toml_path = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    let project_root = PathBuf::from(cargo_toml_path.trim()).parent().unwrap().to_path_buf();

    env::set_current_dir(&project_root).expect("Failed to set working directory");

    let _ = env_logger::builder().is_test(true).try_init();
    let mut node = MadaraCmdBuilder::new()
        .args([
            "--network",
            "devnet",
            "--no-sync-polling",
            "--n-blocks-to-sync",
            "20",
            "--authority",
            "--devnet",
            "--preset=test",
            "--no-l1-sync",
        ])
        .run();
    node.wait_for_ready().await;
    node
}

// #[fixture]
// async fn setup() -> (SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>, MadaraCmd) {
#[fixture]
async fn setup() -> SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet> {
    // let madara_process = spin_up_madara().await;
    // println!("RPC url {:?}", madara_process.rpc_url);
    dotenvy::from_filename_override(".env").expect("Failed to load the .env file");

    let env_settings = EnvSettingsProvider::default();
    let rpc_url = Url::parse(&env_settings.get_settings_or_panic("STARKNET_RPC_URL")).unwrap();
    println!("RPC url {:?}", rpc_url);
    // let endpoint = madara_process.rpc_url.join("/health").unwrap();
    // let endpoint = rpc_url.join("/health").unwrap();
    // let response = reqwest::get(endpoint.clone()).await.expect("Failed to connect to Provider");
    // assert!(response.status().is_success(), "Failed to connect to Provider");

    // let provider = JsonRpcClient::new(HttpTransport::new(Url::parse("http://localhost:9944").unwrap()));
    let provider: JsonRpcClient<HttpTransport> = JsonRpcClient::new(HttpTransport::new(rpc_url));
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        Felt::from_hex(&env_settings.get_settings_or_panic("STARKNET_PRIVATE_KEY")).expect("Invalid private key"),
    ));
    let address = Felt::from_hex(&env_settings.get_settings_or_panic("STARKNET_PUBLIC_KEY")).unwrap();

    let chain_id = provider.chain_id().await.unwrap();
    let mut account = SingleOwnerAccount::new(provider, signer, address, chain_id, ExecutionEncoding::New);

    // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
    // block. Optionally change the target block to pending with the following line:
    account.set_block_id(BlockId::Tag(BlockTag::Pending));
    // (account, madara_process)
    account
}

#[rstest]
#[tokio::test]
// async fn test_deployment(#[future] setup: (SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>, MadaraCmd)) {
async fn test_deployment(#[future] setup: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>) {
    // let (account, madara_process) = setup.await;
    let account = setup.await;

    // println!("the db being used is {:?}", madara_process.tempdir);
    let account = Arc::new(account);

    // NOTE: you will need to declare this class first
    let sierra_class: SierraClass = serde_json::from_reader(
        std::fs::File::open("/Users/bytezorvin/work/karnot/madara-orchestrator/crates/settlement-clients/starknet/src/tests/mock_contracts/target/dev/mock_contracts_Piltover.contract_class.json").unwrap(),
    )
    .unwrap();

    let compiled_class: CompiledClass = serde_json::from_reader(
        std::fs::File::open("/Users/bytezorvin/work/karnot/madara-orchestrator/crates/settlement-clients/starknet/src/tests/mock_contracts/target/dev/mock_contracts_Piltover.compiled_contract_class.json").unwrap(),
    )
    .unwrap();

    let flattened_class = sierra_class.clone().flatten().unwrap();
    let compiled_class_hash = compiled_class.class_hash().unwrap();
    let class_hash = flattened_class.class_hash();
    account.declare_v2(Arc::new(flattened_class), compiled_class_hash).send().await.unwrap();

    // This done since madara currently does not increment nonce for pending transactions
    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

    let contract_factory = ContractFactory::new(class_hash, account);
    contract_factory.deploy_v1(vec![], felt!("1122"), false).send().await.expect("Unable to deploy contract");
}

#[rstest]
#[tokio::test]
async fn test_settle(#[future] setup: SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>) {
    dotenvy::from_filename(".env.test").expect("Failed to load the .env file");
    // let (account, madara_process) = setup.await;
    let account = setup.await;

    // println!("the db being used is {:?}", madara_process.tempdir);
    let account = Arc::new(account);

    // NOTE: you will need to declare this class first
    let sierra_class: SierraClass = serde_json::from_reader(
        std::fs::File::open("/Users/bytezorvin/work/karnot/orchestrator/crates/settlement-clients/starknet/src/tests/mock_contracts/target/dev/mock_contracts_Piltover.contract_class.json").unwrap(),
    )
    .unwrap();

    let compiled_class: CompiledClass = serde_json::from_reader(
        std::fs::File::open("/Users/bytezorvin/work/karnot/orchestrator/crates/settlement-clients/starknet/src/tests/mock_contracts/target/dev/mock_contracts_Piltover.compiled_contract_class.json").unwrap(),
    )
    .unwrap();

    // let flattened_class = sierra_class.clone().flatten().unwrap();
    // let compiled_class_hash = compiled_class.class_hash().unwrap();
    // let DeclareTransactionResult { transaction_hash: _, class_hash } =
    //     account.declare_v2(Arc::new(flattened_class.clone()), compiled_class_hash).send().await.unwrap();
    // assert!(flattened_class.class_hash() == class_hash, "Class hash declared is not same");

    // This is done, since currently madara does not increment nonce for pending transactions
    // tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

    // let contract_factory = ContractFactory::new(flattened_class.class_hash(), account);
    // let deploy_v1 = contract_factory.deploy_v1(vec![], felt!("1122"), false);
    // let deployed_address = deploy_v1.deployed_address();

    let deployed_address = "0x067b25d85c42bae8f3fe833ab5ff97368e1c39019d34f02906e1cc6280f80e50";
    env::set_var("STARKNET_CAIRO_CORE_CONTRACT_ADDRESS", deployed_address);
    // env::set_var("STARKNET_CAIRO_CORE_CONTRACT_ADDRESS", deployed_address.to_hex_string());
    // deploy_v1.send().await.expect("Unable to deploy contract");

    // This is done, since currently madara does not increment nonce for pending transactions
    tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;

    let env_settings = EnvSettingsProvider {};
    let settlement_client = StarknetSettlementClient::new_with_settings(&env_settings).await;
    let onchain_data_hash = [1; 32];
    let mut program_output = Vec::with_capacity(32);
    program_output.fill(onchain_data_hash);
    settlement_client.update_state_calldata(program_output, onchain_data_hash, 1).await.unwrap();
}
