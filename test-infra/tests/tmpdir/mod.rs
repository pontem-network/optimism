use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use log::{debug, error, warn};
use rand::Rng;

#[derive(Debug)]
pub struct TmpDir {
    path: PathBuf,
    pub delete_before_drop: bool,
}

impl TmpDir {
    pub fn new() -> Result<Self> {
        debug!("TmpDir::new");

        let root_tmp_dir = std::env::temp_dir().join("tests_nitro");
        debug!("Tests dir: {root_tmp_dir:?} ");

        if !root_tmp_dir.exists() {
            debug!("{root_tmp_dir:?} exist");
            fs::create_dir(&root_tmp_dir)?;
        }

        loop {
            let random_dir_name = format!("test_{}", rand::thread_rng().gen_range(0..usize::MAX));

            let tmp_dir = root_tmp_dir.join(random_dir_name);
            debug!("Tmp Dir: {tmp_dir:?}");

            if tmp_dir.exists() {
                debug!("exist");
                continue;
            }

            return Ok(Self {
                path: fs::create_dir(&tmp_dir).map(|_| tmp_dir)?,
                delete_before_drop: true,
            });
        }
    }

    pub fn to_pat_buf(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn delete(path: PathBuf) -> Result<()> {
        if !path.exists() {
            warn!("Not {:?} exist", &path);
            return Ok(());
        }

        fs::remove_dir_all(&path)?;

        Ok(())
    }
}

impl Drop for TmpDir {
    fn drop(&mut self) {
        if !self.delete_before_drop {
            return;
        }
        debug!("TmpDir {:?} drop", &self.path);

        if let Err(err) = TmpDir::delete(self.path.clone()) {
            error!("{err}");
        }

        debug!("Drop");
    }
}

impl AsRef<Path> for TmpDir {
    fn as_ref(&self) -> &Path {
        self.path.as_path()
    }
}
