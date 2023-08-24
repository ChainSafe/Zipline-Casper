#![feature(generic_arg_infer)]
extern crate alloc;
use alloc::collections::btree_map::BTreeMap as Map;
use alloc::vec::Vec;
use cannon_unicorn::{new_cannon_unicorn, run, write_input, write_program, TraceConfig, UnsyncRam};
use crypto::hash::hash;
use ethereum_consensus::bellatrix::mainnet as spec;
use ethereum_consensus::capella;
use preimage_oracle::hashmap_oracle::HashMapOracle;
use ssz_rs::prelude::*;
use std::io::Write;
use std::sync::Once;
use zipline_finality_client::ssz_state_reader::{PatchedSszStateReader, SszStateReader};
use zipline_finality_client::{input::ZiplineInput, verify};
use zipline_spec::{MainnetSpec, SpecTestSpec};
use zipline_test_case::ZiplineTestCase;
use std::io::Read;


use crate::direct_state_reader::DirectStateReader;
use crate::direct_state_reader::PatchedDirectStateReader;

static INIT: Once = Once::new();

/// Setup function that is only run once, even if called multiple times.
fn setup() {
    INIT.call_once(|| {
        let _ = env_logger::builder().is_test(true).try_init();
    });
}

mod attestation_test_case;
mod direct_state_reader;
mod zipline_test_case;

macro_rules! test_path {
    ($t:literal) => {
        concat!(
            "../consensus-spec-tests/tests/mainnet/bellatrix/finality/finality/pyspec_tests/",
            $t
        )
    };
}

// Before running the MIPS tests make sure the state transition binary has been built
// by running `build.sh` in the zipline-state-transition-mips directory
const MIPS_SPEC_TEST_BIN_PATH: &str = "../zipline-state-transition-mips/build/spec_test_out.bin";
const MIPS_MAINNET_BIN_PATH: &str = "../zipline-state-transition-mips/build/mainnet_out.bin";

//////////////////////////////////////////////
///          Test caching.
/// Producing zipline tests from eth-spec tests
/// is a time consuming task so we cache it to disk.
/// use `cargo test -p zipline-finality-client -- --ignored`
/// to generate the cached test cases
/////////////////////////////////////////////

#[test]
#[ignore]
fn cache_finality_rule_3() {
    let test_cases =
        ZiplineTestCase::from_eth_spec_path::<SpecTestSpec>(test_path!("finality_rule_3"));
    for (i, case) in test_cases.iter().enumerate() {
        case.serialize_to_file(&format!("test_finality_rule_3_{}.ssz", i));
    }
}

// generates the files needed to run the high level demo script
// these are kept in the repo to make running the demo as easy as possible
#[test]
#[ignore]
fn cache_demo_files() {
    let mut test = ZiplineTestCase::deserialize_from_file("test_finality_rule_3_1.ssz");

    // write the trusted checkpoint and candidate checkpoint to file
    let mut f = std::fs::File::create("../demo/demo_data/trusted_cp.txt").unwrap();
    write!(
        f,
        "{}:0x{}",
        test.trusted.epoch,
        hex::encode(test.trusted.root)
    )
    .unwrap();

    let mut f = std::fs::File::create("../demo/demo_data/candidate_cp.txt").unwrap();
    write!(
        f,
        "{}:0x{}",
        test.candidate.epoch,
        hex::encode(test.candidate.root)
    )
    .unwrap();

    let fake_candidate = [0xff; 32];
    let mut f = std::fs::File::create("../demo/demo_data/fraud_candidate_cp.txt").unwrap();
    write!(
        f,
        "{}:0x{}",
        test.candidate.epoch,
        hex::encode(fake_candidate)
    )
    .unwrap();

    // prepare the zipline input file
    let mut input = test.to_input();
    let mut f = std::fs::File::create("../demo/demo_data/input.ssz.bin").unwrap();
    f.write_all(&serialize(&input).unwrap()).unwrap();

    input.candidate_cp.root = fake_candidate;
    let mut f = std::fs::File::create("../demo/demo_data/fraud_input.ssz.bin").unwrap();
    f.write_all(&serialize(&input).unwrap()).unwrap();

    // prepare the beacon state as a file containing preimage oracle chunks
    let mut f = std::fs::File::create("../demo/demo_data/beacon_state_preimages.bin").unwrap();
    let ssz_nodes = test.state.to_merkle_tree().unwrap();
    for (k, v) in ssz_nodes.iter() {
        f.write_all(k).unwrap();
        f.write_all(v).unwrap();
    }
}

// actual tests
#[test]
fn test_finality_rule_3_0() {
    setup();
    let test = ZiplineTestCase::deserialize_from_file("test_finality_rule_3_0.ssz");
    run_test_native(test)
}

#[test]
fn test_finality_rule_3_1() {
    setup();
    let test = ZiplineTestCase::deserialize_from_file("test_finality_rule_3_1.ssz");
    run_test_native(test)
}

#[test]
fn ssz_test_finality_rule_3_0() {
    setup();
    let test = ZiplineTestCase::deserialize_from_file("test_finality_rule_3_0.ssz");
    run_test_native_ssz(test)
}

#[test]
fn ssz_test_finality_rule_3_1() {
    setup();
    let test = ZiplineTestCase::deserialize_from_file("test_finality_rule_3_1.ssz");
    run_test_native_ssz(test)
}

#[test]
fn unicorn_test_finality_rule_3_0() {
    setup();
    let test = ZiplineTestCase::deserialize_from_file("test_finality_rule_3_0.ssz");
    run_test_unicorn(test)
}

#[test]
fn unicorn_test_finality_rule_3_1() {
    setup();
    let test = ZiplineTestCase::deserialize_from_file("test_finality_rule_3_1.ssz");
    run_test_unicorn(test)
}
#[test]
fn ssz_mainnet() {
    setup();
    let gen_path: &str = "./tests/test_files";

    let mut preimages_file = std::fs::File::open(format!("{gen_path}/preimages.bin")).unwrap();
    let mut pre = [0; 64];
    let mut im = [0; 32];
    use std::io::Read;
    let mut preims = alloc::collections::btree_map::BTreeMap::new();

    while preimages_file.read_exact(&mut im).is_ok() {
        preimages_file.read_exact(&mut pre).unwrap();
        preims.insert(im, pre.to_vec());
    }

    let inputs = std::fs::read(format!("{gen_path}/input.ssz")).unwrap();
    let inputs_deser: ZiplineInput<2048, 10000, 256> = deserialize(&inputs).unwrap();

    let input_hash = hash(&serialize(&inputs_deser).unwrap());
    preims.insert(input_hash.try_into().unwrap(), inputs);

    let hashmap_oracle = HashMapOracle::from(preims);
    let reader = SszStateReader::new(
        hashmap_oracle,
        inputs_deser.state_root.as_ref().try_into().unwrap(),
    )
    .unwrap();

    let result = verify::<
        MainnetSpec,
        PatchedSszStateReader<_, MainnetSpec>,
        { spec::MAX_VALIDATORS_PER_COMMITTEE },
        _,
        _,
    >(reader, inputs_deser)
    .unwrap();

    assert!(result);
}

// Ignore because it takes too long to run
#[test]
#[ignore]
fn unicorn_mainnet() {
    setup();
    let program = std::fs::read(MIPS_MAINNET_BIN_PATH).expect("failed to find MIPS binary");
    let gen_path: &str = "./tests/test_files";

    let mut preimages_file = std::fs::File::open(format!("{gen_path}/preimages.bin")).unwrap();
    let mut pre = [0; 64];
    let mut im = [0; 32];
    use std::io::Read;
    let mut preims = alloc::collections::btree_map::BTreeMap::new();
    while preimages_file.read_exact(&mut im).is_ok() {
        preimages_file.read_exact(&mut pre).unwrap();
        preims.insert(im, pre.to_vec());
    }
    let inputs = std::fs::read(format!("{gen_path}/input.ssz")).unwrap();
    let inputs_deser: ZiplineInput<2048, 10000, 256> = deserialize(&inputs).unwrap();

    let input_hash = hash(&serialize(&inputs_deser).unwrap());
    preims.insert(input_hash.clone().try_into().unwrap(), inputs);

    let mut mu = new_cannon_unicorn(UnsyncRam::new(), preims, None, TraceConfig::NewChallenge);

    // let result = start(&mut mu, &program, &input_hash.try_into().unwrap());

    write_program(&mut mu, &program);
    write_input(&mut mu, &input_hash.try_into().unwrap());
    // Run in the emulator!
    let (snapshot, steps, emulation_output) = run(&mut mu, 0);

    let mut expected_result = [0xff_u8; 68];
    expected_result[..4].copy_from_slice(&[0x13, 0x37, 0xf0, 0x0d]);
    expected_result[4..].copy_from_slice(&[0x00; 64]);
    println!("snapshot: {:?}", snapshot);
    println!("steps: {:?}", steps);
    println!("emulation_output: {:?}", emulation_output);
    assert_eq!(emulation_output, expected_result);
}

// Ignoring because we need a better way to handle bellatrix vs capella BeaconStates in DirectStateReader
#[test]
#[ignore]
fn native_mainnet() {
    setup();
    let gen_path: &str = "./tests/test_files";

    let mut preimages_file = std::fs::File::open(format!("{gen_path}/preimages.bin")).unwrap();
    let mut pre = [0; 64];
    let mut im = [0; 32];
    let mut preims = alloc::collections::btree_map::BTreeMap::new();

    while let Ok(_) = preimages_file.read_exact(&mut im) {
        preimages_file.read_exact(&mut pre).unwrap();
        preims.insert(im, pre.to_vec());
    }

    // the mainnet states are capella states rather than bellatrix like the minimal tests
    let beaconstatefile = std::fs::read(format!("{gen_path}/state196726")).unwrap();
    let state: ethereum_consensus:: capella::mainnet::BeaconState =
        deserialize(&beaconstatefile).unwrap();
    let reader = DirectStateReader::new(state);

    let inputs = std::fs::read(format!("{gen_path}/input.ssz")).unwrap();
    let inputs_deser: ZiplineInput<2048, 10000, 256> = deserialize(&inputs).unwrap();

    log::info!(
        "Patched Randaos: {:?}",
        inputs_deser.patches.iter().collect::<Vec<_>>()
    );

    let result = verify::<
        MainnetSpec,
        PatchedDirectStateReader<spec::BeaconState>,
        { spec::MAX_VALIDATORS_PER_COMMITTEE },
        10000,
        256,
    >(reader, inputs_deser)
    .unwrap();
    assert!(result);
}

/////////////////////////////
//  test runners and helpers
/////////////////////////////

/// Given a zipline input and a state this produces a HashMap containing all preimage requests that could be required by verify
/// This allows retrieving:
///   - the input by its hash
///   - any node in the state SSZ merkle tree

fn run_test_native(mut test: ZiplineTestCase) {
    let reader = DirectStateReader::new(test.state.clone());
    let result = verify::<
        SpecTestSpec,
        PatchedDirectStateReader,
        { spec::MAX_VALIDATORS_PER_COMMITTEE },
        1000,
        10,
    >(reader, test.to_input())
    .unwrap();
    assert_eq!(result, test.expected_result);
}

fn run_test_native_ssz(mut test: ZiplineTestCase) {
    let state_root = test.state.hash_tree_root().unwrap();

    let input = test.to_input();
    let oracle_provider = make_test_oracle_provider(&input, &mut test.state);
    let hashmap_oracle = HashMapOracle::from(oracle_provider);

    let reader =
        SszStateReader::new(hashmap_oracle, state_root.as_ref().try_into().unwrap()).unwrap();

    let result = verify::<
        SpecTestSpec,
        PatchedSszStateReader<_, MainnetSpec>,
        { spec::MAX_VALIDATORS_PER_COMMITTEE },
        _,
        _,
    >(reader, input)
    .unwrap();
    assert_eq!(result, test.expected_result);
}

fn run_test_unicorn(mut test: ZiplineTestCase) {
    let program = std::fs::read(MIPS_SPEC_TEST_BIN_PATH).expect("failed to find MIPS binary");
    let input = test.to_input();

    print!("nValidators: {:?}", input.attestations.len());

    let oracle_provider = make_test_oracle_provider(&input, &mut test.state);

    let mut mu = new_cannon_unicorn(
        UnsyncRam::new(),
        oracle_provider,
        None,
        TraceConfig::NewChallenge,
    );

    let input_hash = hash(&serialize(&input).unwrap());
    write_program(&mut mu, &program);
    write_input(&mut mu, &input_hash.try_into().unwrap());
    // Run in the emulator!
    let (snapshot, steps, emulation_output) = run(&mut mu, 0);
    println!("snapshot: {:?}", snapshot);
    println!("steps: {:?}", steps);
    println!("emulation_output: {:?}", emulation_output);

    let mut expected_result = [0xff_u8; 68];
    expected_result[..4].copy_from_slice(&[0x13, 0x37, 0xf0, 0x0d]);
    if test.expected_result {
        expected_result[4..].copy_from_slice(&[0x00; 64]);
    }

    assert_eq!(emulation_output, expected_result);
}

fn make_test_oracle_provider<
    const MAX_COMMITTEE_SIZE: usize,
    const MAX_ATTESTATIONS: usize,
    const MAX_PATCHES: usize,
>(
    input: &ZiplineInput<MAX_COMMITTEE_SIZE, MAX_ATTESTATIONS, MAX_PATCHES>,
    state: &mut spec::BeaconState,
) -> Map<[u8; 32], Vec<u8>> {
    let state_root = state.hash_tree_root().unwrap();
    assert_eq!(
        state_root, input.state_root,
        "incorrect state provided for input"
    );

    let ssz_nodes = state.to_merkle_tree().unwrap();
    let mut provider: Map<[u8; 32], Vec<u8>> =
        Map::from_iter(ssz_nodes.iter().map(|(k, v)| (*k, v.to_vec())));

    let input_bytes = serialize(input).unwrap();
    let input_hash = hash(&input_bytes);
    provider.insert(input_hash.try_into().unwrap(), input_bytes);

    provider
}
