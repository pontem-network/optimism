// @todo
// make devnet-up
// make devnet-down
// make devnet-clean
//
// L1 contract addresses (opens new window).
// $ cat packages/contracts-bedrock/deploy-config/devnetL1.json
//
// There are some differences between the development node and the real world (a.k.a. Ethereum mainnet and OP Mainnet):
//    Parameter	    Real-world	Devnode
//    L1 chain ID	  1	          900
//    L2 chain ID	  10	        901
//    Time between L1 blocks (in seconds)	12	3
//
// Test L2:
//    Address: 0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266
//    Private key: ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

// BLOCK_SIGNER_ADDRESS="0xca062b0fd91172d89bcd4bb084ac4e21972cc467"
// BLOCK_SIGNER_PRIVATE_KEY="3e4bde571b86929bf08e2aaad9a6a1882664cd5e65b96fff7d03e1c4e6dfa15c"

// http://localhost:7300/metrics
//
// Eth: [
//    8545 - L1,
//    9545 - L2
// ]

// test-jwt-secret.txt
// 688f5d737bad920bdfb2fc2f488d6b6209eebda1dae949a8de91398d932c517a

use anyhow::Result;
use log::{debug, error, info};
use mimicaw::{Args, Test};
use secp256k1::SecretKey;
use std::str::FromStr;
use std::time::Duration;
use test_infra::{new_test_account_with_coins, DEV_ACCOUNT_ADDRESS, DEV_ACCOUNT_PRIVATE_KEY};
use tokio::task;
use web3::contract::Options;
use web3::transports::Http;
use web3::types::U256;
use web3::Web3;

use test_infra::check_service::wait_up;
use test_infra::ethereum_key::{SecretKeyGen, SecretKeyToAddressAccount};
use test_infra::solc::{build_sol, check_solc, SolContract};
use test_infra::tx::{transfer_sign, GetTheBalance};

mod mimicaw_helper;
mod tmp; // experiments
mod tmpdir;

use crate::mimicaw_helper::TestHandleResultToOutcom;

use crate::tmp::find_eth_ports;
use crate::tmpdir::TmpDir;

const L1_ADDRESS: &str = "0x3f1Eae7D46d88F08fc2F8ed27FCb2AB183EB2d0E";
const L1_HTTP: &str = "http://127.0.0.1:8545";
const L2_HTTP: &str = "http://127.0.0.1:9545";

type TestFn = fn() -> task::JoinHandle<Result<()>>;

/// RUN NITRO: $ cd nitro-testnode; ./test-node.bash --dev
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder().is_test(true).try_init()?;

    assert!(wait_up(L1_HTTP).await);
    assert!(wait_up(L2_HTTP).await);
    check_solc().await?;

    // tests
    let args = Args::from_env().unwrap_or_else(|st| st.exit());

    let tests: Vec<Test<TestFn>> = vec![
        Test::<TestFn>::test("node_eth::l1::root_account_exist", || {
            task::spawn(async { l1_root_account_exist().await })
        }),
        Test::<TestFn>::test("node_eth::l2::root_account_exist", || {
            task::spawn(async { l2_root_account_exist().await })
        }),
        Test::<TestFn>::test("node_eth::l1::create_new_account", || {
            task::spawn(async { l1_create_new_account().await })
        }),
        Test::<TestFn>::test("node_eth::l2::create_new_account", || {
            task::spawn(async { l2_create_new_account().await })
        }),
        Test::<TestFn>::test("node_eth::l1::deploy_call_contract", || {
            task::spawn(async { l1_deploy_call_contract().await })
        }),
        Test::<TestFn>::test("node_eth::l2::deploy_call_contract", || {
            task::spawn(async { l2_deploy_call_contract().await })
        }),
    ];

    mimicaw::run_tests(&args, tests, |_, test_fn: TestFn| {
        let handle = test_fn();
        async move { handle.await.to_outcome() }
    })
    .await
    .exit();
}

async fn l1_root_account_exist() -> Result<()> {
    let client = Web3::new(Http::new(L1_HTTP)?);

    let root_account = DEV_ACCOUNT_ADDRESS.clone();
    let balance = root_account.balance(&client).await?;
    debug!("[L1 balance] {root_account}: {balance}");
    assert!(balance > U256::from(0));

    Ok(())
}

async fn l2_root_account_exist() -> Result<()> {
    let client = Web3::new(Http::new(L2_HTTP)?);

    let root_account = DEV_ACCOUNT_ADDRESS.clone();
    let balance = root_account.balance(&client).await?;
    debug!("[L1 balance] {root_account}: {balance}");
    assert!(balance > U256::from(0));

    Ok(())
}

async fn l1_create_new_account() -> Result<()> {
    let client = Web3::new(Http::new(L1_HTTP)?);

    let root_account_address = DEV_ACCOUNT_ADDRESS.clone();
    let root_key = DEV_ACCOUNT_PRIVATE_KEY.clone();

    let alice_key = SecretKey::generate();
    let alice_address = alice_key.to_account_address();
    let balance = alice_address.balance(&client).await?;
    debug!("L1 {alice_address}: {balance}");
    assert_eq!(balance, U256::from(0));

    let coins = U256::exp10(20);
    transfer_sign(&client, &root_key, alice_address, coins).await?;

    let balance = alice_address.balance(&client).await?;
    debug!("L1 {alice_address}: {balance}");
    assert_eq!(balance, coins);

    Ok(())
}

async fn l2_create_new_account() -> Result<()> {
    let client = Web3::new(Http::new(L2_HTTP)?);

    let alice_key = SecretKey::generate();
    let alice_address = alice_key.to_account_address();
    let balance = alice_address.balance(&client).await?;
    debug!("L1 {alice_address}: {balance}");
    assert_eq!(balance, U256::from(0));

    let coins = U256::exp10(20);
    transfer_sign(&client, &DEV_ACCOUNT_PRIVATE_KEY, alice_address, coins).await?;

    let balance = alice_address.balance(&client).await?;
    debug!("L1 {alice_address}: {balance}");
    assert_eq!(balance, coins);

    Ok(())
}

async fn l1_deploy_call_contract() -> Result<()> {
    let contract_sol = SolContract::try_from_path("./tests/sources/sol/const_fn.sol").await?;
    let client = Web3::new(Http::new(L1_HTTP)?);

    let alice_key = new_test_account_with_coins(&client).await?;
    let web3_contract =
        web3::contract::Contract::deploy(client.eth(), contract_sol.abi_str().as_bytes())?
            .confirmations(10)
            .poll_interval(Duration::from_secs(12))
            .options(Options::with(|opt| opt.gas = Some(30_000_000.into())))
            .sign_with_key_and_execute(contract_sol.bin_hex(), (), &alice_key, None)
            .await?;

    let contract_address = web3_contract.address();
    println!("Deployed at: 0x{contract_address:x}");

    //
    let result: u64 = web3_contract
        .query("const_fn_10", (), None, Options::default(), None)
        .await?;
    assert_eq!(result, 10);

    //
    let contract =
        web3::contract::Contract::new(client.eth(), contract_address, contract_sol.abi()?);
    let result: bool = web3_contract
        .query("const_fn_true", (), None, Options::default(), None)
        .await?;
    assert!(result);

    Ok(())
}

async fn l2_deploy_call_contract() -> Result<()> {
    let contract_sol = SolContract::try_from_path("./tests/sources/sol/const_fn.sol").await?;
    let client = Web3::new(Http::new(L1_HTTP)?);

    let alice_key = new_test_account_with_coins(&client).await?;
    let web3_contract =
        web3::contract::Contract::deploy(client.eth(), contract_sol.abi_str().as_bytes())?
            .confirmations(10)
            .poll_interval(Duration::from_secs(3))
            .options(Options::with(|opt| opt.gas = Some(100_000.into())))
            .sign_with_key_and_execute(contract_sol.bin_hex(), (), &alice_key, None)
            .await?;

    let contract_address = web3_contract.address();
    println!("Deployed at: 0x{contract_address:x}");

    //
    let result: u64 = web3_contract
        .query("const_fn_10", (), None, Options::default(), None)
        .await?;
    assert_eq!(result, 10);

    //
    let contract =
        web3::contract::Contract::new(client.eth(), contract_address, contract_sol.abi()?);
    let result: bool = web3_contract
        .query("const_fn_true", (), None, Options::default(), None)
        .await?;
    assert!(result);

    Ok(())
}
