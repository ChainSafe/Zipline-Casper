use std::collections::HashMap;
// An oracle provider is responsible for fetching data given its hash key
// This could read from a HashMap or from a database or filesystem
pub trait OracleProvider {
    fn get<'a>(&'a self, key: &[u8; 32]) -> &'a [u8];
}

impl OracleProvider for HashMap<[u8; 32], Vec<u8>> {
    fn get<'a>(&'a self, key: &[u8; 32]) -> &'a [u8] {
        self.get(key)
            .unwrap_or_else(|| panic!("Preimage Oracle key {:?} not found", key))
    }
}

impl OracleProvider for std::collections::BTreeMap<[u8; 32], Vec<u8>> {
    fn get<'a>(&'a self, key: &[u8; 32]) -> &'a [u8] {
        self.get(key)
            .unwrap_or_else(|| panic!("reimage Oracle key {:?} not found", key))
    }
}
