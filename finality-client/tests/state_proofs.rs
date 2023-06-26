mod state_proof_test_case;

use ethereum_consensus::bellatrix::mainnet as spec;
use ssz_rs::{compute_proof, is_valid_merkle_branch, Merkleized, Node};
use state_proof_test_case::StateProofTestCase;

macro_rules! test_path {
    ($t:literal) => {
        concat!(
            "../consensus-spec-tests/tests/mainnet/bellatrix/finality/finality/pyspec_tests/",
            $t
        )
    };
}

#[test]
fn test_make_proof() {
    let mut block = StateProofTestCase::from_eth_spec_path(test_path!("finality_rule_3"), 0).block;
    let proof = make_state_proof(&mut block);
    let root = block.hash_tree_root().unwrap();

    assert!(is_valid_merkle_branch(
        &block.state_root,
        proof.iter().map(node_from_hash).collect::<Vec<_>>().iter(),
        3,
        11,
        &root
    ));
}

fn node_from_hash(h: &[u8; 32]) -> Node {
    Node::try_from(h.as_ref()).expect("is right size")
}

fn make_state_proof(b: &mut spec::BeaconBlock) -> Vec<[u8; 32]> {
    let tree = b.to_merkle_tree().unwrap();
    compute_proof(&b.hash_tree_root().unwrap(), 11, &tree).unwrap()
}
