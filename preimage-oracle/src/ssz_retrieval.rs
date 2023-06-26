use crate::{error::PreimageOracleError, oracle_backend::PreimageOracle, H256};
use alloc::{vec, vec::Vec};
use bitvec::prelude::*;

type Chunk = [u8; 32];
type GIndex = u64;

fn is_left_node(depth: u64, index: GIndex) -> bool {
    let mask = 1 << depth;
    (index & mask) != mask
}

pub fn iterate_nodes_at_depth<T: SszOracle>(
    oracle: &T,
    root_node: [u8; 32],
    depth: u64,
    start_index: u64,
    count: u64,
) -> impl Iterator<Item = [u8; 32]> + '_ {
    let end_index = start_index + count;

    // Ignore first bit "1", then subtract 1 to get to the parent
    let depth_i_root = depth - 1;
    let depth_i_parent = 0;
    let mut depth_i = depth_i_root;
    let mut node = root_node;

    // Contiguous filled stack of parent nodes. It get filled in the first descent
    // Indexed by depth_i
    let mut parent_node_stack = vec![root_node; depth as usize];
    let mut is_left_stack = vec![false; depth as usize];

    // Insert root node to make the loop below general
    parent_node_stack[depth_i_root as usize] = root_node;

    (start_index..end_index).map(move |index| {
        for d in (depth_i_parent..=depth_i).rev() {
            if d != depth_i {
                parent_node_stack[d as usize] = node;
            }

            let is_left = is_left_node(d, index);
            is_left_stack[d as usize] = is_left;
            node = oracle
                .map(node, |children| {
                    if is_left {
                        children[0..32].try_into().unwrap()
                    } else {
                        children[32..64].try_into().unwrap()
                    }
                })
                .unwrap();
        }

        let result = node;

        // Find the first depth where navigation when left.
        // Store that height and go right from there
        for d in depth_i_parent..=depth_i_root {
            if is_left_stack[d as usize] {
                depth_i = d;
                break;
            }
        }

        node = parent_node_stack[depth_i as usize];

        result
    })
}

/// A specific implementation that assumes the existence of a pre-image oracle which given a hash can return its pre-image
/// This specific trait is for SSZ data that has been merklized according to SSZ specifications.
pub trait SszOracle: PreimageOracle<H256> {
    /// Apply a function to a 32 byte chunk of data in a merklized SSZ data structure and return the result
    /// The tree is defined by `root`. Implementation must have a method for retrieving the tree data given its root.
    /// chunk is indexed by its generalized_index in the tree (gindex)
    fn map_chunk<F, T>(&self, root: H256, gindex: GIndex, func: F) -> Result<T, PreimageOracleError>
    where
        F: FnOnce(&Chunk) -> T,
    {
        if gindex == 1 {
            return Ok(func(&root));
        }
        let chunk = gindex
            .to_be_bytes()
            .view_bits::<Msb0>()
            .iter()
            .skip_while(|b| b.as_ref() == &false) // skip the leading zeros
            .skip(1) // skip the first 1, this just indicates the root
            .try_fold(root, |hash, direction| {
                self.map(hash, |d| {
                    assert!(d.len() == 64, "We should always be receiving two new nodes");
                    let mut next_hash = [0_u8; 32];
                    match direction.as_ref() {
                        false => {
                            next_hash.copy_from_slice(&d[0..32]);
                        }
                        true => {
                            next_hash.copy_from_slice(&d[32..64]);
                        }
                    }
                    next_hash
                })
            })?;
        Ok(func(&chunk))
    }

    fn map_as_uint64<F, T>(
        &self,
        root: H256,
        gindex: GIndex,
        func: F,
    ) -> Result<T, PreimageOracleError>
    where
        F: FnOnce(u64) -> T,
    {
        self.map_chunk(root, gindex, |chunk| {
            let mut b = [0; 8];
            b.copy_from_slice(&chunk[0..8]);
            func(u64::from_le_bytes(b))
        })
    }

    /// Same as above, but write the chunks to the heap
    fn map_cache<F, T>(&self, root: H256, gindex: GIndex, func: F) -> Result<T, PreimageOracleError>
    where
        F: FnOnce(&Chunk) -> T,
    {
        if gindex == 1 {
            return Ok(func(&root));
        }
        let binding = gindex.to_be_bytes();
        let chunk: Vec<_> = binding
            .view_bits::<Msb0>()
            .iter()
            .skip_while(|b| b.as_ref() == &false) // skip the leading zeros
            .skip(1)
            .collect(); // skip the first 1, this just indicates the root
        let mut next_hash = root;
        for c in chunk {
            let d = self.get_cached(next_hash).unwrap();
            assert!(d.len() == 64, "We should always be receiving two new nodes");
            match c.as_ref() {
                false => {
                    next_hash.copy_from_slice(&d[0..32]);
                }
                true => {
                    next_hash.copy_from_slice(&d[32..64]);
                }
            }
        }
        Ok(func(&next_hash))
    }

    /// Directly make a copy of the chunk
    fn copy_chunk(&self, root: H256, gindex: GIndex) -> Result<[u8; 32], PreimageOracleError> {
        self.map_chunk(root, gindex, |chunk| *chunk)
    }

    fn copy_and_cache_chunk(
        &self,
        root: H256,
        gindex: GIndex,
    ) -> Result<[u8; 32], PreimageOracleError> {
        self.map_cache(root, gindex, |chunk| *chunk)
    }
}

impl<T> SszOracle for T where T: PreimageOracle<H256> {}

#[cfg(test)]
mod test {
    use super::SszOracle;
    use crate::oracle_backend::hashmap_oracle::SszHashmapOracle;
    use alloc::collections::btree_map::BTreeMap as Map;

    // Return an oracle where each hash is its gindex and
    // each value is a 32 byte array with the gindex in the 0th element
    fn create_mock_oracle() -> SszHashmapOracle {
        let elements = (1_u8..2 ^ 4).map(|i| {
            let mut preimage = [0_u8; 64];
            preimage[0] = 2 * i;
            preimage[32] = 2 * i + 1;
            let mut v = [0_u8; 32];
            v[0] = i;
            (v, preimage.to_vec())
        });
        SszHashmapOracle::from(elements.collect::<Map<_, _>>())
    }

    #[test]
    fn empty_path() {
        let retriever = create_mock_oracle();

        retriever
            .map_chunk([1; 32], 1, |chunk| {
                assert_eq!(chunk, &[1; 32]);
            })
            .unwrap();
    }

    #[test]
    fn single_left() {
        let retriever = create_mock_oracle();

        let mut root = [0; 32];
        root[0] = 1;
        retriever
            .map_chunk(root, 0b10, |chunk| {
                let mut expected = [0; 32];
                expected[0] = 2;
                assert_eq!(chunk, &expected);
            })
            .unwrap();
    }

    #[test]
    fn single_left_left() {
        let retriever = create_mock_oracle();

        let mut root = [0; 32];
        root[0] = 1;
        retriever
            .map_chunk(root, 0b100, |chunk| {
                let mut expected = [0; 32];
                expected[0] = 4;
                assert_eq!(chunk, &expected);
            })
            .unwrap();
    }

    #[test]
    fn single_left_right() {
        let retriever = create_mock_oracle();

        let mut root = [0; 32];
        root[0] = 1;
        retriever
            .map_chunk(root, 0b101, |chunk| {
                let mut expected = [0; 32];
                expected[0] = 5;
                assert_eq!(chunk, &expected);
            })
            .unwrap();
    }
}
