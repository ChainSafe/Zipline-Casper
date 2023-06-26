use sha2::Digest;
pub use sha2::Sha256 as Context;

/// Length of a SHA256 hash in bytes.
pub const HASH_LEN: usize = 32;

pub type H256 = [u8; HASH_LEN];
use alloc::vec::Vec;
use core::convert::Into;
use core::iter::IntoIterator;
use core::iter::Iterator;
/// Returns the digest of `input` using the best available implementation.
pub fn hash(input: &[u8]) -> Vec<u8> {
    Sha2CrateImpl {}.hash(input)
}

/// Hash function returning a fixed-size array (to save on allocations).
/// This is the preferred way to hash
pub fn hash_fixed(input: &[u8]) -> [u8; HASH_LEN] {
    Sha2CrateImpl {}.hash_fixed(input)
}

/// Compute the hash of two slices concatenated.
pub fn hash_concat(h1: &[u8], h2: &[u8]) -> [u8; HASH_LEN] {
    let mut ctx = <sha2::Sha256 as Sha256Context>::new();
    Sha256Context::update(&mut ctx, h1);
    Sha256Context::update(&mut ctx, h2);
    Sha256Context::finalize(ctx)
}

/// Context trait for abstracting over implementation contexts.
pub trait Sha256Context {
    fn new() -> Self;

    fn update(&mut self, bytes: &[u8]);

    fn finalize(self) -> [u8; HASH_LEN];
}

/// Top-level trait for Sha256 hashing
pub trait Sha256 {
    type Context: Sha256Context;

    fn hash(&self, input: &[u8]) -> Vec<u8>;

    fn hash_fixed(&self, input: &[u8]) -> [u8; HASH_LEN];
}

/// Implementation of SHA256 using the `sha2` crate.
// We can switch this out with other impls if they are found to be faster on MIPS
struct Sha2CrateImpl;

impl Sha256Context for sha2::Sha256 {
    fn new() -> Self {
        sha2::Digest::new()
    }

    fn update(&mut self, bytes: &[u8]) {
        sha2::Digest::update(self, bytes)
    }

    fn finalize(self) -> [u8; HASH_LEN] {
        sha2::Digest::finalize(self).into()
    }
}

impl Sha256 for Sha2CrateImpl {
    type Context = sha2::Sha256;

    fn hash(&self, input: &[u8]) -> Vec<u8> {
        Self::Context::digest(input).into_iter().collect()
    }

    fn hash_fixed(&self, input: &[u8]) -> [u8; HASH_LEN] {
        Self::Context::digest(input).into()
    }
}
