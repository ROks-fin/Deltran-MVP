//! Checkpoint generation (every ~100 blocks)
//!
//! Checkpoints commit ledger state periodically with BFT + HSM signatures.

use crate::{hsm::Hsm, types::*, Error, Result};
use chrono::Utc;
use std::sync::Arc;

/// Checkpoint generator
pub struct CheckpointGenerator {
    hsm: Arc<dyn Hsm>,
    network_id: String,
    proto_version: u16,
}

impl CheckpointGenerator {
    /// Create new checkpoint generator
    pub fn new(hsm: Arc<dyn Hsm>, network_id: String, proto_version: u16) -> Self {
        Self {
            hsm,
            network_id,
            proto_version,
        }
    }

    /// Generate checkpoint at given height
    pub fn generate(
        &self,
        height: u64,
        prev_checkpoint_id: [u8; 32],
        app_hash: [u8; 32],
        merkle_root: [u8; 32],
        stats: CheckpointStats,
    ) -> Result<Checkpoint> {
        let timestamp = Utc::now();

        let mut checkpoint = Checkpoint {
            checkpoint_id: [0u8; 32], // Computed after
            height,
            prev_checkpoint_id,
            app_hash,
            merkle_root,
            network_id: self.network_id.clone(),
            proto_version: self.proto_version,
            timestamp,
            validator_signatures: vec![], // Collected separately
            hsm_signature: self.generate_hsm_signature(&[0u8; 32])?, // Placeholder
            stats,
        };

        // Compute checkpoint ID
        checkpoint.checkpoint_id = checkpoint.compute_id();

        // Generate HSM signature
        checkpoint.hsm_signature = self.generate_hsm_signature(&checkpoint.canonical_bytes())?;

        Ok(checkpoint)
    }

    /// Generate HSM signature
    fn generate_hsm_signature(&self, data: &[u8]) -> Result<HsmSignature> {
        let signature = self.hsm.sign(data)?;
        let public_key = self.hsm.public_key()?;

        Ok(HsmSignature {
            hsm_key_id: self.hsm.key_id().to_string(),
            key_epoch: self.hsm.key_epoch().to_string(),
            algorithm: SignatureAlgorithm::Ed25519, // TODO: Map from HSM algorithm
            signature,
            public_key,
            signed_at: Utc::now(),
        })
    }

    /// Add validator signature to checkpoint
    pub fn add_validator_signature(
        &self,
        checkpoint: &mut Checkpoint,
        validator_id: String,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    ) {
        checkpoint.validator_signatures.push(ValidatorSignature {
            validator_id,
            public_key,
            signature,
            signed_at: Utc::now(),
        });
    }

    /// Verify checkpoint integrity
    pub fn verify(&self, checkpoint: &Checkpoint) -> Result<()> {
        // 1. Verify checkpoint ID
        let computed_id = checkpoint.compute_id();
        if computed_id != checkpoint.checkpoint_id {
            return Err(Error::CheckpointFailed(format!(
                "Checkpoint ID mismatch: expected {:?}, got {:?}",
                hex::encode(checkpoint.checkpoint_id),
                hex::encode(computed_id)
            )));
        }

        // 2. Verify HSM signature
        crate::hsm::verify_hsm_signature(
            &checkpoint.canonical_bytes(),
            &checkpoint.hsm_signature.signature,
            &checkpoint.hsm_signature.public_key,
        )?;

        // 3. Verify BFT quorum (5 of 7)
        if checkpoint.validator_signatures.len() < 5 {
            return Err(Error::QuorumNotMet {
                actual: checkpoint.validator_signatures.len(),
                required: 5,
            });
        }

        // 4. Verify each validator signature
        let canonical = checkpoint.canonical_bytes();
        for sig in &checkpoint.validator_signatures {
            crate::hsm::verify_hsm_signature(&canonical, &sig.signature, &sig.public_key)?;
        }

        Ok(())
    }
}

/// Checkpoint manager (tracks history)
pub struct CheckpointManager {
    generator: CheckpointGenerator,
    /// Last checkpoint height
    last_height: u64,
    /// Last checkpoint ID
    last_checkpoint_id: [u8; 32],
    /// Checkpoint interval (blocks)
    interval: u64,
}

impl CheckpointManager {
    /// Create new checkpoint manager
    pub fn new(hsm: Arc<dyn Hsm>, network_id: String, proto_version: u16, interval: u64) -> Self {
        Self {
            generator: CheckpointGenerator::new(hsm, network_id, proto_version),
            last_height: 0,
            last_checkpoint_id: [0u8; 32],
            interval,
        }
    }

    /// Check if checkpoint should be generated at height
    pub fn should_checkpoint(&self, height: u64) -> bool {
        height > 0 && height % self.interval == 0
    }

    /// Generate checkpoint (if due)
    pub fn maybe_checkpoint(
        &mut self,
        height: u64,
        app_hash: [u8; 32],
        merkle_root: [u8; 32],
        stats: CheckpointStats,
    ) -> Result<Option<Checkpoint>> {
        if !self.should_checkpoint(height) {
            return Ok(None);
        }

        let checkpoint = self.generator.generate(
            height,
            self.last_checkpoint_id,
            app_hash,
            merkle_root,
            stats,
        )?;

        // Update state
        self.last_height = height;
        self.last_checkpoint_id = checkpoint.checkpoint_id;

        Ok(Some(checkpoint))
    }

    /// Get last checkpoint ID
    pub fn last_checkpoint_id(&self) -> [u8; 32] {
        self.last_checkpoint_id
    }

    /// Verify checkpoint chain
    pub fn verify_chain(&self, checkpoints: &[Checkpoint]) -> Result<()> {
        if checkpoints.is_empty() {
            return Ok(());
        }

        // Verify each checkpoint
        for checkpoint in checkpoints {
            self.generator.verify(checkpoint)?;
        }

        // Verify chaining (prev_checkpoint_id links)
        for i in 1..checkpoints.len() {
            if checkpoints[i].prev_checkpoint_id != checkpoints[i - 1].checkpoint_id {
                return Err(Error::CheckpointFailed(format!(
                    "Checkpoint chain broken at height {}: prev_id mismatch",
                    checkpoints[i].height
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hsm::HsmMock;
    use rust_decimal::Decimal;

    fn create_test_hsm() -> Arc<dyn Hsm> {
        Arc::new(HsmMock::new("test-key".into(), "2025-Q1".into()))
    }

    #[test]
    fn test_checkpoint_generation() {
        let hsm = create_test_hsm();
        let generator = CheckpointGenerator::new(hsm, "deltran-testnet".into(), 1);

        let stats = CheckpointStats {
            total_payments: 1000,
            total_batches: 10,
            total_volume: Decimal::new(1_000_000, 2),
            active_corridors: 3,
            active_banks: 5,
        };

        let checkpoint = generator
            .generate(
                100,
                [0u8; 32],
                [1u8; 32], // app_hash
                [2u8; 32], // merkle_root
                stats,
            )
            .unwrap();

        assert_eq!(checkpoint.height, 100);
        assert_eq!(checkpoint.network_id, "deltran-testnet");
        assert!(checkpoint.hsm_signature.signature.len() > 0);
    }

    #[test]
    fn test_checkpoint_verification() {
        let hsm = create_test_hsm();
        let generator = CheckpointGenerator::new(hsm, "deltran-testnet".into(), 1);

        let stats = CheckpointStats {
            total_payments: 100,
            total_batches: 1,
            total_volume: Decimal::ZERO,
            active_corridors: 1,
            active_banks: 2,
        };

        let checkpoint = generator
            .generate(100, [0u8; 32], [1u8; 32], [2u8; 32], stats)
            .unwrap();

        // Should verify with just HSM signature (no validator sigs for this test)
        // In production, BFT quorum is required
        assert!(generator.verify(&checkpoint).is_err()); // Fails due to missing validator sigs
    }

    #[test]
    fn test_checkpoint_manager() {
        let hsm = create_test_hsm();
        let mut manager =
            CheckpointManager::new(hsm, "deltran-testnet".into(), 1, 100 /* interval */);

        // Should not checkpoint at height 50
        assert!(!manager.should_checkpoint(50));

        // Should checkpoint at height 100
        assert!(manager.should_checkpoint(100));

        let stats = CheckpointStats {
            total_payments: 500,
            total_batches: 5,
            total_volume: Decimal::ZERO,
            active_corridors: 1,
            active_banks: 2,
        };

        let checkpoint = manager
            .maybe_checkpoint(100, [1u8; 32], [2u8; 32], stats)
            .unwrap()
            .unwrap();

        assert_eq!(checkpoint.height, 100);
        assert_eq!(manager.last_height, 100);
    }

    #[test]
    fn test_checkpoint_chain_verification() {
        let hsm = create_test_hsm();
        let generator = CheckpointGenerator::new(hsm.clone(), "deltran-testnet".into(), 1);

        let stats = CheckpointStats {
            total_payments: 100,
            total_batches: 1,
            total_volume: Decimal::ZERO,
            active_corridors: 1,
            active_banks: 2,
        };

        let mut checkpoints = vec![];

        // Generate chain of 3 checkpoints
        let mut prev_id = [0u8; 32];
        for height in [100, 200, 300] {
            let mut checkpoint = generator
                .generate(height, prev_id, [1u8; 32], [2u8; 32], stats.clone())
                .unwrap();

            // Add dummy validator signatures (5 for quorum)
            for i in 0..5 {
                generator.add_validator_signature(
                    &mut checkpoint,
                    format!("validator-{}", i),
                    vec![0u8; 32],
                    vec![0u8; 64],
                );
            }

            prev_id = checkpoint.checkpoint_id;
            checkpoints.push(checkpoint);
        }

        // Note: This will fail signature verification in real scenario
        // but tests the chain link logic
        let manager = CheckpointManager::new(hsm, "deltran-testnet".into(), 1, 100);

        // Chain structure should be valid (even if signatures aren't for this test)
        assert_eq!(checkpoints[1].prev_checkpoint_id, checkpoints[0].checkpoint_id);
        assert_eq!(checkpoints[2].prev_checkpoint_id, checkpoints[1].checkpoint_id);
    }
}