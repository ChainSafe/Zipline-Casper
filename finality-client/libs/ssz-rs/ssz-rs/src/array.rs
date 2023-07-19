//! This module provides `SimpleSerialize` implementations for arrays of size 1..=32.
//! These sizes are hard-coded as `SimpleSerialize` requires a `Default` implementation
//! and Rust already defines `Default` for these special array sizes.
//! If/when this restriction is lifted in favor of const generics, the macro here
//! can likely be simplified to a definition over `const N: usize`.
use crate::{
    de::{deserialize_homogeneous_composite, Deserialize, DeserializeError},
    error::{InstanceError, TypeError},
    lib::*,
    merkleization::{
        merkleize, pack, treeify, MerkleizationError, Merkleized, Node, BYTES_PER_CHUNK,
    },
    ser::{serialize_composite, Serialize, SerializeError},
    SimpleSerialize, Sized,
};

macro_rules! define_ssz_for_array_of_size {
    ($n: literal) => {
        impl<T> Sized for [T; $n]
        where
            T: SimpleSerialize,
        {
            fn is_variable_size() -> bool {
                T::is_variable_size()
            }

            fn size_hint() -> usize {
                T::size_hint() * $n
            }
        }

        impl<T> Serialize for [T; $n]
        where
            T: SimpleSerialize,
        {
            fn serialize(&self, buffer: &mut Vec<u8>) -> Result<usize, SerializeError> {
                if $n == 0 {
                    return Err(TypeError::InvalidBound($n).into());
                }
                serialize_composite(self, buffer)
            }
        }

        impl<T> Deserialize for [T; $n]
        where
            T: SimpleSerialize,
        {
            fn deserialize(encoding: &[u8]) -> Result<Self, DeserializeError> {
                if $n == 0 {
                    return Err(TypeError::InvalidBound($n).into());
                }

                if !T::is_variable_size() {
                    let expected_length = $n * T::size_hint();
                    if encoding.len() < expected_length {
                        return Err(DeserializeError::ExpectedFurtherInput {
                            provided: encoding.len(),
                            expected: expected_length,
                        });
                    }
                    if encoding.len() > expected_length {
                        return Err(DeserializeError::AdditionalInput {
                            provided: encoding.len(),
                            expected: expected_length,
                        });
                    }
                }
                let elements = deserialize_homogeneous_composite(encoding)?;
                elements.try_into().map_err(|elements: Vec<T>| {
                    InstanceError::Exact { required: $n, provided: elements.len() }.into()
                })
            }
        }

        impl<T> Merkleized for [T; $n]
        where
            T: SimpleSerialize,
        {
            fn hash_tree_root(&mut self) -> Result<Node, MerkleizationError> {
                if T::is_composite_type() {
                    let mut chunks = vec![0u8; self.len() * BYTES_PER_CHUNK];
                    for (i, elem) in self.iter_mut().enumerate() {
                        let chunk = elem.hash_tree_root()?;
                        let range = i * BYTES_PER_CHUNK..(i + 1) * BYTES_PER_CHUNK;
                        chunks[range].copy_from_slice(chunk.as_ref());
                    }
                    merkleize(&chunks, None)
                } else {
                    let chunks = pack(self)?;
                    merkleize(&chunks, None)
                }
            }
            fn to_merkle_tree(&mut self) -> Result<Vec<([u8; 32], [u8; 64])>, MerkleizationError> {
                let mut ret = vec![];
                if T::is_composite_type() {
                    let mut chunks = vec![0u8; self.len() * BYTES_PER_CHUNK];
                    for (i, elem) in self.iter_mut().enumerate() {
                        let chunk = elem.hash_tree_root()?;
                        ret.append(&mut elem.to_merkle_tree()?);
                        let range = i * BYTES_PER_CHUNK..(i + 1) * BYTES_PER_CHUNK;
                        chunks[range].copy_from_slice(chunk.as_ref());
                    }
                    ret.append(&mut treeify(&chunks, Some(self.len()))?);
                } else {
                    let chunks = pack(self)?;
                    ret.append(&mut treeify(&chunks, Some(self.len()))?);
                }
                Ok(ret)
            }
        }

        impl<T> SimpleSerialize for [T; $n]
        where
            T: SimpleSerialize,
        {
            fn is_composite_type() -> bool {
                T::is_composite_type()
            }
        }
    };
}

define_ssz_for_array_of_size!(1);
define_ssz_for_array_of_size!(2);
define_ssz_for_array_of_size!(3);
define_ssz_for_array_of_size!(4);
define_ssz_for_array_of_size!(5);
define_ssz_for_array_of_size!(6);
define_ssz_for_array_of_size!(7);
define_ssz_for_array_of_size!(8);
define_ssz_for_array_of_size!(9);
define_ssz_for_array_of_size!(10);
define_ssz_for_array_of_size!(11);
define_ssz_for_array_of_size!(12);
define_ssz_for_array_of_size!(13);
define_ssz_for_array_of_size!(14);
define_ssz_for_array_of_size!(15);
define_ssz_for_array_of_size!(16);
define_ssz_for_array_of_size!(17);
define_ssz_for_array_of_size!(18);
define_ssz_for_array_of_size!(19);
define_ssz_for_array_of_size!(20);
define_ssz_for_array_of_size!(21);
define_ssz_for_array_of_size!(22);
define_ssz_for_array_of_size!(23);
define_ssz_for_array_of_size!(24);
define_ssz_for_array_of_size!(25);
define_ssz_for_array_of_size!(26);
define_ssz_for_array_of_size!(27);
define_ssz_for_array_of_size!(28);
define_ssz_for_array_of_size!(29);
define_ssz_for_array_of_size!(30);
define_ssz_for_array_of_size!(31);
define_ssz_for_array_of_size!(32);
