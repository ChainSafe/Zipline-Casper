#![no_std]
#![feature(slice_take)]
#![feature(iterator_try_reduce)]
#![doc = include_str!("../README.md")]

pub mod attestation;
pub mod input;
pub mod signing;
pub mod ssz_state_reader;
pub mod state_patch;
pub mod state_reader;
pub mod verify;

pub use verify::*;

extern crate alloc;
