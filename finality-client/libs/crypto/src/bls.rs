use alloc::string::{String, ToString};
use alloc::vec::Vec;
use blst::min_pk as bls;
use blst::BLST_ERROR;

// domain string, must match what is used in signing. This one should be good for beacon chain
const DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

pub const BLS_SIGNATURE_BYTES_LEN: usize = 96;

#[derive(Debug)]
pub enum BlsError {
    InvalidSignature,
    Other(String),
}

impl From<BLST_ERROR> for BlsError {
    fn from(value: BLST_ERROR) -> Self {
        assert!(value != BLST_ERROR::BLST_SUCCESS);
        Self::Other(format_args!("{:?}", value).to_string())
    }
}

impl From<String> for BlsError {
    fn from(value: String) -> Self {
        Self::Other(value)
    }
}
#[derive(Clone, Debug)]
pub struct PublicKey(bls::PublicKey);

impl PublicKey {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BlsError> {
        Ok(PublicKey(bls::PublicKey::from_bytes(bytes).unwrap()))
    }

    pub fn to_bytes(&self) -> [u8; 48] {
        self.0.to_bytes()
    }
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: PublicKey) -> Self {
        let mut aggkey = bls::AggregatePublicKey::from_public_key(&self.0);
        aggkey.add_public_key(&other.0, false).unwrap();
        Self(aggkey.to_public_key())
    }
}

pub struct Signature(bls::Signature);

impl Signature {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BlsError> {
        Ok(Signature(bls::Signature::from_bytes(bytes)?))
    }
    pub fn to_bytes(&self) -> [u8; 96] {
        self.0.to_bytes()
    }
}

pub fn verify_signature(
    public_key: &PublicKey,
    msg: &[u8],
    signature: &Signature,
) -> Result<(), BlsError> {
    let res = signature.0.verify(true, msg, DST, &[], &public_key.0, true);
    if res == BLST_ERROR::BLST_SUCCESS {
        Ok(())
    } else {
        Err(BlsError::InvalidSignature)
    }
}

pub fn fast_aggregate_verify(
    public_keys: &[PublicKey],
    msg: &[u8],
    signature: &Signature,
) -> Result<(), BlsError> {
    let public_keys = public_keys.iter().map(|k| &k.0).collect::<Vec<_>>();

    let res = signature
        .0
        .fast_aggregate_verify(true, msg, DST, &public_keys);
    if res == BLST_ERROR::BLST_SUCCESS {
        Ok(())
    } else {
        Err(BlsError::InvalidSignature)
    }
}

// This is verification for the case where multiple messages were signed and an aggregate signature obtained by aggregating the resulting signatures.
// TODO: BLST won't do this out of the box but it should be fairly easy to implement with their lower level operations
pub fn multi_message_verify(
    _messages: &[&[u8]],
    _public_key: &PublicKey,
    _signature: &Signature,
) -> Result<(), BlsError> {
    Ok(())
}
