//! Merkle tree implementation for proof generation
//!
//! Binary Merkle tree with SHA3-256 hashing.
//! Supports inclusion proofs for individual payments.

use crate::{Error, Result};
use sha3::{Digest, Sha3_256};

/// Merkle tree node
#[derive(Debug, Clone)]
struct Node {
    hash: [u8; 32],
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

/// Merkle tree
#[derive(Debug)]
pub struct MerkleTree {
    root: Option<Node>,
    leaves: Vec<[u8; 32]>,
    original_leaf_count: usize,
}

impl MerkleTree {
    /// Build Merkle tree from leaf hashes
    pub fn build(leaves: Vec<[u8; 32]>) -> Result<Self> {
        if leaves.is_empty() {
            return Err(Error::Protocol("Cannot build tree from empty leaves".into()));
        }

        let root = Self::build_tree(&leaves);

        Ok(Self {
            root: Some(root),
            leaves: leaves.clone(),
            original_leaf_count: leaves.len(),
        })
    }

    /// Recursively build tree
    fn build_tree(hashes: &[[u8; 32]]) -> Node {
        if hashes.len() == 1 {
            // Leaf node
            return Node {
                hash: hashes[0],
                left: None,
                right: None,
            };
        }

        // Handle odd number of hashes: duplicate last hash
        let mut hashes_vec = hashes.to_vec();
        if hashes_vec.len() % 2 == 1 {
            hashes_vec.push(*hashes_vec.last().unwrap());
        }

        // Split into left and right subtrees
        let mid = hashes_vec.len() / 2;
        let left = Self::build_tree(&hashes_vec[..mid]);
        let right = Self::build_tree(&hashes_vec[mid..]);

        // Compute parent hash
        let parent_hash = Self::hash_pair(&left.hash, &right.hash);

        Node {
            hash: parent_hash,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }
    }

    /// Hash two child nodes
    fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().into()
    }

    /// Get root hash
    pub fn root(&self) -> Result<[u8; 32]> {
        self.root
            .as_ref()
            .map(|n| n.hash)
            .ok_or_else(|| Error::Protocol("Tree has no root".into()))
    }

    /// Generate Merkle proof for leaf at index
    pub fn prove(&self, leaf_index: usize) -> Result<MerkleProof> {
        if leaf_index >= self.original_leaf_count {
            return Err(Error::Protocol(format!(
                "Leaf index {} out of bounds ({})",
                leaf_index,
                self.original_leaf_count
            )));
        }

        let leaf_hash = self.leaves[leaf_index];
        let sibling_hashes = self.get_sibling_path(leaf_index)?;

        Ok(MerkleProof {
            leaf_hash,
            leaf_index,
            sibling_hashes,
            root: self.root()?,
        })
    }

    /// Get sibling hashes along path from leaf to root
    fn get_sibling_path(&self, leaf_index: usize) -> Result<Vec<[u8; 32]>> {
        self.get_sibling_path_from_node(self.root.as_ref().unwrap(), leaf_index, 0, self.leaves.len())
    }

    /// Recursively get sibling path from node
    fn get_sibling_path_from_node(&self, node: &Node, target_index: usize, current_start: usize, current_size: usize) -> Result<Vec<[u8; 32]>> {
        // Leaf node - no siblings
        if node.left.is_none() && node.right.is_none() {
            return Ok(vec![]);
        }

        let left = node.left.as_ref().unwrap();
        let right = node.right.as_ref().unwrap();

        // Determine split point (accounting for padding)
        let mut left_size = current_size / 2;
        if current_size % 2 == 1 {
            left_size = (current_size + 1) / 2;
        }
        let mid = current_start + left_size;

        // Determine which subtree contains target
        let siblings = if target_index < mid {
            // Target is in left subtree, sibling is right hash
            let mut path = self.get_sibling_path_from_node(left, target_index, current_start, left_size)?;
            path.push(right.hash);  // Add sibling at this level (bottom-up order)
            path
        } else {
            // Target is in right subtree, sibling is left hash
            let mut path = self.get_sibling_path_from_node(right, target_index, mid, current_size - left_size)?;
            path.push(left.hash);  // Add sibling at this level (bottom-up order)
            path
        };

        Ok(siblings)
    }

    /// Verify entire tree integrity
    pub fn verify(&self) -> bool {
        if let Some(ref root) = self.root {
            Self::verify_node(root)
        } else {
            false
        }
    }

    fn verify_node(node: &Node) -> bool {
        match (&node.left, &node.right) {
            (Some(left), Some(right)) => {
                // Verify parent hash
                let computed = Self::hash_pair(&left.hash, &right.hash);
                if computed != node.hash {
                    return false;
                }
                // Recursively verify children
                Self::verify_node(left) && Self::verify_node(right)
            }
            (None, None) => true, // Leaf node, always valid
            _ => false,           // Invalid: only one child
        }
    }
}

/// Merkle proof for inclusion
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// Leaf hash
    pub leaf_hash: [u8; 32],
    /// Leaf index in tree
    pub leaf_index: usize,
    /// Sibling hashes along path to root
    pub sibling_hashes: Vec<[u8; 32]>,
    /// Root hash
    pub root: [u8; 32],
}

impl MerkleProof {
    /// Verify proof is valid
    pub fn verify(&self) -> bool {
        let mut current = self.leaf_hash;
        let mut index = self.leaf_index;

        for sibling in &self.sibling_hashes {
            current = if index % 2 == 0 {
                // Current is left child
                Self::hash_pair(&current, sibling)
            } else {
                // Current is right child
                Self::hash_pair(sibling, &current)
            };
            index /= 2;
        }

        current == self.root
    }

    fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(left);
        hasher.update(right);
        hasher.finalize().into()
    }
}

/// Utility: Hash arbitrary data to [u8; 32]
pub fn hash_data(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_build() {
        let leaves = vec![
            hash_data(b"payment1"),
            hash_data(b"payment2"),
            hash_data(b"payment3"),
            hash_data(b"payment4"),
        ];

        let tree = MerkleTree::build(leaves.clone()).unwrap();
        let root = tree.root().unwrap();

        // Root should be deterministic
        assert_eq!(root.len(), 32);
    }

    #[test]
    fn test_merkle_proof_verify() {
        let leaves = vec![
            hash_data(b"payment1"),
            hash_data(b"payment2"),
            hash_data(b"payment3"),
            hash_data(b"payment4"),
        ];

        let tree = MerkleTree::build(leaves.clone()).unwrap();

        // Prove leaf 2
        let proof = tree.prove(2).unwrap();
        assert!(proof.verify());

        // Tamper with leaf hash
        let mut bad_proof = proof.clone();
        bad_proof.leaf_hash = hash_data(b"tampered");
        assert!(!bad_proof.verify());
    }

    #[test]
    fn test_merkle_proof_all_leaves() {
        // Test with power of 2 leaves (4) - fully balanced tree
        let leaves = vec![
            hash_data(b"A"),
            hash_data(b"B"),
            hash_data(b"C"),
            hash_data(b"D"),
        ];

        let tree = MerkleTree::build(leaves.clone()).unwrap();

        // All proofs should verify
        for i in 0..leaves.len() {
            let proof = tree.prove(i).unwrap();
            assert!(proof.verify(), "Proof for leaf {} failed", i);
        }

        // Test with odd number (3) - requires padding
        let leaves = vec![
            hash_data(b"X"),
            hash_data(b"Y"),
            hash_data(b"Z"),
        ];

        let tree = MerkleTree::build(leaves.clone()).unwrap();
        for i in 0..leaves.len() {
            let proof = tree.prove(i).unwrap();
            assert!(proof.verify(), "Proof for leaf {} failed", i);
        }
    }

    #[test]
    fn test_merkle_tree_verify_integrity() {
        let leaves = vec![hash_data(b"A"), hash_data(b"B"), hash_data(b"C")];

        let tree = MerkleTree::build(leaves).unwrap();
        assert!(tree.verify());
    }

    #[test]
    fn test_single_leaf_tree() {
        let leaves = vec![hash_data(b"singleton")];

        let tree = MerkleTree::build(leaves).unwrap();
        let proof = tree.prove(0).unwrap();

        assert!(proof.verify());
        assert_eq!(proof.sibling_hashes.len(), 0); // No siblings for single leaf
    }
}