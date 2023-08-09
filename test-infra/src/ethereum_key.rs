use anyhow::Result;
use rand::{random, Rng};
use secp256k1::{Secp256k1, SecretKey};
use sha3::{Digest, Keccak256};

use web3::types::Address;

type PrivateKeyBytes = [u8; 32];

pub trait SecretKeyToAddressAccount {
    fn to_account_address(&self) -> Address;
}

impl SecretKeyToAddressAccount for SecretKey {
    fn to_account_address(&self) -> Address {
        secret_key_to_address(self)
    }
}

pub fn secret_key_to_address(private_key: &SecretKey) -> Address {
    let secp = Secp256k1::new();
    let public_key = private_key.public_key(&secp);
    let public_key = public_key.serialize_uncompressed()[1..].to_vec();
    let mut hasher = Keccak256::new();
    hasher.update(public_key);
    let address = hasher.finalize();

    Address::from_slice(&address[12..32])
}

pub trait SecretKeyGen {
    fn generate() -> SecretKey;
}

impl SecretKeyGen for SecretKey {
    fn generate() -> SecretKey {
        SecretKey::from_slice(&random::<PrivateKeyBytes>()).expect("failed to create secret key")
    }
}
