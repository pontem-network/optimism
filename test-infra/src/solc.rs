use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{anyhow, ensure, Result};
use ethabi::Contract;
use log::{debug, error};
use regex::Regex;
use semver::{Version, VersionReq};

#[derive(Debug, Clone)]
pub struct SolContract {
    name: String,
    path: PathBuf,
    // hex
    pub bin_hex: String,
    pub abi_string: String,
}

impl SolContract {
    pub async fn try_from_path<P: AsRef<Path>>(path_to_sol: P) -> Result<SolContract> {
        build_sol(path_to_sol).await
    }
}

impl SolContract {
    pub fn bin_hex(&self) -> &str {
        &self.bin_hex
    }

    pub fn abi_str(&self) -> &str {
        &self.abi_string
    }

    pub fn abi(&self) -> Result<Contract> {
        Ok(serde_json::from_str(self.abi_str())?)
    }
}

pub async fn build_sol<P: AsRef<Path>>(path_to_sol: P) -> Result<SolContract> {
    let path_to_sol = path_to_sol.as_ref();
    let path = path_to_sol
        .canonicalize()
        .map_err(|err| anyhow!("{err}. Path: {path_to_sol:?}"))?;

    ensure!(
        path.extension().map(|ext| ext == "sol").unwrap_or_default(),
        "The path to the file with the extension \"sol\" was expected. Path: {path:?}"
    );

    let output = tokio::process::Command::new("solc")
        .args(["--combined-json", "abi,bin"])
        .arg(&path)
        .output()
        .await?;
    ensure!(
        output.status.success(),
        "Compilation error:\n{}",
        String::from_utf8(output.stderr).unwrap_or_default()
    );
    let output = String::from_utf8(output.stdout)?;
    let output_json: serde_json::Value = serde_json::from_str(&output)?;

    let mut result = output_json
        .get("contracts")
        .and_then(|contracts| contracts.as_object())
        .map(|contracts| {
            contracts
                .iter()
                .filter_map(|(index, value)| {
                    let (path, name) = index.rsplit_once(":")?;
                    let path = PathBuf::from(path).canonicalize().ok()?;

                    let abi = value.get("abi")?;
                    let bin_hex = value.get("bin").and_then(|v| v.as_str())?.to_string();

                    Some(SolContract {
                        path,
                        name: name.to_string(),
                        bin_hex,
                        abi_string: abi.to_string(),
                    })
                })
                .collect::<Vec<SolContract>>()
        })
        .ok_or_else(|| anyhow!("invalid sol file"))?;

    ensure!(
        result.len() == 1,
        "The file contains more than one contract"
    );

    Ok(result.remove(0))
}

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
