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
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use tokio::task;
use web3::contract::Options;
use web3::futures::future::join_all;
use web3::transports::Http;
use web3::types::{Address, U256};
use web3::Web3;

use test_infra::check_service::wait_up;
use test_infra::solc::check_solc;

mod eth;
mod mimicaw_helper;
mod sol;
mod tmp; // experiments
mod tmpdir;

use crate::eth::{new_account, root_account, unlock};
use crate::mimicaw_helper::TestHandleResultToOutcom;
use crate::sol::{build_sol, SolContract};
use crate::tmp::find_eth_ports;
use crate::tmpdir::TmpDir;

const L1_ADDRESS: &str = "0x3f1Eae7D46d88F08fc2F8ed27FCb2AB183EB2d0E";
const L1_HTTP: &str = "http://127.0.0.1:8545";
const L2_HTTP: &str = "http://127.0.0.1:9545";

const ACCOUNT_ADDRESS: &str = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";
const ACCOUNT_PRIVATE_KEY: &str =
    "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

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
        Test::<TestFn>::test("node_eth::create_new_account_l1", || {
            task::spawn(async { create_new_account_l1().await })
        }),
        Test::<TestFn>::test("node_eth::create_new_account_l2", || {
            task::spawn(async { create_new_account_l2().await })
        }),
        Test::<TestFn>::test("node_eth::deploy_contract", || {
            task::spawn(async { deploy_contract().await })
        })
        .ignore(true),
    ];

    mimicaw::run_tests(&args, tests, |_, test_fn: TestFn| {
        let handle = test_fn();
        async move { handle.await.to_outcome() }
    })
    .await
    .exit();
}

async fn create_new_account_l1() -> Result<()> {
    let client = Web3::new(Http::new(L1_HTTP)?);
    let mut accounts = client.eth().accounts().await?;
    accounts.push(Address::from_str(ACCOUNT_ADDRESS)?);

    for address in accounts {
        let balance = client.eth().balance(address, None).await?;
        debug!("L1 {address:x}:{balance}");
    }

    todo!();
    // let root_account_address = root_account(&client).await?;
    // new_account(&client, root_account_address).await?;

    // 0xFC5ceF605d3bfC11D6EBe21c1C0304038E65B921
    // {"address":"fc5cef605d3bfc11d6ebe21c1c0304038e65b921","crypto":{"cipher":"aes-128-ctr","ciphertext":"c1db43d5c056f08df2b3bc4b4d474d2d084c35d4f48bbe2ab47a8f5052b33157","cipherparams":{"iv":"d01d157092c89260dbdf1ac2b47940bd"},"kdf":"scrypt","kdfparams":{"dklen":32,"n":262144,"p":1,"r":8,"salt":"76c370e096f853bce8e28162840d6c560d8cc830bd49dd7c41b3efb309fafedb"},"mac":"b24ebdcacef62d0f232a8a1ab03f42ec89ddda3b0c32259847cb0a20e5df3ea0"},"id":"81ee2b72-b634-49bd-98ed-248c4b739aeb","version":3}

    Ok(())
}

async fn create_new_account_l2() -> Result<()> {
    let client = Web3::new(Http::new(L2_HTTP)?);

    let mut accounts = client.eth().accounts().await?;
    accounts.push(Address::from_str(ACCOUNT_ADDRESS)?);

    for address in accounts {
        let balance = client.eth().balance(address, None).await?;
        debug!("L2 {address:x}:{balance}");
    }
    // 0.16226701000000000
    todo!();
    // let root_account_address = root_account(&client).await?;
    // new_account(&client, root_account_address).await?;

    // 0xFC5ceF605d3bfC11D6EBe21c1C0304038E65B921
    // {"address":"fc5cef605d3bfc11d6ebe21c1c0304038e65b921","crypto":{"cipher":"aes-128-ctr","ciphertext":"c1db43d5c056f08df2b3bc4b4d474d2d084c35d4f48bbe2ab47a8f5052b33157","cipherparams":{"iv":"d01d157092c89260dbdf1ac2b47940bd"},"kdf":"scrypt","kdfparams":{"dklen":32,"n":262144,"p":1,"r":8,"salt":"76c370e096f853bce8e28162840d6c560d8cc830bd49dd7c41b3efb309fafedb"},"mac":"b24ebdcacef62d0f232a8a1ab03f42ec89ddda3b0c32259847cb0a20e5df3ea0"},"id":"81ee2b72-b634-49bd-98ed-248c4b739aeb","version":3}

    Ok(())
}

async fn deploy_contract() -> Result<()> {
    todo!()
    // let contract_sol = SolContract::try_from_path("./tests/sol_sources/const_fn.sol").await?;
    //
    // let client = Web3::new(Http::new(&l1_http())?);
    //
    // let root_account_address = root_account(&client).await?;
    // let (alice_address, alice_key) = new_account(&client, root_account_address).await?;
    //
    // let web3_contract =
    //     web3::contract::Contract::deploy(client.eth(), contract_sol.abi_str().as_bytes())?
    //         .confirmations(1)
    //         .poll_interval(Duration::from_secs(1))
    //         .options(Options::with(|opt| opt.gas = Some(3_000_000.into())))
    //         .execute(contract_sol.bin_hex(), (), alice_address)
    //         .await?;
    // let contract_address = web3_contract.address();
    // println!("Deployed at: 0x{contract_address:x}");
    //
    // //
    // let result: u64 = web3_contract
    //     .query("const_fn_10", (), None, Options::default(), None)
    //     .await?;
    // assert_eq!(result, 10);
    //
    // //
    // let contract =
    //     web3::contract::Contract::new(client.eth(), contract_address, contract_sol.abi()?);
    // let result: bool = web3_contract
    //     .query("const_fn_true", (), None, Options::default(), None)
    //     .await?;
    // assert!(result);
    //
    // Ok(())
}
