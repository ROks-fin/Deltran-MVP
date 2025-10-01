// consensus/batch_verifier.rs
// Batch signature verification for 10k TPS
// Moves HSM operations off hot path using Merkle proofs

use blake3::Hash;
use ed25519_dalek::{PublicKey, Signature, Verifier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

const BATCH_WINDOW_MS: u64 = 500; // 500ms batching window
const MAX_BATCH_SIZE: usize = 5000; // Max payments per batch

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub payment_id: Uuid,
    pub data: Vec<u8>, // Serialized payment data
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub checkpoint_id: Uuid,
    pub shard_id: u32,
    pub merkle_root: [u8; 32],
    pub signature: Vec<u8>,
    pub payment_count: usize,
    pub metadata: CheckpointMetadata,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub checkpoint_id: Uuid,
    pub shard_id: u32,
    pub payment_count: usize,
    pub previous_checkpoint: Option<Uuid>,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<[u8; 32]>,
    layers: Vec<Vec<[u8; 32]>>,
}

impl MerkleTree {
    pub fn new(leaves: Vec<[u8; 32]>) -> Self {
        if leaves.is_empty() {
            panic!("Cannot create Merkle tree with no leaves");
        }

        let mut layers = vec![leaves.clone()];
        let mut current_layer = leaves;

        // Build tree bottom-up
        while current_layer.len() > 1 {
            let mut next_layer = Vec::new();

            for chunk in current_layer.chunks(2) {
                let hash = if chunk.len() == 2 {
                    Self::hash_pair(&chunk[0], &chunk[1])
                } else {
                    // Odd leaf - hash with itself
                    Self::hash_pair(&chunk[0], &chunk[0])
                };
                next_layer.push(hash);
            }

            layers.push(next_layer.clone());
            current_layer = next_layer;
        }

        Self { leaves, layers }
    }

    pub fn root(&self) -> [u8; 32] {
        self.layers.last().unwrap()[0]
    }

    pub fn proof(&self, leaf_index: usize) -> MerkleProof {
        let mut proof_hashes = Vec::new();
        let mut index = leaf_index;

        for layer in &self.layers[..self.layers.len() - 1] {
            let sibling_index = if index % 2 == 0 { index + 1 } else { index - 1 };

            if sibling_index < layer.len() {
                proof_hashes.push((layer[sibling_index], index % 2 == 0));
            }

            index /= 2;
        }

        MerkleProof {
            leaf_index,
            proof_hashes,
        }
    }

    fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut combined = Vec::with_capacity(64);
        combined.extend_from_slice(left);
        combined.extend_from_slice(right);
        blake3::hash(&combined).into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_index: usize,
    pub proof_hashes: Vec<([u8; 32], bool)>, // (hash, is_left_sibling)
}

impl MerkleProof {
    pub fn verify(&self, leaf: &[u8; 32], root: &[u8; 32]) -> bool {
        let mut current_hash = *leaf;

        for &(sibling_hash, is_left) in &self.proof_hashes {
            current_hash = if is_left {
                MerkleTree::hash_pair(&sibling_hash, &current_hash)
            } else {
                MerkleTree::hash_pair(&current_hash, &sibling_hash)
            };
        }

        &current_hash == root
    }
}

// Batch Verifier Service
pub struct BatchVerifier {
    pending_payments: Arc<RwLock<Vec<Payment>>>,
    checkpoints: Arc<RwLock<HashMap<Uuid, Checkpoint>>>,
    public_keys: Arc<RwLock<HashMap<u32, PublicKey>>>, // shard_id -> public_key
    batch_window_ms: u64,
    max_batch_size: usize,
}

impl BatchVerifier {
    pub fn new() -> Self {
        Self {
            pending_payments: Arc::new(RwLock::new(Vec::new())),
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
            public_keys: Arc::new(RwLock::new(HashMap::new())),
            batch_window_ms: BATCH_WINDOW_MS,
            max_batch_size: MAX_BATCH_SIZE,
        }
    }

    /// Register validator public key for shard
    pub async fn register_validator_key(&self, shard_id: u32, public_key: PublicKey) {
        self.public_keys.write().await.insert(shard_id, public_key);
        info!("Registered validator public key for shard {}", shard_id);
    }

    /// Add payment to batch queue
    pub async fn enqueue_payment(&self, payment: Payment) {
        let mut pending = self.pending_payments.write().await;
        pending.push(payment);

        // Trigger batch if threshold reached
        if pending.len() >= self.max_batch_size {
            drop(pending);
            self.create_checkpoint(0).await.ok(); // Shard 0 for now
        }
    }

    /// Background task to create checkpoints periodically
    pub async fn run(&self, shard_id: u32) {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_millis(self.batch_window_ms)
        );

        loop {
            interval.tick().await;

            let pending_count = self.pending_payments.read().await.len();

            if pending_count > 0 {
                if let Err(e) = self.create_checkpoint(shard_id).await {
                    error!("Failed to create checkpoint for shard {}: {:?}", shard_id, e);
                }
            }
        }
    }

    /// Create checkpoint from pending payments
    pub async fn create_checkpoint(&self, shard_id: u32) -> Result<Checkpoint, VerifierError> {
        let mut pending = self.pending_payments.write().await;

        if pending.is_empty() {
            return Err(VerifierError::NoPendingPayments);
        }

        // Take batch
        let batch: Vec<Payment> = pending.drain(..).collect();
        let checkpoint_id = Uuid::new_v4();

        info!(
            "Creating checkpoint {} for shard {} with {} payments",
            checkpoint_id,
            shard_id,
            batch.len()
        );

        // Build Merkle tree from payment hashes
        let leaves: Vec<[u8; 32]> = batch
            .iter()
            .map(|p| blake3::hash(&p.data).into())
            .collect();

        let merkle_tree = MerkleTree::new(leaves);
        let merkle_root = merkle_tree.root();

        // Get previous checkpoint
        let previous_checkpoint = self.get_latest_checkpoint(shard_id).await;

        // Build metadata
        let metadata = CheckpointMetadata {
            checkpoint_id,
            shard_id,
            payment_count: batch.len(),
            previous_checkpoint: previous_checkpoint.map(|c| c.checkpoint_id),
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Serialize metadata
        let metadata_bytes = bincode::serialize(&metadata)
            .map_err(|e| VerifierError::SerializationError(e.to_string()))?;
        let metadata_hash: [u8; 32] = blake3::hash(&metadata_bytes).into();

        // Combine root + metadata hash
        let checkpoint_data = [merkle_root.as_slice(), metadata_hash.as_slice()].concat();

        // Sign checkpoint (HSM operation - single call for entire batch)
        let signature = self.sign_checkpoint(&checkpoint_data, shard_id).await?;

        let checkpoint = Checkpoint {
            checkpoint_id,
            shard_id,
            merkle_root,
            signature,
            payment_count: batch.len(),
            metadata,
            created_at: chrono::Utc::now().timestamp(),
        };

        // Store checkpoint
        self.checkpoints
            .write()
            .await
            .insert(checkpoint_id, checkpoint.clone());

        info!(
            "Checkpoint {} created for shard {}: {} payments, root: {}",
            checkpoint_id,
            shard_id,
            batch.len(),
            hex::encode(&merkle_root[..8])
        );

        Ok(checkpoint)
    }

    /// Sign checkpoint (HSM/validator operation)
    async fn sign_checkpoint(
        &self,
        data: &[u8],
        shard_id: u32,
    ) -> Result<Vec<u8>, VerifierError> {
        // In production: call HSM
        // For now: simulate with in-memory key

        // Placeholder: would call HSM here
        // signature = hsm_client.sign(shard_id, data).await?;

        // Mock signature for testing
        let signature = blake3::hash(data).as_bytes().to_vec();

        Ok(signature)
    }

    /// Verify payment is in checkpoint (CPU-only, no HSM)
    pub async fn verify_payment_in_checkpoint(
        &self,
        payment_id: Uuid,
        checkpoint_id: Uuid,
        merkle_proof: &MerkleProof,
    ) -> Result<bool, VerifierError> {
        let checkpoints = self.checkpoints.read().await;
        let checkpoint = checkpoints
            .get(&checkpoint_id)
            .ok_or(VerifierError::CheckpointNotFound(checkpoint_id))?;

        // Get payment data (would fetch from DB in production)
        let payment_hash = blake3::hash(payment_id.as_bytes()); // Simplified

        // Verify Merkle proof (CPU-only, ~1μs)
        if !merkle_proof.verify(payment_hash.as_bytes(), &checkpoint.merkle_root) {
            return Ok(false);
        }

        // Verify checkpoint signature (CPU-only with cached key, ~100μs)
        let public_keys = self.public_keys.read().await;
        let public_key = public_keys
            .get(&checkpoint.shard_id)
            .ok_or(VerifierError::PublicKeyNotFound(checkpoint.shard_id))?;

        // Reconstruct checkpoint data
        let metadata_bytes = bincode::serialize(&checkpoint.metadata)
            .map_err(|e| VerifierError::SerializationError(e.to_string()))?;
        let metadata_hash: [u8; 32] = blake3::hash(&metadata_bytes).into();

        let checkpoint_data = [checkpoint.merkle_root.as_slice(), metadata_hash.as_slice()].concat();

        // Verify signature
        let signature = Signature::from_bytes(&checkpoint.signature)
            .map_err(|e| VerifierError::InvalidSignature(e.to_string()))?;

        public_key
            .verify(&checkpoint_data, &signature)
            .map_err(|e| VerifierError::SignatureVerificationFailed(e.to_string()))?;

        Ok(true)
    }

    /// Batch verify multiple payments (amortized cost)
    pub async fn batch_verify_payments(
        &self,
        verifications: Vec<(Uuid, Uuid, MerkleProof)>, // (payment_id, checkpoint_id, proof)
    ) -> Result<Vec<bool>, VerifierError> {
        let mut results = Vec::new();

        // Group by checkpoint for efficient verification
        let mut by_checkpoint: HashMap<Uuid, Vec<(Uuid, MerkleProof)>> = HashMap::new();

        for (payment_id, checkpoint_id, proof) in verifications {
            by_checkpoint
                .entry(checkpoint_id)
                .or_insert_with(Vec::new)
                .push((payment_id, proof));
        }

        let checkpoints = self.checkpoints.read().await;

        // Verify each checkpoint once, then verify all proofs
        for (checkpoint_id, payments) in by_checkpoint {
            let checkpoint = checkpoints
                .get(&checkpoint_id)
                .ok_or(VerifierError::CheckpointNotFound(checkpoint_id))?;

            // Verify checkpoint signature once (amortized)
            self.verify_checkpoint_signature(checkpoint).await?;

            // Verify all Merkle proofs (CPU-only)
            for (payment_id, proof) in payments {
                let payment_hash = blake3::hash(payment_id.as_bytes());
                let valid = proof.verify(payment_hash.as_bytes(), &checkpoint.merkle_root);
                results.push(valid);
            }
        }

        Ok(results)
    }

    /// Verify checkpoint signature
    async fn verify_checkpoint_signature(
        &self,
        checkpoint: &Checkpoint,
    ) -> Result<(), VerifierError> {
        let public_keys = self.public_keys.read().await;
        let public_key = public_keys
            .get(&checkpoint.shard_id)
            .ok_or(VerifierError::PublicKeyNotFound(checkpoint.shard_id))?;

        let metadata_bytes = bincode::serialize(&checkpoint.metadata)
            .map_err(|e| VerifierError::SerializationError(e.to_string()))?;
        let metadata_hash: [u8; 32] = blake3::hash(&metadata_bytes).into();

        let checkpoint_data = [checkpoint.merkle_root.as_slice(), metadata_hash.as_slice()].concat();

        let signature = Signature::from_bytes(&checkpoint.signature)
            .map_err(|e| VerifierError::InvalidSignature(e.to_string()))?;

        public_key
            .verify(&checkpoint_data, &signature)
            .map_err(|e| VerifierError::SignatureVerificationFailed(e.to_string()))?;

        Ok(())
    }

    /// Get latest checkpoint for shard
    async fn get_latest_checkpoint(&self, shard_id: u32) -> Option<Checkpoint> {
        let checkpoints = self.checkpoints.read().await;

        checkpoints
            .values()
            .filter(|c| c.shard_id == shard_id)
            .max_by_key(|c| c.created_at)
            .cloned()
    }

    /// Get checkpoint statistics
    pub async fn get_stats(&self) -> BatchVerifierStats {
        let checkpoints = self.checkpoints.read().await;
        let pending = self.pending_payments.read().await;

        let total_checkpoints = checkpoints.len();
        let total_payments: usize = checkpoints.values().map(|c| c.payment_count).sum();
        let pending_payments = pending.len();

        let avg_batch_size = if total_checkpoints > 0 {
            total_payments / total_checkpoints
        } else {
            0
        };

        BatchVerifierStats {
            total_checkpoints,
            total_payments,
            pending_payments,
            avg_batch_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatchVerifierStats {
    pub total_checkpoints: usize,
    pub total_payments: usize,
    pub pending_payments: usize,
    pub avg_batch_size: usize,
}

#[derive(Debug)]
pub enum VerifierError {
    NoPendingPayments,
    CheckpointNotFound(Uuid),
    PublicKeyNotFound(u32),
    InvalidSignature(String),
    SignatureVerificationFailed(String),
    SerializationError(String),
}

impl std::fmt::Display for VerifierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifierError::NoPendingPayments => write!(f, "No pending payments"),
            VerifierError::CheckpointNotFound(id) => write!(f, "Checkpoint {} not found", id),
            VerifierError::PublicKeyNotFound(shard) => {
                write!(f, "Public key for shard {} not found", shard)
            }
            VerifierError::InvalidSignature(e) => write!(f, "Invalid signature: {}", e),
            VerifierError::SignatureVerificationFailed(e) => {
                write!(f, "Signature verification failed: {}", e)
            }
            VerifierError::SerializationError(e) => write!(f, "Serialization error: {}", e),
        }
    }
}

impl std::error::Error for VerifierError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree() {
        let leaves: Vec<[u8; 32]> = (0..4)
            .map(|i| {
                let data = format!("leaf-{}", i);
                blake3::hash(data.as_bytes()).into()
            })
            .collect();

        let tree = MerkleTree::new(leaves.clone());
        let root = tree.root();

        // Verify proof for each leaf
        for (i, leaf) in leaves.iter().enumerate() {
            let proof = tree.proof(i);
            assert!(proof.verify(leaf, &root));
        }
    }

    #[test]
    fn test_merkle_proof_invalid() {
        let leaves: Vec<[u8; 32]> = (0..4)
            .map(|i| blake3::hash(format!("leaf-{}", i).as_bytes()).into())
            .collect();

        let tree = MerkleTree::new(leaves);
        let root = tree.root();
        let proof = tree.proof(0);

        // Try to verify with wrong leaf
        let wrong_leaf = blake3::hash(b"wrong-leaf");
        assert!(!proof.verify(wrong_leaf.as_bytes(), &root));
    }

    #[tokio::test]
    async fn test_batch_verifier() {
        let verifier = BatchVerifier::new();

        // Add payments
        for i in 0..10 {
            let payment = Payment {
                payment_id: Uuid::new_v4(),
                data: format!("payment-{}", i).into_bytes(),
            };
            verifier.enqueue_payment(payment).await;
        }

        // Create checkpoint
        let checkpoint = verifier.create_checkpoint(0).await.unwrap();
        assert_eq!(checkpoint.payment_count, 10);
    }
}
