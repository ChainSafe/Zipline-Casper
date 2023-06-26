use crate::attestation::{Attestation, Checkpoint};
use crate::state_patch::StatePatch;
use alloc::{vec, vec::Vec};
use crypto::hash::H256;
use ssz_rs::prelude::*;

/// An SSZ container capturing all of the inputs required for one call to 'verify'
#[derive(Clone, Debug, Default, SimpleSerialize, PartialEq)]
pub struct ZiplineInput<
    const MAX_COMMITTEE_SIZE: usize,
    const MAX_ATTESTATIONS: usize,
    const MAX_PATCHES: usize,
> {
    pub trusted_cp: Checkpoint,
    pub candidate_cp: Checkpoint,
    pub state_root: H256, // state root beacon state as of the trusted_cp block
    pub patches: List<StatePatch, MAX_PATCHES>,
    pub attestations: List<Attestation<MAX_COMMITTEE_SIZE>, MAX_ATTESTATIONS>,
    pub state_proof: List<H256, 3>, // SSZ proof that the state root is contained in the trusted_cp block
}

impl<const MAX_COMMITTEE_SIZE: usize, const MAX_ATTESTATIONS: usize, const MAX_PATCHES: usize>
    ZiplineInput<MAX_COMMITTEE_SIZE, MAX_ATTESTATIONS, MAX_PATCHES>
{
    /// Deserialize from SSZ encoded bytes
    pub fn from_ssz_bytes(bytes: &[u8]) -> Self {
        <Self as ssz_rs::Deserialize>::deserialize(bytes).unwrap()
    }
}
