#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

mod array;
mod bitlist;
mod bitvector;
mod boolean;
mod container;
mod de;
mod error;
mod list;
mod merkleization;
mod ser;
#[cfg(feature = "serde")]
mod serde;
mod uint;
mod union;
mod utils;
mod vector;

pub use crate::{
    bitlist::Bitlist,
    bitvector::Bitvector,
    de::{Deserialize, DeserializeError},
    error::{Error as SimpleSerializeError, InstanceError, TypeError},
    list::List,
    merkleization::{is_valid_merkle_branch, compute_proof, MerkleizationError, Merkleized, Node},
    ser::{Serialize, SerializeError},
    uint::U256,
    utils::{deserialize, serialize},
    vector::Vector,
};

mod lib {
    mod core {
        #[cfg(not(feature = "std"))]
        pub use core::*;
        #[cfg(feature = "std")]
        pub use std::*;
    }

    pub use self::core::{any, cmp, fmt, iter, slice};

    pub use self::{
        cmp::Ordering,
        core::{
            array::TryFromSliceError,
            fmt::{Debug, Display, Formatter},
            ops::{Deref, DerefMut, Index, IndexMut},
            slice::{IterMut, SliceIndex},
        },
        iter::Enumerate,
    };

    #[cfg(not(feature = "std"))]
    pub use alloc::{format, string::String, vec, vec::Vec};

    #[cfg(feature = "std")]
    pub use std::vec::Vec;
}

/// `Sized` is a trait for types that can
/// provide sizing information relevant for the SSZ spec.
pub trait Sized {
    // is this type variable or fixed size?
    fn is_variable_size() -> bool;

    fn size_hint() -> usize;
}

/// `SimpleSerialize` is a trait for types
/// conforming to the SSZ spec.
pub trait SimpleSerialize: Serialize + Deserialize + Sized + Merkleized + Default {
    fn is_composite_type() -> bool {
        true
    }
}

/// The `prelude` contains common traits and types a user of this library
/// would want to have handy with a simple (single) import.
pub mod prelude {
    pub use crate::{
        bitlist::Bitlist,
        bitvector::Bitvector,
        de::{Deserialize, DeserializeError},
        error::{Error as SimpleSerializeError, InstanceError, TypeError},
        list::List,
        merkleization::{is_valid_merkle_branch, MerkleizationError, Merkleized, Node},
        ser::{Serialize, SerializeError},
        uint::U256,
        utils::{deserialize, serialize},
        vector::Vector,
        SimpleSerialize, Sized,
    };
    // expose this so the derive macro has everything in scope
    // with a simple `prelude` import
    pub use crate as ssz_rs;
    pub use ssz_rs_derive::SimpleSerialize;
}

#[doc(hidden)]
/// `internal` contains functionality that is exposed purely for the derive proc macro crate
pub mod __internal {
    // exported for derive macro to avoid code duplication...
    pub use crate::{
        merkleization::{merkleize, mix_in_selector, mix_in_selector_tree, treeify},
        ser::serialize_composite_from_components,
    };
}
