use anyhow::Result;
use async_trait::async_trait;
use ethabi::ethereum_types::U256;
use secp256k1::SecretKey;
use std::time::Duration;
use web3::transports::Http;
use web3::types::{Address, TransactionParameters};
use web3::Web3;

#[async_trait]
pub trait GetTheBalance {
    async fn balance(self, client: &Web3<Http>) -> Result<U256>;
}

#[async_trait]
impl GetTheBalance for Address {
    async fn balance(self, client: &Web3<Http>) -> Result<U256> {
        balance(client, self).await
    }
}

pub async fn balance(client: &Web3<Http>, address: Address) -> Result<U256> {
    Ok(client.eth().balance(address, None).await?)
}

pub async fn transfer_sign(
    client: &Web3<Http>,
    sender_key: &SecretKey,
    recipient: Address,
    value: U256,
) -> Result<()> {
    let tx = TransactionParameters {
        to: Some(recipient),
        gas: 50_000.into(),
        value,
        ..Default::default()
    };
    let tx = client.accounts().sign_transaction(tx, sender_key).await?;

    client
        .send_raw_transaction_with_confirmation(tx.raw_transaction, Duration::from_secs(12), 10)
        .await?;
    Ok(())
}
