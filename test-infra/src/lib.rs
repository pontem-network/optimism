use anyhow::Result;
use lazy_static::lazy_static;
use log::debug;
use secp256k1::SecretKey;
use std::str::FromStr;
use web3::transports::Http;
use web3::types::{Address, U256};
use web3::Web3;

pub mod check_service;
pub mod solc;

pub mod ethereum_key;

pub mod tx;

use crate::ethereum_key::{SecretKeyGen, SecretKeyToAddressAccount};
use crate::tx::{transfer_sign, GetTheBalance};

lazy_static! {
    pub static ref DEV_ACCOUNT_ADDRESS: Address = {
        Address::from_str("0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266")
            .expect(r#"Invalid address of the "dev" account"#)
    };
    pub static ref DEV_ACCOUNT_PRIVATE_KEY: SecretKey = {
        SecretKey::from_str("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
            .expect(r#"Invalid private key of the "dev" account"#)
    };
}

pub async fn new_test_account_with_coins(client: &Web3<Http>) -> Result<SecretKey> {
    let new_account_key = SecretKey::generate();
    let new_account_address = new_account_key.to_account_address();

    transfer_sign(
        &client,
        &DEV_ACCOUNT_PRIVATE_KEY,
        new_account_address,
        U256::exp10(20),
    )
    .await?;

    Ok(new_account_key)
}
