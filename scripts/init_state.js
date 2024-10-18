/// The purpose of this script is to initialize the state of the madara orchestrator
/// This is what it will do
/// 1. Do a dummy transaction on Madara (SNOS currently can't run on empty blocks)
/// 2. Fetch the latest block B from Madara and ensure it can be run in SNOS
///    - Gas fees and Data gas prices are not 0 (eth and strk)
///    - The block has at least one transaction
/// 3. Add data in the mongo DB fit for block B-1
/// 4. Call updateStateOverride to set the core contract state to B-1

const starknet = require("starknet");
const ethers = require("ethers");
const { MongoClient } = require("mongodb");
const { v4 } = require("uuid");

// using default anvil key which has funds
const ETHEREUM_PRIVATE_KEY =
  "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const eth_provider = new ethers.JsonRpcProvider("http://localhost:8545");
const wallet = new ethers.Wallet(ETHEREUM_PRIVATE_KEY, eth_provider);

const starknet_provider = new starknet.RpcProvider({
  nodeUrl: "http://localhost:9944",
});
// TODO: fetch these from bootstrapper output
const ETHEREUM_APP_CHAIN_ADDRESS =
  "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7";
const OZ_ACCOUNT_CLASS_HASH =
  "0x01484c93b9d6cf61614d698ed069b3c6992c32549194fc3465258c2194734189";

// TODO: Fetch from env
const MONGO_URL = "mongodb://localhost:27017/orchestrator";

async function getAppChainBalance(address) {
  const abi = [
    {
      name: "balanceOf",
      type: "function",
      inputs: [
        {
          name: "account",
          type: "felt",
        },
      ],
      outputs: [
        {
          name: "balance",
          type: "Uint256",
        },
      ],
      stateMutability: "view",
    },
  ];
  const ethContract = new starknet.Contract(
    abi,
    ETHEREUM_APP_CHAIN_ADDRESS,
    starknet_provider,
  );

  // Interaction with the contract with call
  const balance = await ethContract.balanceOf(address);
  return balance.balance;
}

async function bridgeToChain(bridge_address, starnet_expected_account_address) {
  // call deposit function with 10 as argument and also send 10 eth to the contract
  const contract = new ethers.Contract(
    bridge_address,
    ["function deposit(uint256, uint256)"],
    wallet,
  );

  const initial_app_chain_balance = await getAppChainBalance(
    starnet_expected_account_address,
  );
  const tx = await contract.deposit(
    ethers.parseEther("1"),
    starnet_expected_account_address,
    { value: ethers.parseEther("2") },
  );

  tx.wait();
  // wait for the transaction to be successful
  console.log("✅ Successfully sent 1 ETH on L1 bridge");

  let counter = 10;
  while (counter--) {
    const final_app_chain_balance = await getAppChainBalance(
      starnet_expected_account_address,
    );
    if (final_app_chain_balance > initial_app_chain_balance) {
      console.log(
        "💰 App chain balance:",
        (final_app_chain_balance / 10n ** 18n).toString(),
        "ETH",
      );
      return;
    }
    console.log("🔄 Waiting for funds to arrive on app chain...");
    await new Promise((resolve) => setTimeout(resolve, 5000));
  }
  console.log("❌ Failed to get funds on app chain");
  process.exit(1);
}

function calculatePrefactualAccountAddress() {
  // new Open Zeppelin account v0.8.1
  // Generate public and private key pair.
  const privateKey = starknet.stark.randomAddress();
  console.log("🔑 Starknet private key:", privateKey);
  const starkKeyPub = starknet.ec.starkCurve.getStarkKey(privateKey);
  console.log("🔑 Starknet public key:", starkKeyPub);

  // Calculate future address of the account
  const OZaccountConstructorCallData = starknet.CallData.compile({
    publicKey: starkKeyPub,
  });
  const OZcontractAddress = starknet.hash.calculateContractAddressFromHash(
    starkKeyPub,
    OZ_ACCOUNT_CLASS_HASH,
    OZaccountConstructorCallData,
    0,
  );
  return {
    address: OZcontractAddress,
    private_key: privateKey,
    public_key: starkKeyPub,
  };
}

async function validateBlockPassesSnosChecks(block_number) {
  console.log("⏳ Checking if block", block_number, "can be run in SNOS...");
  const block = await starknet_provider.getBlock(block_number);

  // block number must be >= 10
  if (block_number < 10) {
    console.log("❌ Block number must be >= 10");
    process.exit(1);
  }
  console.log("✅ Block number is >= 10");

  // block must not be empty
  if (block.transactions.length === 0) {
    console.log("❌ Block has no transactions");
    process.exit(1);
  }
  console.log("✅ Block has transactions");

  // gas price shouldn't be 0
  if (
    block.l1_gas_price.price_in_fri == 0 ||
    block.l1_gas_price.price_in_wei == 0
  ) {
    console.log("❌ L1 gas price is 0", block.l1_gas_price);
    process.exit(1);
  }
  console.log("✅ L1 gas price is non zero");

  // data as price shouldn't be 0
  if (
    block.l1_data_gas_price.price_in_fri == 0 ||
    block.l1_data_gas_price.price_in_wei == 0
  ) {
    console.log("❌ L1 data gas price is 0", block.l1_data_gas_price);
    process.exit(1);
  }
  console.log("✅ L1 data gas price is non zero");
}

async function deployStarknetAccount(
  starknet_private_key,
  starnet_expected_account_address,
  starknet_account_public_key,
) {
  console.log("⏳ Deploying Starknet account...");
  const account = new starknet.Account(
    starknet_provider,
    starnet_expected_account_address,
    starknet_private_key,
    "1",
  );
  const { transaction_hash, contract_address } = await account.deployAccount({
    classHash: OZ_ACCOUNT_CLASS_HASH,
    constructorCalldata: [starknet_account_public_key],
    addressSalt: starknet_account_public_key,
  });

  let receipt = await starknet_provider.waitForTransaction(transaction_hash);
  if (!receipt.isSuccess()) {
    console.log("❌ Failed to deploy Starknet account");
    process.exit(1);
  }
  console.log("✅ Successfully deployed Starknet account");
  return receipt.block_number;
}

// Due to restrictions in SNOS at the moment (as it's designed for Sepolia right now),
// we need to skip the starting few blocks from running on SNOS.
// This function overrides the state on the core contract to the block after which we
// can run SNOS
async function overrideStateOnCoreContract(
  block_number,
  core_contract_address,
) {
  let state_update = await starknet_provider.getStateUpdate(block_number);
  const artifactPath =
    "/Users/apoorvsadana/Documents/GitHub/madara-starknet-test-contracts/out/Starknet.sol/Starknet.json";
  let abi = [
    {
      type: "function",
      name: "updateStateOverride",
      inputs: [
        {
          name: "globalRoot",
          type: "uint256",
          internalType: "uint256",
        },
        {
          name: "blockNumber",
          type: "int256",
          internalType: "int256",
        },
        {
          name: "blockHash",
          type: "uint256",
          internalType: "uint256",
        },
      ],
      outputs: [],
      stateMutability: "nonpayable",
    },
  ];

  const contract = new ethers.Contract(core_contract_address, abi, wallet);
  const tx = await contract.updateStateOverride(
    state_update.new_root,
    block_number,
    state_update.block_hash,
  );
  const receipt = await tx.wait();
  if (!receipt.status) {
    console.log("❌ Failed to override state on core contract");
    process.exit(1);
  }
  console.log("✅ Successfully overridden state on core contract");
}

async function setupMongoDb(block_number) {
  const client = new MongoClient(MONGO_URL);
  await client.connect();
  let db = client.db("orchestrator");
  const collection = db.collection("jobs");

  // delete everything in the collection
  await collection.deleteMany({});

  // insert all jobs
  let insert_promises = [
    "SnosRun",
    "ProofCreation",
    "DataSubmission",
    "StateTransition",
  ].map(async (job_type) => {
    console.log("Inserting job:", job_type);
    let metadata = {};
    if (job_type === "StateTransition") {
      metadata = {
        blocks_number_to_settle: String(block_number),
      };
    }
    await collection.insertOne({
      job_type,
      internal_id: String(block_number),
      external_id: "",
      status: "Completed",
      created_at: new Date(),
      updated_at: new Date(),
      id: v4(),
      metadata,
      version: 0,
    });
  });
  await Promise.all(insert_promises);
  await client.close();
  console.log("✅ Successfully inserted all jobs in MongoDB");
}

async function transfer(
  starknet_account_private_key,
  starnet_expected_account_address,
) {
  const account = new starknet.Account(
    starknet_provider,
    starnet_expected_account_address,
    starknet_account_private_key,
    "1",
  );
  const abi = [
    {
      members: [
        {
          name: "low",
          offset: 0,
          type: "felt",
        },
        {
          name: "high",
          offset: 1,
          type: "felt",
        },
      ],
      name: "Uint256",
      size: 2,
      type: "struct",
    },
    {
      inputs: [
        {
          name: "recipient",
          type: "felt",
        },
        {
          name: "amount",
          type: "Uint256",
        },
      ],
      name: "transfer",
      outputs: [
        {
          name: "success",
          type: "felt",
        },
      ],
      type: "function",
    },
  ];
  const contract = new starknet.Contract(
    abi,
    ETHEREUM_APP_CHAIN_ADDRESS,
    starknet_provider,
  );
  let calldata = contract.populate("transfer", {
    recipient: "0x1234",
    amount: {
      low: 1,
      high: 0,
    },
  });

  let txn_hash = await account.execute(calldata);
  let receipt = await starknet_provider.waitForTransaction(
    txn_hash.transaction_hash,
  );
  if (!receipt.isSuccess()) {
    console.log("❌ Failed to do a transfer on Starknet account");
    process.exit(1);
  }
  console.log("✅ Successfully did a transfer on Starknet account");
  return receipt.block_number;
}

async function main() {
  // tage bridge address as an argument --bridge_address
  const bridge_address = process.argv[2];
  const core_contract_address = process.argv[3];
  const {
    address: starnet_expected_account_address,
    private_key: starknet_account_private_key,
    public_key: starknet_account_public_key,
  } = calculatePrefactualAccountAddress();
  console.log(
    "🏦 Starknet expected account address:",
    starnet_expected_account_address,
  );

  await bridgeToChain(bridge_address, starnet_expected_account_address);

  let block_number = await deployStarknetAccount(
    starknet_account_private_key,
    starnet_expected_account_address,
    starknet_account_public_key,
  );

  // SNOS doesn't seem to be able to run on deploy account block
  await starknet_provider.waitForBlock(block_number + 1);

  block_number = await transfer(
    starknet_account_private_key,
    starnet_expected_account_address,
  );

  await validateBlockPassesSnosChecks(block_number);

  // overriding core contract state to block_number - 1 so that
  // we can run orchestrator on block_number
  await overrideStateOnCoreContract(block_number - 1, core_contract_address);

  await setupMongoDb(block_number - 1);
}

main();
