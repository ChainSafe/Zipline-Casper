use crate::attestation::Attestation;
use alloc::{vec, vec::Vec};
use crypto::bls::{fast_aggregate_verify, BlsError, PublicKey, Signature};
use ssz_rs::prelude::*;
use zipline_spec::Spec;
pub type Domain = [u8; 32];
pub type Root = Node;

#[derive(Debug)]
pub enum SigningError {
    Merkleization,
    BlsError(BlsError),
}

impl From<MerkleizationError> for SigningError {
    fn from(_value: MerkleizationError) -> Self {
        SigningError::Merkleization
    }
}

impl From<BlsError> for SigningError {
    fn from(value: BlsError) -> Self {
        Self::BlsError(value)
    }
}

#[derive(Default, Debug, SimpleSerialize)]
pub struct SigningData {
    pub object_root: Root,
    pub domain: Domain,
}

pub fn compute_signing_root<T: SimpleSerialize>(
    ssz_object: &mut T,
    domain: Domain,
) -> Result<Root, SigningError> {
    let object_root = ssz_object.hash_tree_root()?;

    let mut s = SigningData {
        object_root,
        domain,
    };
    Ok(s.hash_tree_root()?)
}

pub fn verify_signed_attestation<S: Spec, const MAX_COMMITTEE_SIZE: usize>(
    a: &mut Attestation<MAX_COMMITTEE_SIZE>,
    public_keys: &[PublicKey],
) -> Result<(), SigningError> {
    let domain = S::beacon_attester_signing_domain();
    let signing_root = compute_signing_root(&mut a.data, domain)?;
    fast_aggregate_verify(
        public_keys,
        signing_root.as_ref(),
        &Signature::from_bytes(&a.signature)?,
    )
    .map_err(Into::into)
}
