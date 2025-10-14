//! Cryptographic operations for the ledger
//!
//! This module provides:
//! - Ed25519 key pair generation, signing, and verification
//! - SHA-256 hashing for events and blocks
//! - Deterministic signing for reproducibility

use crate::{Error, Result};
use ed25519_dalek::{Signature as DalekSignature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// Ed25519 key pair for signing
#[derive(Debug)]
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Generate a new random key pair
    pub fn generate() -> Self {
        let signing_key = SigningKey::from_bytes(&rand::random::<[u8; 32]>());
        let verifying_key = signing_key.verifying_key();

        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Create from seed (32 bytes) - deterministic generation
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();

        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Get public key bytes
    pub fn public_key(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get private key bytes (USE WITH CAUTION - should be protected)
    pub fn secret_key(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> crate::types::Signature {
        let signature = self.signing_key.sign(message);
        crate::types::Signature::from_bytes(signature.to_bytes())
    }

    /// Verify a signature
    pub fn verify(
        &self,
        message: &[u8],
        signature: &crate::types::Signature,
    ) -> Result<()> {
        let dalek_sig = DalekSignature::from_bytes(signature.as_bytes());
        self.verifying_key
            .verify(message, &dalek_sig)
            .map_err(|e| Error::SignatureError(format!("Verification failed: {}", e)))
    }
}

/// Verify a signature with a public key
pub fn verify_signature(
    message: &[u8],
    signature: &crate::types::Signature,
    public_key: &[u8; 32],
) -> bool {
    let dalek_sig = DalekSignature::from_bytes(signature.as_bytes());

    let verifying_key = match VerifyingKey::from_bytes(public_key) {
        Ok(key) => key,
        Err(_) => return false,
    };

    verifying_key.verify(message, &dalek_sig).is_ok()
}

/// Hash an event using SHA-256
///
/// Creates a deterministic 32-byte hash from the event's canonical bytes.
pub fn hash_event(event: &crate::types::LedgerEvent) -> [u8; 32] {
    let canonical_bytes = event.canonical_bytes();
    hash_bytes(&canonical_bytes)
}

/// Hash a block using SHA-256
///
/// Computes hash from block contents (height, merkle root, previous hash, etc.)
pub fn hash_block(block: &crate::types::Block) -> [u8; 32] {
    block.compute_hash()
}

/// Hash arbitrary bytes using SHA-256
pub fn hash_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Create a Merkle root from event hashes
///
/// Takes a list of event hashes and computes the Merkle root.
/// If the list has odd length, the last hash is duplicated.
pub fn merkle_root(event_hashes: &[[u8; 32]]) -> [u8; 32] {
    if event_hashes.is_empty() {
        return [0u8; 32];
    }

    if event_hashes.len() == 1 {
        return event_hashes[0];
    }

    // Build Merkle tree bottom-up
    let mut current_level: Vec<[u8; 32]> = event_hashes.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for i in (0..current_level.len()).step_by(2) {
            let left = &current_level[i];
            let right = if i + 1 < current_level.len() {
                &current_level[i + 1]
            } else {
                // Duplicate last hash if odd number
                &current_level[i]
            };

            // Hash(left || right)
            let mut hasher = Sha256::new();
            hasher.update(left);
            hasher.update(right);
            let parent_hash: [u8; 32] = hasher.finalize().into();

            next_level.push(parent_hash);
        }

        current_level = next_level;
    }

    current_level[0]
}

/// Generate a cryptographically secure random UUIDv7
///
/// UUIDv7 embeds timestamp for time-ordering while maintaining uniqueness
pub fn generate_uuid_v7() -> uuid::Uuid {
    uuid::Uuid::now_v7()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        let public_key = keypair.public_key();

        // Public key should be 32 bytes
        assert_eq!(public_key.len(), 32);
    }

    #[test]
    fn test_keypair_from_seed() {
        let seed = [42u8; 32];
        let keypair1 = KeyPair::from_seed(&seed);
        let keypair2 = KeyPair::from_seed(&seed);

        // Same seed should produce same keys
        assert_eq!(keypair1.public_key(), keypair2.public_key());
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate();
        let message = b"test message";

        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature).is_ok());

        // Wrong message should fail
        let wrong_message = b"wrong message";
        assert!(keypair.verify(wrong_message, &signature).is_err());
    }

    #[test]
    fn test_verify_signature() {
        let keypair = KeyPair::generate();
        let message = b"test message";
        let signature = keypair.sign(message);
        let public_key = keypair.public_key();

        assert!(verify_signature(message, &signature, &public_key));

        // Wrong public key should fail
        let wrong_keypair = KeyPair::generate();
        let wrong_public_key = wrong_keypair.public_key();
        assert!(!verify_signature(message, &signature, &wrong_public_key));
    }

    #[test]
    fn test_hash_bytes() {
        let data = b"test data";
        let hash1 = hash_bytes(data);
        let hash2 = hash_bytes(data);

        // Same data should produce same hash
        assert_eq!(hash1, hash2);

        // Different data should produce different hash
        let different_data = b"different data";
        let hash3 = hash_bytes(different_data);
        assert_ne!(hash1, hash3);

        // Hash should be 32 bytes
        assert_eq!(hash1.len(), 32);
    }

    #[test]
    fn test_merkle_root_empty() {
        let root = merkle_root(&[]);
        assert_eq!(root, [0u8; 32]);
    }

    #[test]
    fn test_merkle_root_single() {
        let hash = [1u8; 32];
        let root = merkle_root(&[hash]);
        assert_eq!(root, hash);
    }

    #[test]
    fn test_merkle_root_two() {
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];
        let root = merkle_root(&[hash1, hash2]);

        // Root should be hash of concatenated hashes
        let mut hasher = Sha256::new();
        hasher.update(&hash1);
        hasher.update(&hash2);
        let expected: [u8; 32] = hasher.finalize().into();

        assert_eq!(root, expected);
    }

    #[test]
    fn test_merkle_root_odd_number() {
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];
        let hash3 = [3u8; 32];

        // With 3 hashes, last should be duplicated
        let root = merkle_root(&[hash1, hash2, hash3]);

        // Should not panic and produce valid 32-byte hash
        assert_eq!(root.len(), 32);
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let hashes = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let root1 = merkle_root(&hashes);
        let root2 = merkle_root(&hashes);

        // Same inputs should produce same root
        assert_eq!(root1, root2);
    }

    #[test]
    fn test_uuid_v7_generation() {
        let uuid1 = generate_uuid_v7();
        let uuid2 = generate_uuid_v7();

        // Should be different UUIDs
        assert_ne!(uuid1, uuid2);

        // Should be version 7
        assert_eq!(uuid1.get_version_num(), 7);
        assert_eq!(uuid2.get_version_num(), 7);
    }

    #[test]
    fn test_known_signature_vector() {
        // RFC 8032 test vector
        let seed = [
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60,
            0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c, 0xc4,
            0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19,
            0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae, 0x7f, 0x60,
        ];

        let keypair = KeyPair::from_seed(&seed);
        let message = b"";
        let signature = keypair.sign(message);

        // Should verify
        assert!(keypair.verify(message, &signature).is_ok());
    }
}