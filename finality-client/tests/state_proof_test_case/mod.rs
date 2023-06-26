use ethereum_consensus::bellatrix::mainnet as spec;
use ssz_rs::prelude::*;
use test_utils::load_snappy_ssz;

#[derive(Default)]
pub struct StateProofTestCase {
    pub block: spec::BeaconBlock,
}

impl StateProofTestCase {
    pub fn from_eth_spec_path(test_case_path: &str, block: usize) -> Self {
        let path = format!("{test_case_path}/blocks_{block}.ssz_snappy");
        let block: spec::SignedBeaconBlock = load_snappy_ssz(&path).unwrap();
        Self {
            block: block.message,
        }
    }
}
