use crate::state_patch::StatePatch;
use crypto::bls::{BlsError, PublicKey};
use crypto::hash::H256;
use log::warn;
use preimage_oracle::error::PreimageOracleError;

use alloc::vec::Vec;
use zipline_spec::Spec;
#[derive(Debug)]
pub enum StateReadError {
    UntrustedState,
    Randao(usize),
    ActiveValidators,
    BlsError(BlsError),
    ValidatorRetrieval,
    RootUnknown,
    PreimageOracleError(PreimageOracleError),
}

impl From<BlsError> for StateReadError {
    fn from(value: BlsError) -> Self {
        Self::BlsError(value)
    }
}

impl From<PreimageOracleError> for StateReadError {
    fn from(e: PreimageOracleError) -> Self {
        Self::PreimageOracleError(e)
    }
}

// Something able to read requisite values from the state
// in the prod implementation this will make use of the pre-image oracle to retrieve state
// data. In testing we can mock it out with more direct read access to a state object
pub trait StateReader {
    fn root(&self) -> Result<H256, StateReadError>;

    fn get_validator_count(&self) -> Result<usize, StateReadError>;

    // can override if there is already a cached copy of this available
    fn get_active_validator_indices(&self, epoch: u64) -> Result<Vec<usize>, StateReadError> {
        Ok((0_usize..self.get_validator_count()?)
            .filter(|validator_index| {
                // TODO: Remove this unwrap
                let (activation, exit) = self
                    .get_validator_activation_and_exit_epochs(*validator_index)
                    .unwrap();
                activation <= epoch && epoch < exit
            })
            .collect())
    }

    fn get_total_active_balance(&self, epoch: u64) -> Result<u64, StateReadError> {
        self.aggregate_validator_keys_and_balance(&self.get_active_validator_indices(epoch)?)
            .map(|x| x.1)
    }

    // the the randao value for shuffling a particular epoch
    // can only look 1 epoch into the future
    fn get_randao<S: Spec>(&self, epoch: u64) -> Result<[u8; 32], StateReadError>;

    // return the single aggregate signature and combined active balance obtained from validators indexed by the given indices
    // not that validators indices never change so this is valid even if using a newer state than the current epoch
    fn aggregate_validator_keys_and_balance(
        &self,
        indices: &[usize],
    ) -> Result<(Vec<PublicKey>, u64), StateReadError>;

    fn get_validator_activation_and_exit_epochs(
        &self,
        validator_index: usize,
    ) -> Result<(u64, u64), StateReadError>;
}

/// A patched state reader adds patches to an underlying state. It also implements StateReader and
/// can be used anywhere a StateReader is expected
pub trait PatchedStateReader {
    type SR: StateReader;

    fn new(state_reader: Self::SR) -> Self;

    fn with_patch(self, patch: StatePatch) -> Self;

    fn patches(&self) -> &[StatePatch];

    fn reader(&self) -> &Self::SR;
}

impl<T> StateReader for T
where
    T: PatchedStateReader,
{
    fn root(&self) -> Result<H256, StateReadError> {
        if self.patches().is_empty() {
            self.reader().root()
        } else {
            Err(StateReadError::RootUnknown)
        }
    }

    fn get_active_validator_indices(&self, epoch: u64) -> Result<Vec<usize>, StateReadError> {
        let indices = self.reader().get_active_validator_indices(epoch);
        indices
    }

    fn get_validator_count(&self) -> Result<usize, StateReadError> {
        let mut count = self.reader().get_validator_count()?;
        for patch in self.patches() {
            count += patch.n_deposits_processed as usize;
        }
        Ok(count)
    }

    // the randao value for computing shuffling a particular epoch
    fn get_randao<S: Spec>(&self, epoch: u64) -> Result<[u8; 32], StateReadError> {
        // see if we can use a patch randao to see
        // further into the future
        for patch in self.patches() {
            if patch.epoch + 1 == epoch {
                log::info!(
                    "Using patch {} for randao for epoch {}, randao: {:?}",
                    patch.epoch,
                    epoch,
                    patch.randao_next
                );
                return Ok(patch.randao_next);
            }
        }
        warn!("No patch randao for epoch {}, use state reader", epoch);
        // otherwise just try and use the state reader
        self.reader().get_randao::<S>(epoch)
    }

    fn aggregate_validator_keys_and_balance(
        &self,
        indices: &[usize],
    ) -> Result<(Vec<PublicKey>, u64), StateReadError> {
        // pass straight to the reader. Patch cannot change this
        self.reader().aggregate_validator_keys_and_balance(indices)
    }

    fn get_validator_activation_and_exit_epochs(
        &self,
        validator_index: usize,
    ) -> Result<(u64, u64), StateReadError> {
        let (mut activation, mut exit) = self
            .reader()
            .get_validator_activation_and_exit_epochs(validator_index)?;
        // replace any activations/exists with their most recent patch updates if any
        for patch in self.patches() {
            if patch
                .activations
                .iter()
                .filter(|vi| **vi == validator_index.try_into().unwrap())
                .last()
                .is_some()
            {
                log::info!(
                    "validator {} Patched! activation: {} exit: {}",
                    validator_index,
                    activation,
                    exit
                );
                activation = patch.epoch;
            }
            if patch
                .exits
                .iter()
                .filter(|vi| **vi == validator_index.try_into().unwrap())
                .last()
                .is_some()
            {
                log::info!(
                    "validator {} Patched! activation: {} exit: {}",
                    validator_index,
                    activation,
                    exit
                );
                exit = patch.epoch;
            }
        }

        Ok((activation, exit))
    }
}
