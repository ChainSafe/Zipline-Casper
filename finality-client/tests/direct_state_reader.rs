use crypto::bls::PublicKey;
use ethereum_consensus::bellatrix::get_active_validator_indices;
use ethereum_consensus::bellatrix::mainnet::BeaconState;
use ssz_rs::prelude::*;
use validator_shuffling::get_randao_index;
use zipline_finality_client::state_patch::StatePatch;
use zipline_finality_client::state_reader::{PatchedStateReader, StateReadError, StateReader};
use zipline_spec::Spec;

// holds a eth-consensus BeaconState object which it can read from directly
// useful for testing only
#[derive(Default, SimpleSerialize)]
pub struct DirectStateReader {
    state: BeaconState,
}

pub struct PatchedDirectStateReader {
    state_reader: DirectStateReader,
    patches: Vec<StatePatch>,
}

impl PatchedStateReader for PatchedDirectStateReader {
    type SR = DirectStateReader;

    fn new(state_reader: Self::SR) -> Self {
        Self {
            state_reader,
            patches: Vec::new(),
        }
    }

    fn with_patch(mut self, patch: StatePatch) -> Self {
        self.patches.push(patch);
        self
    }

    fn patches(&self) -> &[StatePatch] {
        &self.patches
    }

    fn reader(&self) -> &Self::SR {
        &self.state_reader
    }
}

impl DirectStateReader {
    pub fn new(state: BeaconState) -> Self {
        Self { state }
    }
}

impl StateReader for DirectStateReader {
    fn root(&self) -> Result<crypto::hash::H256, StateReadError> {
        let mut s = self.state.clone(); // needed cos hash_tree_root needs a mutable ref for some reason..
        Ok(s.hash_tree_root().unwrap().as_ref().try_into().unwrap())
    }

    fn get_validator_count(&self) -> Result<usize, StateReadError> {
        Ok(self.state.validators.len())
    }

    fn get_active_validator_indices(&self, epoch: u64) -> Result<Vec<usize>, StateReadError> {
        Ok(get_active_validator_indices(&self.state, epoch))
    }

    fn get_randao<S: Spec>(&self, epoch: u64) -> Result<[u8; 32], StateReadError> {
        let index = get_randao_index::<S>(epoch);
        let mut result = [0; 32];
        result.copy_from_slice(self.state.randao_mixes[index].as_ref());
        Ok(result)
    }

    fn aggregate_validator_keys_and_balance(
        &self,
        indices: &[usize],
    ) -> Result<(Vec<PublicKey>, u64), StateReadError> {
        // build the iterator over keys but don't collect it yet
        let keys_balances = indices.iter().map(|index| {
            let validator = self.state.validators[*index].clone();
            (
                PublicKey::from_bytes(&validator.public_key).unwrap(),
                validator.effective_balance,
            )
        });

        // aggregate the rest on top of that one
        let (aggregate, balance) =
            keys_balances.fold((vec![], 0), |(mut keys, total_balance), (key, balance)| {
                keys.push(key);
                (keys, total_balance + balance)
            });
        Ok((aggregate, balance))
    }

    fn get_validator_activation_and_exit_epochs(
        &self,
        validator_index: usize,
    ) -> Result<(u64, u64), StateReadError> {
        let validator = self.state.validators[validator_index].clone();
        Ok((validator.activation_epoch, validator.exit_epoch))
    }
}
