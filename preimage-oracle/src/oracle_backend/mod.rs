use crate::error::PreimageOracleError;
use core::borrow::Borrow;

#[cfg(feature = "fs-oracle")]
pub mod filesystem_oracle;

#[cfg(feature = "hashmap-oracle")]
pub mod hashmap_oracle;

/// A PreimageOracle allows you to retrieve the pre-image of a hash.
/// This allows for a generic backend. For example, you may choose to
/// store your preimage mappings in a HashMap, or in the filesystem, or
/// a combination of both.
pub trait PreimageOracle<TImage>
where
    TImage: Borrow<TImage>,
{
    // Accepts a function that will be called with the data that is the pre-image of `hash`
    // This is nice and safe as it ensures passed function has exclusive access to the
    // pre-image oracle memory for the duration of its call but it also prevents a copy
    fn map<T, F>(&self, key: TImage, f: F) -> Result<T, PreimageOracleError>
    where
        F: FnOnce(&[u8]) -> T;

    // Request a preimage from the oracle.
    //
    // This will cache the preimage Data and returned slice is valid until the end of the program.
    // This causes the data to be copied to the heap
    fn get_cached(&self, hash: TImage) -> Option<&[u8]>;
}
