use anyhow::{ensure, Result};
use log::debug;
use std::time::Duration;
use web3::transports::Http;
use web3::types::{Address, TransactionRequest, U256};
use web3::Web3;

const L1PASSPHRASE: &str = "";

type PrivateKeyBytes = [u8; 32];

pub(crate) fn generate_private_key() -> PrivateKeyBytes {
    use rand::Rng;

    rand::thread_rng().gen::<PrivateKeyBytes>()
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
) -> Result<(Address, PrivateKeyBytes)> {
    let new_private_key = generate_private_key();
    let new_account_address = client
        .personal()
        .import_raw_key(&new_private_key, L1PASSPHRASE)
        .await?;
    debug!("An account has been created: 0x{new_account_address:x}");

    // fund new account
    let coins = U256::exp10(20);
    let tx_object = TransactionRequest {
        from: root_account_address,
        to: Some(new_account_address),
        gas: Some(50_000.into()),
        value: Some(coins),
        ..Default::default()
    };

    client
        .send_transaction_with_confirmation(tx_object, Duration::from_secs(1), 1)
        .await?;
    debug!(
        "Fund new account 0x{new_account_address:x}: {} ETH",
        coins / U256::exp10(18)
    );
    unlock(client, new_account_address).await?;

    Ok((new_account_address, new_private_key))
}
