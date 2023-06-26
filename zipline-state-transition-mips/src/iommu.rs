//! This module is used when the state machine compiled into MIPS to interact with the host
//! environment. The host environment is either the prover or the onchain one step verifier.

use alloc::string::ToString;
use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use core::ptr;
use preimage_oracle::{error::PreimageOracleError, PreimageOracle, H256};
// starting place in memory for all of the Cannon special memory slots
// This must match what is in the Solidity contract MIPSMemory.sol contract
const SPECIAL_MEM_BASE: usize = 0x30000000;

/// The address of the input hash.
const PTR_INPUT_HASH: usize = SPECIAL_MEM_BASE;
/// The address where the output hash is written at the end of execution.
const PTR_OUTPUT_HASH: usize = SPECIAL_MEM_BASE + 0x00000804;
/// The address where a special magic value is written at the end of execution.
const PTR_MAGIC: usize = SPECIAL_MEM_BASE + 0x00000800;
/// The address where the preimage hash for the preimage oracle is written by the guest.
const PTR_PREIMAGE_ORACLE_HASH: usize = SPECIAL_MEM_BASE + 0x00001000;
/// The address where the preimage oracle output size is written by the host.
const PTR_PREIMAGE_ORACLE_SIZE: usize = SPECIAL_MEM_BASE + 0x01000000;
/// The address where the preimage oracle output data is written by the host.
const PTR_PREIMAGE_ORACLE_DATA: usize = SPECIAL_MEM_BASE + 0x01000004;

// value that must be written to PTR_MAGIC on successful termination
const MAGIC_VALUE: u32 = 0x1337f00d;

/// Loads the input hash from the host environment.
pub fn input_hash() -> H256 {
    unsafe { ptr::read_volatile(PTR_INPUT_HASH as *const H256) }
}

/// Prepares the guest environment to exiting. Writes the output hash and the magic to be read by
/// the host and then halts the execution.
pub fn output(hash: H256) -> ! {
    unsafe {
        ptr::write_volatile(PTR_MAGIC as *mut u32, MAGIC_VALUE);
        ptr::write_volatile(PTR_OUTPUT_HASH as *mut H256, hash);
        ffi::halt();
    }
}

static mut PREIMAGE_CACHE: Option<BTreeMap<H256, Vec<u8>>> = None;

pub struct CannonPreimageOracle;

pub fn preimage_oracle() -> CannonPreimageOracle {
    unsafe{
    match PREIMAGE_CACHE {
        Some(_) => panic!("preimage oracle already initialized"),
        None => {
            let cache = BTreeMap::new();
            PREIMAGE_CACHE = Some(cache);
            PREIMAGE_CACHE.as_mut().unwrap()
        }
    };
}
    CannonPreimageOracle {}
}

impl PreimageOracle<H256> for CannonPreimageOracle {
    fn map<T, F>(&self, hash: H256, f: F) -> Result<T, PreimageOracleError>
    where
        F: FnOnce(&[u8]) -> T,
    {
        unsafe {
            // preimage cache should be initialized
            let preimage_cache = match PREIMAGE_CACHE {
                Some(ref mut cache) => cache,
                None => {
                    panic!("preimage cache not initialized");
                }
            };
            // Check if the preimage is already cached.
            if let Some(data) = preimage_cache.get(&hash) {
                Ok(f(data))
            } else {
                // write the hash to the special memory location
                *(PTR_PREIMAGE_ORACLE_HASH as *mut [u8; 32]) = hash;

                ffi::preimage_oracle();

                // Read the size of the preimage. It seems to be BE, so no conversion needed.
                let size = *(PTR_PREIMAGE_ORACLE_SIZE as *const u32);
                if size == 0 {
                    return Err(PreimageOracleError::Other("Preimage size 0".to_string()));
                }

                // Read the preimage from its memory location.
                //
                // SAFETY: The pointer is aligned by definition and is not null.
                let data =
                    core::slice::from_raw_parts(PTR_PREIMAGE_ORACLE_DATA as *const u8, size as usize);
                // call passed function with this data and return the result
                Ok(f(data))
            }


        }
    }

    fn get_cached(&self, hash: H256) -> Option<&[u8]> {
        // The cache of all requested preimages to avoid going via the host boundary every time.
        //
        // Under MIPS this is running exclusively in single-threaded mode. We could've avoided using
        // a Mutex, but it seems to be fine. Uncontended use is just atomic writes.

        // assume the given reference is valid for the whole program lifetime.
        let eternalize = |v: &Vec<u8>| -> &'static [u8] {
            // SAFETY: this is safe because we are creating the slice from the pointer and the size
            //         that were already produced by a vec.
            //
            //         use-after-free is also a non concern because the vec is owned by the cache and
            //         the cache is never pruned.
            unsafe { core::slice::from_raw_parts(v.as_ptr(), v.len()) }
        };

        unsafe {
            // preimage cache should be initialized
            let preimage_cache = match PREIMAGE_CACHE {
                Some(ref mut cache) => cache,
                None => {
                    panic!("preimage cache not initialized");
                }
            };
            // Check if the preimage is already cached.
            if let Some(preimage) = preimage_cache.get(&hash) {
                Some(eternalize(preimage))
            } else {
                // if not then cache it
                self.map(hash, |data| {
                    preimage_cache.insert(hash, data.to_vec());
                })
                .unwrap();
                // safe to unwrap as we literally just put it there
                Some(eternalize(preimage_cache.get(&hash).unwrap()))
            }
        }
    }
}

pub fn print(s: &str) {
    unsafe {
        ffi::write(1, s.as_ptr(), s.len());
    }
}

mod ffi {
    //! See asm.S
    extern "C" {
        pub fn halt() -> !;
        pub fn preimage_oracle();
        pub fn write(fd: usize, buf: *const u8, count: usize);
    }
}
