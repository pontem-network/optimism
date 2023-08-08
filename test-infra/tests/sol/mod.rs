use std::path::{Path, PathBuf};

use anyhow::{anyhow, ensure, Result};
use ethabi::Contract;

#[derive(Debug, Clone)]
pub(crate) struct SolContract {
    name: String,
    path: PathBuf,
    // hex
    pub bin_hex: String,
    pub abi_string: String,
}

impl SolContract {
    pub(crate) async fn try_from_path<P: AsRef<Path>>(path_to_sol: P) -> Result<SolContract> {
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

    pub(crate) fn abi(&self) -> Result<Contract> {
        Ok(serde_json::from_str(self.abi_str())?)
    }
}

pub(crate) async fn build_sol<P: AsRef<Path>>(path_to_sol: P) -> Result<SolContract> {
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
