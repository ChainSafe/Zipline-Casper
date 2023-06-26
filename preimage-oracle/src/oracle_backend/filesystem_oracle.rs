// use crate::oracle_backend::Oracle;
use crate::{error::PreimageOracleError, oracle_backend::PreimageOracle, H256};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
/// A mock oracle that loads files from a given directory
/// The files within the directory should have names which are 32 byte hex encoded with 0x prefix
/// The contents if the files should be the pre-image of this hash
#[derive(Clone)]
pub struct FilesystemOracle {
    root_dir: PathBuf,
}

impl FilesystemOracle {
    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }
}

fn load_file(root_dir: &Path, file_name: &str) -> std::io::Result<Vec<u8>> {
    let path = root_dir.join(file_name);
    std::fs::read(path)
}

fn load_preimage(root_dir: &Path, hash: H256) -> std::io::Result<Vec<u8>> {
    let path = format!("0x{:}", hex::encode(hash));
    let data = load_file(root_dir, &path)?;
    if sha256(&data) != hash {
        println!(
            "warning: contents of file {:?} is not the correct preimage",
            path
        );
    }
    Ok(data)
}

fn sha256(data: &[u8]) -> H256 {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

impl PreimageOracle<H256> for FilesystemOracle {
    fn map<T, F>(&self, key: [u8; 32], f: F) -> Result<T, PreimageOracleError>
    where
        F: FnOnce(&[u8]) -> T,
    {
        let data: Vec<u8> = load_preimage(&self.root_dir, key)
            .map_err(|_| PreimageOracleError::PreimageNotFound(format!("{:?}", key)))?;

        Ok(f(&data))
    }

    fn get_cached(&self, _hash: [u8; 32]) -> Option<&'static [u8]> {
        todo!()
    }
}
