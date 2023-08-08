use log::{debug, error};
use std::time::Duration;
use tokio::time::sleep;

#[inline]
pub(crate) async fn check_up(rest_url: &str) -> bool {
    match reqwest::get(rest_url).await {
        Ok(_) => true,
        Err(err) => {
            error!("{err}");
            false
        }
    }
}

pub async fn wait_up(rest_url: &str) -> bool {
    for num in 0..30 {
        debug!("Waiting for the server: #{num}");
        if check_up(rest_url).await {
            debug!("Server is up");
            return true;
        }
        sleep(Duration::from_secs(1)).await;
    }

    error!("Failed to connect to the Nitro server");
    false
}
