use crate::merkleization::{MerkleizationError, Node};
use bitvec::prelude::*;
use sha2::{Digest, Sha256};

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec, collections::BTreeMap as Map};

#[cfg(feature = "std")]
use std::collections::HashMap as Map;

/// `is_valid_merkle_branch` verifies the Merkle proof
/// against the `root` given the other metadata.
pub fn is_valid_merkle_branch<'a>(
    leaf: &Node,
    mut branch: impl Iterator<Item = &'a Node>,
    depth: usize,
    index: usize,
    root: &Node,
) -> bool {
    let mut value = *leaf;

    let mut hasher = Sha256::new();
    for i in 0..depth {
        let next_node = match branch.next() {
            Some(node) => node,
            None => return false,
        };
        if (index / 2usize.pow(i as u32)) % 2 != 0 {
            hasher.update(next_node.as_ref());
            hasher.update(value.as_ref());
        } else {
            hasher.update(value.as_ref());
            hasher.update(next_node.as_ref());
        }
        value.as_mut().copy_from_slice(&hasher.finalize_reset());
    }
    value == *root
}

// only use this method for very small trees. It is extremely inefficient and holds the whole tree
// in memory (and clones it :())
pub fn compute_proof(
    root: &Node,
    gindex: usize,
    tree: &[([u8; 32], [u8; 64])],
) -> Result<Vec<[u8; 32]>, MerkleizationError> {
    let tree_map: Map<[u8; 32], [u8; 64]> = tree.iter().cloned().collect();
    let root: [u8; 32] = root.as_ref().try_into().unwrap();
    let (_, proof): (_, Vec<[u8; 32]>) = gindex
        .view_bits::<Msb0>()
        .iter()
        .skip_while(|b| b.as_ref() == &false) // skip the leading zeros
        .skip(1) // skip the first 1, this just indicates the root
        .try_fold((root, vec![]), |(hash, mut proof), direction| {
            let leaves = tree_map.get(hash.as_ref()).ok_or(MerkleizationError::MissingNode)?;
            let mut left = [0_u8; 32];
            let mut right = [0_u8; 32];
            left.copy_from_slice(&leaves[0..32]);
            right.copy_from_slice(&leaves[32..64]);
            match direction.as_ref() {
                false => {
                    // left
                    proof.insert(0, right.try_into().unwrap());
                    Ok::<_, MerkleizationError>((left, proof))
                }
                true => {
                    //right
                    proof.insert(0, left.try_into().unwrap());
                    Ok((right, proof))
                }
            }
        })?;
    Ok(proof)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_node_from_hex(hex: &str) -> Node {
        let bytes = hex::decode(hex).expect("is hex");
        Node::try_from(bytes.as_ref()).expect("is right size")
    }

    #[test]
    fn test_basic_proof() {
        let leaf = decode_node_from_hex(
            "94159da973dfa9e40ed02535ee57023ba2d06bad1017e451055470967eb71cd5",
        );
        let branch = [
            "8f594dbb4f4219ad4967f86b9cccdb26e37e44995a291582a431eef36ecba45c",
            "f8c2ed25e9c31399d4149dcaa48c51f394043a6a1297e65780a5979e3d7bb77c",
            "382ba9638ce263e802593b387538faefbaed106e9f51ce793d405f161b105ee6",
            "c78009fdf07fc56a11f122370658a353aaa542ed63e44c4bc15ff4cd105ab33c",
        ]
        .into_iter()
        .map(decode_node_from_hex)
        .collect::<Vec<_>>();
        let depth = 3;
        let index = 2;
        let root = decode_node_from_hex(
            "27097c728aade54ff1376d5954681f6d45c282a81596ef19183148441b754abb",
        );

        assert!(is_valid_merkle_branch(&leaf, branch.iter(), depth, index, &root))
    }
}
