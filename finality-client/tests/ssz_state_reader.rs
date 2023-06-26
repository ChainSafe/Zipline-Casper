use direct_state_reader::DirectStateReader;
use ethereum_consensus::bellatrix::mainnet as spec;
use ethereum_consensus::state_transition::Context;
use preimage_oracle::hashmap_oracle::HashMapOracle;
use preimage_oracle::PreimageOracle;
use ssz_rs::prelude::*;
use std::collections::BTreeMap as Map;
use std::sync::Once;
use test_utils::load_snappy_ssz;
use zipline_finality_client::{
    ssz_state_reader::SszStateReader,
    state_reader::{StateReadError, StateReader},
};
use zipline_spec::{MainnetSpec as S, Spec};

mod direct_state_reader;

macro_rules! test_path {
    ($t:literal) => {
        concat!(
            "../consensus-spec-tests/tests/mainnet/bellatrix/epoch_processing/registry_updates/pyspec_tests/",
            $t
        )
    };
}

static INIT: Once = Once::new();

/// Setup function that is only run once, even if called multiple times.
fn setup() {
    INIT.call_once(|| {
        let _ = env_logger::builder().is_test(true).try_init();
    });
}

fn same_randao<T: PreimageOracle<[u8; 32]>, S: Spec>(
    direct_state_reader: &DirectStateReader,
    ssz_state_reader: &SszStateReader<T, S>,
) -> Result<(), StateReadError> {
    for i in 0..S::epochs_per_historical_vector() {
        assert_eq!(
            direct_state_reader.get_randao::<S>(i as u64)?,
            ssz_state_reader.get_randao::<S>(i as u64)?
        );
    }
    Ok(())
}

fn same_validator_count<T: PreimageOracle<[u8; 32]>, S: Spec>(
    direct_state_reader: &DirectStateReader,
    ssz_state_reader: &SszStateReader<T, S>,
) -> Result<(), StateReadError> {
    assert_eq!(
        direct_state_reader.get_validator_count()?,
        ssz_state_reader.get_validator_count()?
    );
    Ok(())
}

fn same_active_validators<T: PreimageOracle<[u8; 32]>, S: Spec>(
    direct_state_reader: &DirectStateReader,
    ssz_state_reader: &SszStateReader<T, S>,
    epoch: u64,
) -> Result<(), StateReadError> {
    assert_eq!(
        direct_state_reader.get_active_validator_indices(epoch)?,
        ssz_state_reader.get_active_validator_indices(epoch)?
    );
    Ok(())
}

fn same_validator_activation_and_exits<T: PreimageOracle<[u8; 32]>, S: Spec>(
    direct_state_reader: &DirectStateReader,
    ssz_state_reader: &SszStateReader<T, S>,
    n_validators: usize,
) -> Result<(), StateReadError> {
    for i in 0..n_validators {
        assert_eq!(
            direct_state_reader.get_validator_activation_and_exit_epochs(i)?,
            ssz_state_reader.get_validator_activation_and_exit_epochs(i)?
        );
    }
    Ok(())
}

#[test]
fn test_equivalency() {
    setup();

    let mut state: spec::BeaconState =
        load_snappy_ssz(test_path!("add_to_activation_queue/pre.ssz_snappy"))
            .expect("Failed to load test state");

    let epoch = spec::compute_epoch_at_slot(state.slot, &Context::for_mainnet());

    let direct_state_reader = DirectStateReader::new(state.clone());

    let root = state.hash_tree_root().unwrap();
    let preim = state.to_merkle_tree().unwrap();
    let preim: Map<[u8; 32], [u8; 64]> = Map::from_iter(preim.iter().map(|(k, v)| (*k, *v)));

    let hashmap_oracle = HashMapOracle::from(preim);
    let ssz_state_reader: SszStateReader<_, S> =
        SszStateReader::new(hashmap_oracle, root.as_ref().try_into().unwrap()).unwrap();

    // check all the state read methods produce the same result
    assert!(same_randao(&direct_state_reader, &ssz_state_reader).is_ok());
    assert!(same_validator_count(&direct_state_reader, &ssz_state_reader).is_ok());
    assert!(same_validator_activation_and_exits(
        &direct_state_reader,
        &ssz_state_reader,
        state.validators.len(),
    )
    .is_ok());
    assert!(same_active_validators(&direct_state_reader, &ssz_state_reader, epoch).is_ok());
}
