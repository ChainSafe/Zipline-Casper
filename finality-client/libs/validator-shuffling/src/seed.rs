use crypto::hash::{hash_fixed, H256};
use zipline_spec::Spec;

/// Generate a seed for the given `epoch`.
pub fn committee_shuffle_seed_from_randao<T: Spec>(randao_mix: H256, epoch: usize) -> H256 {
    let domain_bytes = int_to_bytes4(T::domain_beacon_attester());
    let epoch_bytes = (epoch as u64).to_le_bytes().to_vec();

    const NUM_DOMAIN_BYTES: usize = 4;
    const NUM_EPOCH_BYTES: usize = 8;
    const MIX_OFFSET: usize = NUM_DOMAIN_BYTES + NUM_EPOCH_BYTES;
    const NUM_MIX_BYTES: usize = 32;

    let mut preimage = [0; NUM_DOMAIN_BYTES + NUM_EPOCH_BYTES + NUM_MIX_BYTES];
    preimage[0..NUM_DOMAIN_BYTES].copy_from_slice(&domain_bytes);
    preimage[NUM_DOMAIN_BYTES..MIX_OFFSET].copy_from_slice(&epoch_bytes);
    preimage[MIX_OFFSET..].copy_from_slice(&randao_mix);

    hash_fixed(&preimage)
}

// returns the correct index into the state rando array for a given epoch
// see https://github.com/ethereum/annotated-spec/blob/master/phase0/beacon-chain.md#get_seed
pub fn get_randao_index<T: Spec>(epoch: u64) -> usize {
    // (epoch + EPOCHS_PER_HISTORICAL_VECTOR - MIN_SEED_LOOKAHEAD - 1) % EPOCHS_PER_HISTORICAL_VECTOR
    ((epoch as usize) + T::epochs_per_historical_vector() - T::min_seed_lookahead() - 1)
        % T::epochs_per_historical_vector()
}

/// Returns `int` as little-endian bytes with a length of 4.
pub fn int_to_bytes4(int: u32) -> [u8; 4] {
    int.to_le_bytes()
}
