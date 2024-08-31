//! Fact node structure and helpers.
//!
//! The fact of each task is stored as a (non-binary) Merkle tree.
//! Leaf nodes are labeled with the hash of their data.
//! Each non-leaf node is labeled as 1 + the hash of (node0, end0, node1, end1, ...)
//! where node* is a label of a child children and end* is the total number of data words up to
//! and including that node and its children (including the previous sibling nodes).
//! We add 1 to the result of the hash to prevent an attacker from using a preimage of a leaf node
//! as a preimage of a non-leaf hash and vice versa.
//!
//! The structure of the tree is passed as a list of pairs (n_pages, n_nodes), and the tree is
//! constructed using a stack of nodes (initialized to an empty stack) by repeating for each pair:
//!   1. Add #n_pages lead nodes to the stack.
//!   2. Pop the top #n_nodes, construct a parent node for them, and push it back to the stack.
//!      After applying the steps above, the stack must contain exactly one node, which will
//!      constitute the root of the Merkle tree.
//!
//! For example, [(2, 2)] will create a Merkle tree with a root and two direct children, while
//! [(3, 2), (0, 2)] will create a Merkle tree with a root whose left child is a leaf and
//! right child has two leaf children.
//!
//! Port of https://github.com/starkware-libs/cairo-lang/blob/master/src/starkware/cairo/bootloaders/compute_fact.py

use alloy::primitives::{keccak256, B256};
use cairo_vm::Felt252;
use itertools::Itertools;
use num_bigint::BigUint;
use std::ops::Add;
use utils::ensure;

use super::error::FactCheckerError;
use super::fact_topology::FactTopology;

/// Node of the fact tree
#[derive(Debug, Clone)]
pub struct FactNode {
    /// Page hash (leaf) or 1 + keccak{children} (non-leaf)
    pub node_hash: B256,
    /// Total number of data words up to that node (including it and its children)
    pub end_offset: usize,
    /// Page size
    pub page_size: usize,
    /// Child nodes
    pub children: Vec<FactNode>,
}

/// Generates the root of the output Merkle tree for the program fact computation.
///
/// Basically it transforms the flat fact topology into a non-binary Merkle tree and then computes
/// its root, enriching the nodes with metadata such as page sizes and hashes.
pub fn generate_merkle_root(
    program_output: &[Felt252],
    fact_topology: &FactTopology,
) -> Result<FactNode, FactCheckerError> {
    let FactTopology { tree_structure, mut page_sizes } = fact_topology.clone();

    let mut end_offset: usize = 0;
    let mut node_stack: Vec<FactNode> = Vec::with_capacity(page_sizes.len());
    let mut output_iter = program_output.iter();

    for (n_pages, n_nodes) in tree_structure.into_iter().tuples() {
        ensure!(
            n_pages <= page_sizes.len(),
            FactCheckerError::TreeStructurePagesCountOutOfRange(n_pages, page_sizes.len())
        );

        // Push n_pages (leaves) to the stack
        for _ in 0..n_pages {
            let page_size = page_sizes.remove(0);
            // Page size is already validated upon retrieving the topology
            let page = output_iter.by_ref().take(page_size).map(|felt| felt.to_bytes_be().to_vec()).concat();
            let node_hash = keccak256(&page);
            end_offset += page_size;
            // Add lead node (no children)
            node_stack.push(FactNode { node_hash, end_offset, page_size, children: vec![] })
        }

        ensure!(
            n_nodes <= node_stack.len(),
            FactCheckerError::TreeStructureNodesCountOutOfRange(n_nodes, node_stack.len())
        );

        if n_nodes > 0 {
            // Create a parent node to the last n_nodes in the head of the stack.
            let children: Vec<FactNode> = node_stack.drain(node_stack.len() - n_nodes..).collect();
            let mut node_data = Vec::with_capacity(2 * 32 * children.len());
            let mut total_page_size = 0;
            let mut child_end_offset = 0;

            for node in children.iter() {
                node_data.extend_from_slice(node.node_hash.as_slice());
                node_data.extend_from_slice(&[0; 32 - (usize::BITS / 8) as usize]); // pad usize to 32 bytes
                node_data.extend_from_slice(&node.end_offset.to_be_bytes());
                total_page_size += node.page_size;
                child_end_offset = node.end_offset;
            }

            node_stack.push(FactNode {
                node_hash: calculate_node_hash(node_data.as_slice()),
                end_offset: child_end_offset,
                page_size: total_page_size,
                children,
            })
        }
    }

    ensure!(node_stack.len() == 1, FactCheckerError::TreeStructureRootInvalid);
    ensure!(page_sizes.is_empty(), FactCheckerError::TreeStructurePagesNotProcessed(page_sizes.len()));
    ensure!(
        end_offset == program_output.len(),
        FactCheckerError::TreeStructureEndOffsetInvalid(end_offset, program_output.len())
    );
    ensure!(
        node_stack[0].end_offset == program_output.len(),
        FactCheckerError::TreeStructureRootOffsetInvalid(node_stack[0].end_offset, program_output.len(),)
    );

    Ok(node_stack.remove(0))
}

/// Calculates the keccak hash and adds 1 to it.
fn calculate_node_hash(node_data: &[u8]) -> B256 {
    let hash = keccak256(node_data);
    let hash_biguint = BigUint::from_bytes_be(hash.as_slice());
    let incremented_hash = hash_biguint.add(BigUint::from(1u8));
    let mut hash_bytes = incremented_hash.to_bytes_be();
    while hash_bytes.len() < 32 {
        hash_bytes.insert(0, 0);
    }
    B256::from_slice(&hash_bytes[..32])
}

#[cfg(test)]
mod test {
    use crate::fact_node::generate_merkle_root;
    use crate::fact_topology::FactTopology;
    use alloy::primitives::B256;
    use cairo_vm::Felt252;
    use std::str::FromStr;

    /// Here we are comparing our output with the same function run in the
    /// `generate_output_root` function in cairo-lang repo.
    /// We are comparing the output hash of the `generate_merkle_root` function
    /// with our python output.
    ///
    /// Function link : https://github.com/starkware-libs/cairo-lang/blob/a86e92bfde9c171c0856d7b46580c66e004922f3/src/starkware/cairo/bootloaders/compute_fact.py#L47
    ///
    /// This will ensure that our logic here is correct.
    #[test]
    fn test_generate_merkle_root() {
        let program_output_vec: Vec<Felt252> = vec![
            1.into(),
            2.into(),
            3.into(),
            4.into(),
            5.into(),
            6.into(),
            7.into(),
            8.into(),
            9.into(),
            10.into(),
            11.into(),
            12.into(),
        ];

        let fact_topology =
            FactTopology { tree_structure: vec![1, 0, 1, 0, 0, 2, 1, 1, 0, 2], page_sizes: vec![4, 4, 4] };

        let merkle_root = generate_merkle_root(program_output_vec.as_slice(), &fact_topology).unwrap().node_hash;
        let python_program_output =
            B256::from_str("0x17F41BA1DB11E3A164B23B72B52190FB0DA6184B4B80CF74E0882FDE7438E47F").unwrap();

        assert_eq!(merkle_root, python_program_output);
    }
}
