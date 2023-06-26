use alloc::{vec, vec::Vec};
use crypto::hash::H256;
use log::trace;
/// A state patch is a small amount of data that can be applied to an existing BeaconState so that it can be correctly used to
/// compute the shufflings in a future epoch. It is important to note though that this will not actually produce the correct state
/// for the future epoch and that is ok. This is reasonable to do because of the churn limit which prevents an actor submitting a patch
/// which could replace the entire validator set. It can only manipulate a handful of validators which across very few epochs is not enough to significantly
/// reduce the security.
/// This does allow a potential attacker to manipulate the RANDO which we should be aware of. This means an attacker would be able to insert malicious validators
/// and get them into the same committee. Even with this ability there is still only a minor impact to the economic security.
/// TODO: Calculate exactly how much the security decays per epoch
use ssz_rs::prelude::*;
use zipline_spec::Spec;
// these are just temp for now, should read from a zipline spec
const MAX_ACTIVATIONS: usize = 256;
const MAX_EXITS: usize = 256;

#[derive(Clone, Debug, Default, SimpleSerialize, PartialEq)]
pub struct StatePatch {
    pub epoch: u64, // epoch this patches up to. A single patch should only increment the epoch by 1
    pub activations: List<u32, MAX_ACTIVATIONS>,
    pub exits: List<u32, MAX_EXITS>,
    pub n_deposits_processed: u32,
    pub randao_next: H256, // randao value needed to compute the shuffling in the NEXT epoch
}

impl StatePatch {
    pub fn is_valid<S: Spec>(&self, n_active_validators: u32) -> bool {
        let churn_limit = get_validator_churn_limit::<S>(n_active_validators);
        if (self.activations.len() as u32) > churn_limit || (self.exits.len() as u32) > churn_limit
        {
            trace!("patch activations or exits exceeds churn limit");
            return false;
        }

        if self.n_deposits_processed > S::max_deposits() * (S::slots_per_epoch() as u32) {
            trace!("patch n_deposits_processed exceeds max");
            return false;
        }

        true
    }
}

// https://eth2book.info/bellatrix/part3/helper/accessors/#get_validator_churn_limit
fn get_validator_churn_limit<S: Spec>(n_active_validators: u32) -> u32 {
    core::cmp::max(S::min_per_epoch_churn_limit(), n_active_validators) / S::churn_limit_quotient()
}
