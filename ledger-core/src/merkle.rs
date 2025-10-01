//! Merkle tree for cryptographic proofs
//!
//! This module provides an incremental Merkle tree implementation
//! for efficient proof generation and verification.
//!
//! # Design
//!
//! - Binary Merkle tree with SHA-256 hashing
//! - Incremental updates (append-only)
//! - Efficient proof generation (O(log n))
//! - Compact storage (only leaf and internal nodes)

use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Merkle tree node
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleNode {
    /// Node hash
    pub hash: [u8; 32],
    /// Left child hash (if internal node)
    pub left: Option<[u8; 32]>,
    /// Right child hash (if internal node)
    pub right: Option<[u8; 32]>,
}

impl MerkleNode {
    /// Create leaf node
    pub fn leaf(hash: [u8; 32]) -> Self {
        Self {
            hash,
            left: None,
            right: None,
        }
    }

    /// Create internal node
    pub fn internal(left: [u8; 32], right: [u8; 32]) -> Self {
        let hash = hash_pair(&left, &right);
        Self {
            hash,
            left: Some(left),
            right: Some(right),
        }
    }

    /// Check if node is a leaf
    pub fn is_leaf(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }
}

/// Hash a pair of hashes (used for internal nodes)
fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Merkle proof (path from leaf to root)
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// Leaf hash being proven
    pub leaf_hash: [u8; 32],
    /// Sibling hashes along the path to root
    pub siblings: Vec<(Direction, [u8; 32])>,
    /// Root hash
    pub root_hash: [u8; 32],
}

/// Direction of sibling in Merkle tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Sibling is on the left
    Left,
    /// Sibling is on the right
    Right,
}

impl MerkleProof {
    /// Verify proof against a root hash
    pub fn verify(&self) -> bool {
        let mut current_hash = self.leaf_hash;

        // Walk up the tree, hashing with siblings
        for (direction, sibling_hash) in &self.siblings {
            current_hash = match direction {
                Direction::Left => hash_pair(sibling_hash, &current_hash),
                Direction::Right => hash_pair(&current_hash, sibling_hash),
            };
        }

        current_hash == self.root_hash
    }
}

/// Incremental Merkle tree
pub struct MerkleTree {
    /// Leaf hashes (indexed by position)
    leaves: Vec<[u8; 32]>,
    /// Cached root hash
    cached_root: Option<[u8; 32]>,
}

impl MerkleTree {
    /// Create empty tree
    pub fn new() -> Self {
        Self {
            leaves: Vec::new(),
            cached_root: None,
        }
    }

    /// Create tree from existing leaves
    pub fn from_leaves(leaves: Vec<[u8; 32]>) -> Self {
        let mut tree = Self::new();
        for leaf in leaves {
            tree.append(leaf);
        }
        tree
    }

    /// Append a new leaf
    pub fn append(&mut self, leaf_hash: [u8; 32]) {
        self.leaves.push(leaf_hash);
        self.cached_root = None; // Invalidate cache
    }

    /// Get number of leaves
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Check if tree is empty
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// Compute Merkle root
    pub fn root(&mut self) -> [u8; 32] {
        if let Some(root) = self.cached_root {
            return root;
        }

        if self.leaves.is_empty() {
            return [0u8; 32];
        }

        if self.leaves.len() == 1 {
            self.cached_root = Some(self.leaves[0]);
            return self.leaves[0];
        }

        let root = Self::compute_root(&self.leaves);
        self.cached_root = Some(root);
        root
    }

    /// Compute root from leaves (internal)
    fn compute_root(leaves: &[[u8; 32]]) -> [u8; 32] {
        if leaves.is_empty() {
            return [0u8; 32];
        }

        if leaves.len() == 1 {
            return leaves[0];
        }

        let mut current_level = leaves.to_vec();

        while current_level.len() > 1 {
            let mut next_level = Vec::new();

            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    // Duplicate last hash if odd
                    current_level[i]
                };

                next_level.push(hash_pair(&left, &right));
            }

            current_level = next_level;
        }

        current_level[0]
    }

    /// Generate Merkle proof for a leaf at given index
    pub fn generate_proof(&mut self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.leaves.len() {
            return None;
        }

        let leaf_hash = self.leaves[leaf_index];
        let root_hash = self.root();
        let mut siblings = Vec::new();

        if self.leaves.len() == 1 {
            // Single leaf - no siblings
            return Some(MerkleProof {
                leaf_hash,
                siblings,
                root_hash,
            });
        }

        let mut current_level = self.leaves.clone();
        let mut current_index = leaf_index;

        // Walk up the tree, collecting siblings
        while current_level.len() > 1 {
            // Find sibling
            let is_left = current_index % 2 == 0;
            let sibling_index = if is_left {
                if current_index + 1 < current_level.len() {
                    current_index + 1
                } else {
                    // No sibling, duplicate self
                    current_index
                }
            } else {
                current_index - 1
            };

            let sibling_hash = current_level[sibling_index];
            let direction = if is_left {
                Direction::Right
            } else {
                Direction::Left
            };

            siblings.push((direction, sibling_hash));

            // Move to parent level
            let mut next_level = Vec::new();
            for i in (0..current_level.len()).step_by(2) {
                let left = current_level[i];
                let right = if i + 1 < current_level.len() {
                    current_level[i + 1]
                } else {
                    current_level[i]
                };
                next_level.push(hash_pair(&left, &right));
            }

            current_level = next_level;
            current_index /= 2;
        }

        Some(MerkleProof {
            leaf_hash,
            siblings,
            root_hash,
        })
    }

    /// Verify that a leaf exists in the tree
    pub fn verify_leaf(&mut self, leaf_index: usize, leaf_hash: [u8; 32]) -> bool {
        if leaf_index >= self.leaves.len() {
            return false;
        }

        if self.leaves[leaf_index] != leaf_hash {
            return false;
        }

        let proof = match self.generate_proof(leaf_index) {
            Some(p) => p,
            None => return false,
        };

        proof.verify()
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash_data(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    #[test]
    fn test_empty_tree() {
        let mut tree = MerkleTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        assert_eq!(tree.root(), [0u8; 32]);
    }

    #[test]
    fn test_single_leaf() {
        let mut tree = MerkleTree::new();
        let leaf = hash_data(b"leaf1");
        tree.append(leaf);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree.root(), leaf);
    }

    #[test]
    fn test_two_leaves() {
        let mut tree = MerkleTree::new();
        let leaf1 = hash_data(b"leaf1");
        let leaf2 = hash_data(b"leaf2");

        tree.append(leaf1);
        tree.append(leaf2);

        assert_eq!(tree.len(), 2);

        let expected_root = hash_pair(&leaf1, &leaf2);
        assert_eq!(tree.root(), expected_root);
    }

    #[test]
    fn test_four_leaves() {
        let mut tree = MerkleTree::new();
        let leaves = vec![
            hash_data(b"leaf1"),
            hash_data(b"leaf2"),
            hash_data(b"leaf3"),
            hash_data(b"leaf4"),
        ];

        for leaf in &leaves {
            tree.append(*leaf);
        }

        assert_eq!(tree.len(), 4);

        // Manually compute expected root
        let h01 = hash_pair(&leaves[0], &leaves[1]);
        let h23 = hash_pair(&leaves[2], &leaves[3]);
        let expected_root = hash_pair(&h01, &h23);

        assert_eq!(tree.root(), expected_root);
    }

    #[test]
    fn test_odd_number_leaves() {
        let mut tree = MerkleTree::new();
        let leaves = vec![
            hash_data(b"leaf1"),
            hash_data(b"leaf2"),
            hash_data(b"leaf3"),
        ];

        for leaf in &leaves {
            tree.append(*leaf);
        }

        assert_eq!(tree.len(), 3);

        // With odd number, last leaf is duplicated
        let h01 = hash_pair(&leaves[0], &leaves[1]);
        let h22 = hash_pair(&leaves[2], &leaves[2]);
        let expected_root = hash_pair(&h01, &h22);

        assert_eq!(tree.root(), expected_root);
    }

    #[test]
    fn test_proof_generation_single_leaf() {
        let mut tree = MerkleTree::new();
        let leaf = hash_data(b"leaf1");
        tree.append(leaf);

        let proof = tree.generate_proof(0).unwrap();
        assert_eq!(proof.leaf_hash, leaf);
        assert_eq!(proof.siblings.len(), 0);
        assert_eq!(proof.root_hash, leaf);
        assert!(proof.verify());
    }

    #[test]
    fn test_proof_generation_two_leaves() {
        let mut tree = MerkleTree::new();
        let leaf1 = hash_data(b"leaf1");
        let leaf2 = hash_data(b"leaf2");

        tree.append(leaf1);
        tree.append(leaf2);

        // Proof for leaf 0
        let proof0 = tree.generate_proof(0).unwrap();
        assert_eq!(proof0.leaf_hash, leaf1);
        assert_eq!(proof0.siblings.len(), 1);
        assert_eq!(proof0.siblings[0], (Direction::Right, leaf2));
        assert!(proof0.verify());

        // Proof for leaf 1
        let proof1 = tree.generate_proof(1).unwrap();
        assert_eq!(proof1.leaf_hash, leaf2);
        assert_eq!(proof1.siblings.len(), 1);
        assert_eq!(proof1.siblings[0], (Direction::Left, leaf1));
        assert!(proof1.verify());
    }

    #[test]
    fn test_proof_generation_four_leaves() {
        let mut tree = MerkleTree::new();
        let leaves = vec![
            hash_data(b"leaf1"),
            hash_data(b"leaf2"),
            hash_data(b"leaf3"),
            hash_data(b"leaf4"),
        ];

        for leaf in &leaves {
            tree.append(*leaf);
        }

        // Proof for leaf 0
        let proof = tree.generate_proof(0).unwrap();
        assert_eq!(proof.leaf_hash, leaves[0]);
        assert_eq!(proof.siblings.len(), 2);
        assert!(proof.verify());

        // Proof for leaf 2
        let proof = tree.generate_proof(2).unwrap();
        assert_eq!(proof.leaf_hash, leaves[2]);
        assert_eq!(proof.siblings.len(), 2);
        assert!(proof.verify());
    }

    #[test]
    fn test_proof_verification_invalid() {
        let mut tree = MerkleTree::new();
        let leaf1 = hash_data(b"leaf1");
        let leaf2 = hash_data(b"leaf2");

        tree.append(leaf1);
        tree.append(leaf2);

        let mut proof = tree.generate_proof(0).unwrap();

        // Tamper with root
        proof.root_hash = hash_data(b"fake_root");
        assert!(!proof.verify());
    }

    #[test]
    fn test_verify_leaf() {
        let mut tree = MerkleTree::new();
        let leaves = vec![
            hash_data(b"leaf1"),
            hash_data(b"leaf2"),
            hash_data(b"leaf3"),
        ];

        for leaf in &leaves {
            tree.append(*leaf);
        }

        // Valid leaves
        assert!(tree.verify_leaf(0, leaves[0]));
        assert!(tree.verify_leaf(1, leaves[1]));
        assert!(tree.verify_leaf(2, leaves[2]));

        // Invalid index
        assert!(!tree.verify_leaf(3, leaves[0]));

        // Wrong hash
        assert!(!tree.verify_leaf(0, leaves[1]));
    }

    #[test]
    fn test_incremental_updates() {
        let mut tree = MerkleTree::new();

        let leaf1 = hash_data(b"leaf1");
        tree.append(leaf1);
        let root1 = tree.root();

        let leaf2 = hash_data(b"leaf2");
        tree.append(leaf2);
        let root2 = tree.root();

        // Root should change after append
        assert_ne!(root1, root2);

        // Expected root after 2 leaves
        let expected_root2 = hash_pair(&leaf1, &leaf2);
        assert_eq!(root2, expected_root2);
    }

    #[test]
    fn test_from_leaves() {
        let leaves = vec![
            hash_data(b"leaf1"),
            hash_data(b"leaf2"),
            hash_data(b"leaf3"),
        ];

        let mut tree = MerkleTree::from_leaves(leaves.clone());

        assert_eq!(tree.len(), 3);

        // Verify all leaves
        for (i, leaf) in leaves.iter().enumerate() {
            assert!(tree.verify_leaf(i, *leaf));
        }
    }
}