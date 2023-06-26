use core::fmt::Debug;

use alloc::vec;
use alloc::vec::Vec;
use crypto::bls::BLS_SIGNATURE_BYTES_LEN;
use crypto::hash::H256;
use ssz_rs::prelude::*;

#[derive(Clone, Debug, Default, SimpleSerialize, PartialEq)]

pub struct Attestation<const MAX_COMMITTEE_SIZE: usize> {
    pub aggregation_bits: Bitlist<MAX_COMMITTEE_SIZE>,
    pub data: AttestationData,
    pub signature: Vector<u8, BLS_SIGNATURE_BYTES_LEN>,
}

#[derive(Clone, Debug, Default, SimpleSerialize, PartialEq)]
pub struct AttestationData {
    pub slot: u64,
    pub index: u64,
    pub beacon_block_root: H256,
    pub source: Checkpoint, // these are indices into the checkpoints list
    pub target: Checkpoint, // in the root SuperAttestation
}

impl AttestationData {
    pub fn link(&self) -> CasperLink {
        CasperLink {
            source: self.source,
            target: self.target,
        }
    }
}
#[derive(Default, Copy, Clone, SimpleSerialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Checkpoint {
    pub epoch: u64,
    pub root: H256,
}

impl Debug for Checkpoint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Checkpoint")
            .field("epoch", &self.epoch)
            .finish()
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CasperLink {
    pub source: Checkpoint,
    pub target: Checkpoint,
}
