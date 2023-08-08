use anyhow::{anyhow, ensure, Result};
use log::{debug, error};
use regex::Regex;
use semver::{Version, VersionReq};
use std::str::FromStr;

/// Version solc "0.8.19" is expected
///
/// OPCODE push0 is not yet supported, but will soon be available.
/// This means that solidity version 0.8.20 or higher can only be used with an evm-version lower than
/// the default shanghai (see instructions here to change that parameter in solc, or here to set the
/// solidity or evmVersion configuration parameters in hardhat). Versions up to 0.8.19 (included) are
/// fully compatible.
/// Source: https://developer.arbitrum.io/solidity-support
///
/// Error: Error checking the "solc" version or unsuitable "solc" version
///
pub async fn check_solc() -> Result<()> {
    let exp = "=0.8.19";
    let req = VersionReq::parse(exp)?;

    let version = version_solc().await?;

    ensure!(
        req.matches(&version),
        "Version {exp:?} is expected. The current version is {version:?}"
    );

    Ok(())
}

async fn version_solc() -> Result<Version> {
    debug!("$ solc --version");

    let result = tokio::process::Command::new("solc")
        .arg("--version")
        .output()
        .await?;

    let stderr = String::from_utf8(result.stderr)?;
    let stdout = String::from_utf8(result.stdout)?;

    if !stderr.is_empty() {
        error!("{stderr}");
    }

    { result.status.success() && !stdout.is_empty() }
        .then_some(&stdout)
        .and_then(|stdout| stdout.split_once("Version:"))
        .map(|(_, version)| version.trim())
        .ok_or_else(|| {
            debug!("{stdout}");
            anyhow!("Couldn't get the \"solc\" version")
        })
        .and_then(string_to_version)
}

fn string_to_version(source: &str) -> Result<Version> {
    debug!("Parse string: {source}");

    let rg = Regex::new(r"^(.*[^d])?(?<version>(\d+\.){2}\d+)([^d].*)?$")?;
    let version_str = rg
        .captures(source)
        .and_then(|f| f.name("version"))
        .map(|version| version.as_str().trim())
        .ok_or_else(|| anyhow!("Not found"))?;
    debug!("Version: {version_str}");

    Ok(Version::from_str(&version_str)?)
}
