use anyhow::{ensure, Result};
use log::debug;
use secp256k1::{Secp256k1, SecretKey};
use std::str::FromStr;
use std::time::Duration;
use web3::transports::Http;
use web3::types::{Address, TransactionRequest, H256, U256};
use web3::Web3;

const L1PASSPHRASE: &str = "";

type PrivateKeyBytes = [u8; 32];

pub(crate) fn generate_private_key() -> Result<SecretKey> {
    use rand::Rng;

    let private_key = rand::thread_rng().gen::<PrivateKeyBytes>();

    Ok(SecretKey::from_slice(&private_key)?)
}

/// Calculates the address from the private key
/// ```
/// use ethereum_private_key_to_address::PrivateKey;
/// use std::str::FromStr;
///
/// let pk = PrivateKey::from_str("ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80").unwrap();
///
/// println!("{}", pk.address());
/// ```
pub fn private_key_to_address(private_key: &SecretKey) -> Address {
    use sha3::{Digest, Keccak256};

    let secp = Secp256k1::new();
    let public_key = private_key.public_key(&secp);
    let public_key = public_key.serialize_uncompressed()[1..].to_vec();
    let mut hasher = Keccak256::new();
    hasher.update(public_key);
    let address = hasher.finalize();

    Address::from_slice(&address[12..32])
}

pub(crate) async fn root_account(client: &Web3<Http>) -> Result<Address> {
    let mut accounts = Vec::default();
    for x in client.eth().accounts().await? {
        let balance = client.eth().balance(x, None).await?;
        accounts.push((x, balance));
    }
    ensure!(!accounts.is_empty(), "Failed to get root_account");
    let root_account = accounts.iter().max_by(|a, b| a.1.cmp(&b.1)).unwrap();

    debug!(
        "Root Account 0x{:x}: {} ETH",
        root_account.0,
        root_account.1 / U256::exp10(18)
    );
    unlock(&client, root_account.0).await?;

    Ok(root_account.0)
}

pub(crate) async fn unlock(client: &Web3<Http>, account: Address) -> Result<()> {
    debug!("unlocking account: 0x{account:x}");
    client
        .personal()
        .unlock_account(account, L1PASSPHRASE, Some(u16::MAX))
        .await?;

    Ok(())
}

pub(crate) async fn new_account(
    client: &Web3<Http>,
    root_account_address: Address,
) -> Result<(Address, SecretKey)> {
    // SecretKey
    let new_private_key = generate_private_key()?;
    dbg!(1);
    let new_account_address = client
        .personal()
        .import_raw_key(new_private_key.as_ref(), L1PASSPHRASE)
        .await?;
    debug!("An account has been created: 0x{new_account_address:x}");
    unlock(client, root_account_address).await?;
    dbg!(2);
    // fund new account
    let coins = U256::exp10(20);
    let tx_object = TransactionRequest {
        from: root_account_address,
        to: Some(new_account_address),
        gas: Some(50_000.into()),
        value: Some(coins),
        ..Default::default()
    };

    dbg!(3);
    let r = client
        .send_transaction_with_confirmation(tx_object, Duration::from_secs(1), 1)
        .await?;
    dbg!(4);
    dbg!(&r);

    debug!(
        "Fund new account 0x{new_account_address:x}: {} ETH",
        coins / U256::exp10(18)
    );
    unlock(client, new_account_address).await?;

    Ok((new_account_address, new_private_key))
}
