use crate::error::PreimageOracleError;
use crate::oracle_backend::PreimageOracle;
use crate::H256;
use alloc::collections::btree_map::BTreeMap as Map;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::fmt::Debug;
use core::hash::Hash;
/// Stores all the preimages into a HashMap on the heap.
#[derive(Default, Clone)]
pub struct HashMapOracle<TImage>(Map<TImage, Vec<u8>>);

impl<TImage, const C: usize> From<Map<TImage, [u8; C]>> for HashMapOracle<TImage>
where
    TImage: Borrow<TImage> + PartialEq + Eq + Hash + Ord,
{
    fn from(hm: Map<TImage, [u8; C]>) -> Self {
        Self(hm.into_iter().map(|(k, v)| (k, v.to_vec())).collect())
    }
}

impl<TImage> From<Map<TImage, Vec<u8>>> for HashMapOracle<TImage> {
    fn from(hm: Map<TImage, Vec<u8>>) -> Self {
        Self(hm)
    }
}

impl<TImage> PreimageOracle<TImage> for HashMapOracle<TImage>
where
    TImage: Borrow<TImage> + PartialEq + Eq + Hash + Debug + Ord,
{
    fn map<T, F>(&self, hash: TImage, f: F) -> Result<T, PreimageOracleError>
    where
        F: FnOnce(&[u8]) -> T,
    {
        Ok(f(self.0.get(&hash).ok_or(
            PreimageOracleError::PreimageNotFound(format_args!("{:?}", hash).to_string()),
        )?))
    }

    fn get_cached(&self, hash: TImage) -> Option<&[u8]> {
        self.0.get(&hash).map(|v| v.as_slice())
    }
}

#[cfg(feature = "ssz")]
pub type SszHashmapOracle = HashMapOracle<H256>;
