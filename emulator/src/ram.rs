use byteorder::{ByteOrder, BE};
use preimage_oracle::H256;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use std::sync::Arc;

use eth_trie::MemoryDB;
use eth_trie::{EthTrie, Trie, TrieError};
use std::iter::Iterator;

use log::debug;

pub trait Ram {
    fn write(&self, addr: u32, value: u32);
    fn read(&self, addr: u32) -> Option<u32>;
    fn read_or_default(&self, addr: u32) -> u32;
    fn load_data(&self, data: &[u8], base: u32);
    fn load_mapped_file(&self, file: String, base: u32);
    fn zero_registers(&self);
    fn ram_to_trie(&self, memdb: &Arc<MemoryDB>) -> Result<H256, TrieError>;
}

#[derive(Default)]
pub struct UnsyncRam {
    ram: Rc<RefCell<BTreeMap<u32, u32>>>,
}

// Implemented this myself because im not sure if deriving it will make a Rc clone of the Ram
impl Clone for UnsyncRam {
    fn clone(&self) -> Self {
        Self {
            ram: Rc::clone(&self.ram),
        }
    }
}

impl UnsyncRam {
    pub fn new() -> Self {
        Self {
            ram: Rc::new(RefCell::new(BTreeMap::new())),
        }
    }
}
impl Ram for UnsyncRam {
    // Writes a value to the Ram. Will panic if already mutably borrowed (though it shouldnt happen... tm)
    fn write(&self, addr: u32, value: u32) {
        // we no longer delete from ram, since deleting from tries is hard
        // if value == 0 && false {
        if false {
            self.ram.borrow_mut().remove(&addr);
        } else {
            /*if addr < 0xc0000000 {
                fmt.Printf("store %x = %x\n", addr, value)
            }*/
            self.ram.borrow_mut().insert(addr, value);
        }
    }

    fn read(&self, addr: u32) -> Option<u32> {
        self.ram.borrow().get(&addr).copied()
    }
    fn read_or_default(&self, addr: u32) -> u32 {
        self.ram.borrow().get(&addr).copied().unwrap_or_default()
    }

    fn load_data(&self, data: &[u8], base: u32) {
        let mut i = 0;
        let mut data_iter = data.chunks_exact(4);
        for chunk in data_iter.by_ref() {
            let value = BE::read_u32(chunk);
            if value != 0 {
                self.write(base + (i as u32 * 4), value);
            }
            i += 1;
        }
        let remaining_data = data_iter.remainder();
        if !remaining_data.is_empty() {
            let mut chunk = [0; 4];
            chunk[..remaining_data.len()].copy_from_slice(remaining_data);
            let value = BE::read_u32(&chunk);
            if value != 0 {
                self.write(base + (i as u32 * 4), value);
            }
        }
    }
    fn load_mapped_file(&self, file: String, base: u32) {
        let data = std::fs::read(file).unwrap();
        self.load_data(&data, base);
    }

    fn zero_registers(&self) {
        (0xC0000000..0xC0000000 + 36 * 4).step_by(4).for_each(|i| {
            self.write(i, 0);
        });
    }

    fn ram_to_trie(&self, memdb: &Arc<MemoryDB>) -> Result<H256, TrieError> {
        let mut trie = EthTrie::new(Arc::clone(memdb));

        let ram = self.ram.borrow();
        let count = ram.len();
        let ram_iter = ram.iter();

        let mut sram = vec![0; ram.len()];

        for (i, (k, v)) in ram_iter.enumerate() {
            sram[i] = ((*k as u64) << 32) | *v as u64;
        }
        sram.sort();

        for kv in sram.iter() {
            let mut k = (*kv >> 32) as u32;
            let v = (*kv) as u32;
            k >>= 2;

            let tk = k.to_be_bytes();
            let tv = v.to_be_bytes();
            trie.insert(&tk, &tv)?;
        }
        let root = trie.root_hash()?;

        debug!("hash count {}", count);
        // debug!("root {:?}", root);
        Ok(root.into())
    }
}
