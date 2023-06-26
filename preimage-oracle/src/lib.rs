#![no_std]
#![feature(type_alias_impl_trait)]
pub mod oracle_backend;

#[cfg(feature = "ssz")]
pub mod ssz_retrieval;
#[cfg(feature = "ssz")]
pub use ssz_retrieval::*;

pub type H256 = [u8; 32];

pub use oracle_backend::*;

pub mod error;

extern crate alloc;
