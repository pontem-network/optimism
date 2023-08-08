use anyhow::Result;
use log::{debug, info};
use serde_json::Value;
use web3::futures::future::join_all;
use web3::transports::Http;
use web3::Web3;

#[inline]
fn ports() -> Vec<u16> {
    std::fs::read_to_string("../docker_ports.list")
        .map(|cont| {
            cont.lines()
                .map(|line| line.trim())
                .filter(|line| line.chars().all(|ch| ch.is_ascii_digit()))
                .filter_map(|line| line.parse::<u16>().ok())
                .collect::<Vec<u16>>()
        })
        .unwrap_or_default()
}

async fn f_check_rpc__optimism_outputAtBlock(port: u16) -> Result<String> {
    debug!("check_rpc_url: {port}");

    let body: Value = serde_json::from_str(
        r#"{"jsonrpc":"2.0","method":"optimism_outputAtBlock","params":["latest"],"id":1}"#,
    )?;

    let result = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}"))
        .json(&body)
        .send()
        .await?
        .text()
        .await?;

    Ok(result)
}

async fn check_rpc__optimism_outputAtBlock() -> Result<()> {
    let ports: Vec<u16> = ports();

    let success = join_all(
        ports
            .iter()
            .map(|port| f_check_rpc__optimism_outputAtBlock(*port)),
    );
    for (result, port) in success
        .await
        .into_iter()
        .zip(ports)
        .filter_map(|(result, port)| Some((result.ok()?, port)))
        .filter(|(result, _)| !result.contains("404 page not found"))
        .filter(|(result, _)| {
            !result.contains("the method optimism_outputAtBlock does not exist/is not available")
        })
    {
        // debug!("{result}");
        info!("Success: {port}");
        // debug!("{x:?}");
    }
    Ok(())
}

pub(crate) async fn find_eth_ports() -> Vec<u16> {
    let result = ports().into_iter().filter_map(|port| {
        let client = Web3::new(Http::new(&format!("http://127.0.0.1:{port}")).ok()?);
        Some(client.eth().accounts())
    });

    join_all(result)
        .await
        .into_iter()
        .zip(ports())
        .filter_map(|(result, port)| {
            result.ok()?;
            Some(port)
        })
        .collect()
}
