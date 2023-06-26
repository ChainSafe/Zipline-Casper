#![cfg(test)]

use crate::{
    committee_shuffle_seed_from_randao, get_randao_index, CommitteeCache as OurCommitteeCache,
    ShuffleData,
};
use zipline_spec::{MinimalSpec, Spec};

use testing_utils::*;

fn shuffle_data_from_state<T: EthSpec, S: Spec>(
    state: &BeaconState<T>,
    spec: &ChainSpec,
    epoch: Epoch,
) -> Result<ShuffleData, Error> {
    // calculate the index ourselves
    let index = get_randao_index::<S>(epoch.as_usize());
    // get the mix from the state rando mix array
    let mix = state
        .randao_mixes()
        .get(index)
        .ok_or(Error::EpochOutOfBounds)?
        .0;
    // use this mix to calculate the seed
    let seed = committee_shuffle_seed_from_randao::<S>(mix, epoch.as_usize());
    Ok(ShuffleData {
        seed,
        len_total_validators: state.validators().len(),
        active_validator_indices: state.get_active_validator_indices(epoch, spec)?,
    })
}

#[tokio::test]
async fn ensure_same_as_lighthouse() {
    let num_validators = MinimalEthSpec::minimum_validator_count() * 2;
    let epoch = Epoch::new(6);
    let slot = epoch.start_slot(MinimalEthSpec::slots_per_epoch());
    let spec = &MinimalEthSpec::default_spec();

    let mut state = new_state::<MinimalEthSpec>(num_validators, slot).await;
    assert_eq!(state.current_epoch(), epoch);

    // add some random random mixes to the state
    let distinct_hashes: Vec<Hash256> = (0..MinimalEthSpec::epochs_per_historical_vector())
        .map(|i| Hash256::from_low_u64_be(i as u64))
        .collect();
    *state.randao_mixes_mut() = FixedVector::from(distinct_hashes);

    // We can initialize the committee cache at recent epochs in the past, and one epoch into the
    // future. Use each of these and compare our committee cache to the lighthouse implementation
    for e in (0..=epoch.as_u64() + 1).map(Epoch::new) {
        let lighthouse_cache = CommitteeCache::initialized(&state, e, spec)
            .unwrap_or_else(|_| panic!("failed to construct lighthouse cache at epoch {}", e));

        let shuffle_data = shuffle_data_from_state::<_, MinimalSpec>(&state, spec, e)
            .expect("Could not derive shuffle data from state");

        let our_cache = OurCommitteeCache::initialized::<MinimalSpec>(shuffle_data, e.into())
            .unwrap_or_else(|_| panic!("failed to construct our cache at epoch {}", e));

        assert_eq!(lighthouse_cache.shuffling(), our_cache.shuffling(),);
    }
}
