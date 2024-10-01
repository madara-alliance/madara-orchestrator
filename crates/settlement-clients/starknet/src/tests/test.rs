use super::setup::{MadaraCmd, MadaraCmdBuilder};
use crate::StarknetSettlementClient;
use rstest::{fixture, rstest};
use settlement_client_interface::SettlementClient;
use starknet::{
    accounts::{Account, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount},
    contract::ContractFactory,
    core::types::{
        contract::{CompiledClass, SierraClass},
        BlockId, BlockTag, DeclareTransactionResult, Felt, FunctionCall, InvokeTransactionResult, StarknetError,
        TransactionExecutionStatus, TransactionStatus,
    },
    macros::{felt, selector},
    providers::{jsonrpc::HttpTransport, JsonRpcClient, Provider, ProviderError, Url},
    signers::{LocalWallet, SigningKey},
};
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use utils::settings::env::EnvSettingsProvider;
use utils::settings::Settings;

pub async fn spin_up_madara() -> MadaraCmd {
    log::trace!("Spinning up Madara");
    env::set_current_dir(env::var("MADARA_BINARY_PATH").unwrap()).expect("Failed to set working directory");
    println!("Current working directory: {:?}", env::current_dir().unwrap());
    let mut node = MadaraCmdBuilder::new()
        .args([
            "--network",
            "devnet",
            "--no-sync-polling",
            "--authority",
            "--devnet",
            "--preset=test",
            "--no-l1-sync",
            "--rpc-cors",
            "all",
            "--chain-config-override",
            "--block-time",
            "1",
        ])
        .run();
    node.wait_for_ready().await;
    node
}

async fn wait_for_tx(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>,
    transaction_hash: Felt,
    duration: Duration,
) -> bool {
    let mut attempt = 0;
    loop {
        attempt += 1;
        if attempt >= 5 {
            return false;
        }
        let reciept = match account.provider().get_transaction_status(transaction_hash).await {
            Ok(reciept) => reciept,
            Err(ProviderError::StarknetError(StarknetError::TransactionHashNotFound)) => {
                tokio::time::sleep(duration).await;
                continue;
            }
            _ => panic!("Unknown error"),
        };

        match reciept {
            TransactionStatus::Received => (),
            TransactionStatus::Rejected => return false,
            TransactionStatus::AcceptedOnL2(status) => match status {
                TransactionExecutionStatus::Succeeded => return true,
                TransactionExecutionStatus::Reverted => return false,
            },
            TransactionStatus::AcceptedOnL1(status) => match status {
                TransactionExecutionStatus::Succeeded => return true,
                TransactionExecutionStatus::Reverted => return false,
            },
        }
        // This is done, since currently madara does not increment nonce for pending transactions
        tokio::time::sleep(duration).await;
    }
}

#[fixture]
async fn setup() -> (SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>, MadaraCmd) {
    dotenvy::from_filename_override(".env.test").expect("Failed to load the .env file");

    let madara_process = spin_up_madara().await;
    env::set_var("STARKNET_RPC_URL", madara_process.rpc_url.to_string());

    let env_settings = EnvSettingsProvider::default();
    let rpc_url = Url::parse(&env_settings.get_settings_or_panic("STARKNET_RPC_URL")).unwrap();

    // let provider = JsonRpcClient::new(HttpTransport::new(Url::parse("http://localhost:9944").unwrap()));
    let provider: JsonRpcClient<HttpTransport> = JsonRpcClient::new(HttpTransport::new(rpc_url));
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(
        Felt::from_hex(&env_settings.get_settings_or_panic("STARKNET_PRIVATE_KEY")).expect("Invalid private key"),
    ));
    let address = Felt::from_hex(&env_settings.get_settings_or_panic("STARKNET_ACCOUNT_ADDRESS")).unwrap();

    let chain_id = provider.chain_id().await.unwrap();
    let mut account = SingleOwnerAccount::new(provider, signer, address, chain_id, ExecutionEncoding::New);

    // `SingleOwnerAccount` defaults to checking nonce and estimating fees against the latest
    // block. Optionally change the target block to pending with the following line:
    account.set_block_id(BlockId::Tag(BlockTag::Pending));
    (account, madara_process)
}

#[rstest]
#[tokio::test]
async fn test_deployment(#[future] setup: (SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>, MadaraCmd)) {
    let (account, _madara_process) = setup.await;
    let account = Arc::new(account);

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap();
    let contract_path = project_root.join("crates/settlement-clients/starknet/src/tests/mock_contracts/target/dev");
    let sierra_class: SierraClass = serde_json::from_reader(
        std::fs::File::open(contract_path.join("mock_contracts_Piltover.contract_class.json"))
            .expect("Could not open sierra class file"),
    )
    .expect("Failed to parse SierraClass");

    let compiled_class: CompiledClass = serde_json::from_reader(
        std::fs::File::open(contract_path.join("mock_contracts_Piltover.compiled_contract_class.json"))
            .expect("Could not open compiled class file"),
    )
    .expect("Failed to parse CompiledClass");

    let flattened_class = sierra_class.clone().flatten().unwrap();
    let compiled_class_hash = compiled_class.class_hash().unwrap();

    let DeclareTransactionResult { transaction_hash: declare_tx_hash, class_hash: _ } =
        account.declare_v2(Arc::new(flattened_class.clone()), compiled_class_hash).send().await.unwrap();
    println!("declare tx hash {:?}", declare_tx_hash);

    let is_success = wait_for_tx(&account, declare_tx_hash, Duration::from_secs(2)).await;
    assert!(is_success, "Declare trasactiion failed");

    let contract_factory = ContractFactory::new(flattened_class.class_hash(), account.clone());
    let deploy_v1 = contract_factory.deploy_v1(vec![], felt!("1122"), false);
    let deployed_address = deploy_v1.deployed_address();

    env::set_var("STARKNET_CAIRO_CORE_CONTRACT_ADDRESS", deployed_address.to_hex_string());
    let InvokeTransactionResult { transaction_hash: deploy_tx_hash } =
        deploy_v1.send().await.expect("Unable to deploy contract");

    let is_success = wait_for_tx(&account, deploy_tx_hash, Duration::from_secs(2)).await;
    assert!(is_success, "Deploy trasaction failed");
}

#[rstest]
#[tokio::test]
async fn test_settle(#[future] setup: (SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>, MadaraCmd)) {
    let (account, _madara_process) = setup.await;
    let account = Arc::new(account);

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(3).unwrap();
    let contract_path = project_root.join("crates/settlement-clients/starknet/src/tests/mock_contracts/target/dev");
    let sierra_class: SierraClass = serde_json::from_reader(
        std::fs::File::open(contract_path.join("mock_contracts_Piltover.contract_class.json"))
            .expect("Could not open sierra class file"),
    )
    .expect("Failed to parse SierraClass");

    let compiled_class: CompiledClass = serde_json::from_reader(
        std::fs::File::open(contract_path.join("mock_contracts_Piltover.compiled_contract_class.json"))
            .expect("Could not open compiled class file"),
    )
    .expect("Failed to parse CompiledClass");

    let flattened_class = sierra_class.clone().flatten().unwrap();
    let compiled_class_hash = compiled_class.class_hash().unwrap();

    let DeclareTransactionResult { transaction_hash: declare_tx_hash, class_hash: _ } =
        account.declare_v2(Arc::new(flattened_class.clone()), compiled_class_hash).send().await.unwrap();
    println!("declare tx hash {:?}", declare_tx_hash);

    let is_success = wait_for_tx(&account, declare_tx_hash, Duration::from_secs(2)).await;
    assert!(is_success, "Declare trasactiion failed");

    let contract_factory = ContractFactory::new(flattened_class.class_hash(), account.clone());
    let deploy_v1 = contract_factory.deploy_v1(vec![], felt!("1122"), false);
    let deployed_address = deploy_v1.deployed_address();

    env::set_var("STARKNET_CAIRO_CORE_CONTRACT_ADDRESS", deployed_address.to_hex_string());
    let InvokeTransactionResult { transaction_hash: deploy_tx_hash } =
        deploy_v1.send().await.expect("Unable to deploy contract");

    let is_success = wait_for_tx(&account, deploy_tx_hash, Duration::from_secs(2)).await;
    assert!(is_success, "Deploy trasaction failed");

    let env_settings = EnvSettingsProvider {};
    let settlement_client = StarknetSettlementClient::new_with_settings(&env_settings).await;
    let onchain_data_hash = [1; 32];
    let mut program_output = Vec::with_capacity(32);
    program_output.fill(onchain_data_hash);
    let update_state_tx_hash = settlement_client
        .update_state_calldata(program_output, onchain_data_hash, [1; 32])
        .await
        .expect("Sending Update state");

    println!("update state tx hash {:?}", update_state_tx_hash);

    let is_success = wait_for_tx(
        &account,
        Felt::from_hex(&update_state_tx_hash).expect("Incorrect transaction hash"),
        Duration::from_secs(2),
    )
    .await;
    assert!(is_success, "Update state transaction failed/reverted");

    let call_result = account
        .provider()
        .call(
            FunctionCall {
                contract_address: deployed_address,
                entry_point_selector: selector!("get_is_updated"),
                calldata: vec![Felt::from_bytes_be_slice(&onchain_data_hash)],
            },
            BlockId::Tag(BlockTag::Latest),
        )
        .await
        .expect("failed to call the contract");
    assert!(call_result[0] == true.into(), "Should be updated");
}
