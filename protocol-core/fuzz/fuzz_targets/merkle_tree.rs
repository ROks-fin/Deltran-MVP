#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol_core::merkle::{MerkleTree, hash_data};

fuzz_target!(|data: &[u8]| {
    // Skip very small inputs
    if data.len() < 32 {
        return;
    }

    // Split data into variable-length chunks (leaf hashes)
    let chunk_size = ((data[0] as usize) % 32) + 1;
    let num_chunks = data.len() / chunk_size;

    if num_chunks == 0 {
        return;
    }

    let mut leaves = Vec::new();
    for i in 0..num_chunks.min(100) {  // Limit to 100 leaves to avoid OOM
        let start = i * chunk_size;
        let end = ((i + 1) * chunk_size).min(data.len());
        if start < end {
            leaves.push(hash_data(&data[start..end]));
        }
    }

    if leaves.is_empty() {
        return;
    }

    // Build tree (should not panic)
    let tree = match MerkleTree::build(leaves.clone()) {
        Ok(t) => t,
        Err(_) => return,
    };

    // Verify tree integrity
    assert!(tree.verify(), "Tree integrity check failed");

    // Generate and verify proofs for all leaves
    for i in 0..leaves.len() {
        if let Ok(proof) = tree.prove(i) {
            // Proof verification should always succeed for valid tree
            assert!(proof.verify(), "Proof verification failed for leaf {}", i);

            // Root should match tree root
            if let Ok(tree_root) = tree.root() {
                assert_eq!(proof.root, tree_root, "Proof root mismatch");
            }
        }
    }
});
