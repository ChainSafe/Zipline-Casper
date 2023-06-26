use crate::state_patch::StatePatch;
use crate::state_reader::{PatchedStateReader, StateReadError, StateReader};
use alloc::vec::Vec;
use core::marker::PhantomData;
use crypto::bls::PublicKey;
use crypto::hash::H256;
use preimage_oracle::SszOracle;
use typenum::Unsigned;
use validator_shuffling::get_randao_index;
use zipline_spec::Spec;

pub(crate) struct ValidatorInfo {
    pub pubkey: PublicKey,
    pub effective_balance: u64,
    pub activation_epoch: u64,
    pub exit_epoch: u64,
}
pub struct SszStateReader<TSsz: SszOracle, TSpec> {
    oracle: TSsz,
    root: H256,
    spec: PhantomData<TSpec>,
    validator_cache: Vec<ValidatorInfo>,
}

impl<TSsz: SszOracle, TSpec: Spec> SszStateReader<TSsz, TSpec> {
    fn build_validator_cache(&mut self) -> Result<(), StateReadError> {
        log::debug!("Starting to build validator cache");
        let start_index = TSpec::Validators0Gindex::to_u64();

        let val_bal_gindex = TSpec::EffectiveBalanceGindex::to_u64();
        let val_key_gindex = TSpec::PubkeyGindex::to_u64();
        let activation_epoch_gindex = TSpec::ActivationEpochGindex::to_u64();
        let exit_epoch_gindex = TSpec::ExitEpochGindex::to_u64();

        let count = self.get_validator_count().unwrap();

        let the_iter = preimage_oracle::ssz_retrieval::iterate_nodes_at_depth(
            &self.oracle,
            self.root,
            46,
            start_index,
            count as u64,
        )
        .enumerate()
        .map(|(i, val_i_root)| {
            if i % 5000 == 0 {
                log::debug!("Building Validator cache {}/{}", i, count);
            }
            let mut pk: [u8; 48] = [0; 48];
            self.oracle
                .map_chunk(val_i_root, val_key_gindex, |k_root| {
                    self.oracle.map(*k_root, |k| {
                        pk.copy_from_slice(&k[0..48]);
                    })
                })??;

            let balance = self
                .oracle
                .map_as_uint64(val_i_root, val_bal_gindex, |x| x)?;
            let activation_epoch =
                self.oracle
                    .map_as_uint64(val_i_root, activation_epoch_gindex, |x| x)?;
            let exit_epoch = self
                .oracle
                .map_as_uint64(val_i_root, exit_epoch_gindex, |x| x)?;

            Ok(ValidatorInfo {
                pubkey: PublicKey::from_bytes(&pk)?,
                effective_balance: balance,
                activation_epoch,
                exit_epoch,
            })
        });
        self.validator_cache = the_iter.collect::<Result<Vec<_>, StateReadError>>()?;
        Ok(())
    }
    pub fn new(oracle: TSsz, root: H256) -> Result<Self, StateReadError> {
        let mut reader = Self {
            oracle,
            root,
            spec: PhantomData,
            validator_cache: Vec::default(),
        };

        reader.build_validator_cache()?;

        Ok(reader)
    }
}

impl<TSsz: SszOracle, TSpec: Spec> StateReader for SszStateReader<TSsz, TSpec> {
    fn root(&self) -> Result<H256, StateReadError> {
        Ok(self.root)
    }

    fn get_validator_count(&self) -> Result<usize, StateReadError> {
        log::trace!("SszStateReader: get_validator_count");
        let validator_count_gindex = TSpec::ValidatorsLengthGindex::to_u64();
        let validator_count =
            self.oracle
                .map_as_uint64(self.root, validator_count_gindex, |x| x)?;
        Ok(validator_count as usize)
    }

    fn get_randao<S: Spec>(&self, epoch: u64) -> Result<[u8; 32], StateReadError> {
        log::trace!("SszStateReader get_randao({})", epoch);
        let randao_index = get_randao_index::<S>(epoch);
        let randao_mixes_0_gindex = TSpec::RandaoMixes0Gindex::to_u64(); // TODO
        let randao_mixes_gindex = randao_mixes_0_gindex + randao_index as u64;
        let randao_mix = self.oracle.copy_chunk(self.root, randao_mixes_gindex)?;

        Ok(randao_mix)
    }

    fn get_active_validator_indices(&self, epoch: u64) -> Result<Vec<usize>, StateReadError> {
        log::trace!("SszStateReader get_active_validator_indices({})", epoch);
        let start_index = TSpec::Validators0Gindex::to_u64();
        let activation_epoch_gindex = TSpec::ActivationEpochGindex::to_u64();
        let exit_epoch_gindex = TSpec::ExitEpochGindex::to_u64();

        let count = self.get_validator_count()?;
        Ok(preimage_oracle::ssz_retrieval::iterate_nodes_at_depth(
            &self.oracle,
            self.root,
            46,
            start_index,
            count as u64,
        )
        .enumerate()
        .filter_map(|val_i_root| {
            if val_i_root.0 % 100000 == 0 {
                log::trace!(
                    "SszStateReader get_active_validator_indices validator {}",
                    val_i_root.0
                );
            }
            let activation = self
                .oracle
                .map_as_uint64(val_i_root.1, activation_epoch_gindex, |x| x)
                .unwrap();
            let exit = self
                .oracle
                .map_as_uint64(val_i_root.1, exit_epoch_gindex, |x| x)
                .unwrap();
            if (activation <= epoch) && (epoch < exit) {
                Some(val_i_root.0)
            } else {
                None
            }
        })
        .collect())
    }

    fn aggregate_validator_keys_and_balance(
        &self,
        indices: &[usize],
    ) -> Result<(Vec<PublicKey>, u64), StateReadError> {
        log::trace!(
            "SszStateReader aggregate_validator_keys_and_balance len: {}",
            indices.len()
        );
        let mut pk_acc: Vec<PublicKey> = Vec::with_capacity(indices.len());
        let mut bal_acc = 0;
        for idx in indices.iter() {
            let ValidatorInfo {
                pubkey: pk,
                effective_balance: bal,
                ..
            } = &self.validator_cache[*idx];
            pk_acc.push(pk.clone());
            bal_acc += bal;
        }
        Ok((pk_acc, bal_acc))
    }

    fn get_validator_activation_and_exit_epochs(
        &self,
        validator_index: usize,
    ) -> Result<(u64, u64), StateReadError> {
        Ok((
            self.validator_cache[validator_index].activation_epoch,
            self.validator_cache[validator_index].exit_epoch,
        ))
    }
}

pub struct PatchedSszStateReader<TSsz: SszOracle, TSpec> {
    state_reader: SszStateReader<TSsz, TSpec>,
    patches: Vec<StatePatch>,
}

impl<TSsz: SszOracle, TSpec: Spec> PatchedStateReader for PatchedSszStateReader<TSsz, TSpec> {
    type SR = SszStateReader<TSsz, TSpec>;

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
