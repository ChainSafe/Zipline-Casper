use ssz_rs::prelude::*;

type Version = [u8; 4];
pub type Root = Node;
use alloc::{vec, vec::Vec};
#[derive(Default, Debug, SimpleSerialize)]
pub struct ForkData {
    pub current_version: Version,
    pub genesis_validators_root: Root,
}
