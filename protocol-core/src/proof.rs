//! Settlement proof generation
//!
//! Generates cryptographic proofs of settlement with:
//! - BFT validator multi-sig (5 of 7)
//! - HSM coordinator signature
//! - Merkle inclusion proofs

use crate::{hsm::Hsm, merkle::*, types::*, Error, Result};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

/// Proof generator
pub struct ProofGenerator {
    hsm: Arc<dyn Hsm>,
    network_id: String,
    proto_version: u16,
}

impl ProofGenerator {
    /// Create new proof generator
    pub fn new(hsm: Arc<dyn Hsm>, network_id: String, proto_version: u16) -> Self {
        Self {
            hsm,
            network_id,
            proto_version,
        }
    }

    /// Generate settlement proof
    pub fn generate_settlement_proof(
        &self,
        batch_id: Uuid,
        checkpoint_height: u64,
        prev_checkpoint_id: [u8; 32],
        app_hash: [u8; 32],
        payment_hashes: Vec<[u8; 32]>,
        summary: BatchSummary,
        authorized_parties: Vec<String>,
    ) -> Result<SettlementProof> {
        // Build Merkle tree
        let merkle_tree = MerkleTree::build(payment_hashes.clone())?;
        let merkle_root = merkle_tree.root()?;

        // Generate Merkle proofs for each payment
        let mut merkle_paths = Vec::new();
        for (index, payment_hash) in payment_hashes.iter().enumerate() {
            let proof = merkle_tree.prove(index)?;
            merkle_paths.push(MerkleProofPath {
                payment_id: Uuid::new_v4(), // TODO: Pass actual payment IDs
                leaf_hash: *payment_hash,
                sibling_hashes: proof.sibling_hashes,
                leaf_index: index as u32,
            });
        }

        let now = Utc::now();

        let mut proof = SettlementProof {
            proof_id: Uuid::new_v4(),
            batch_id,
            checkpoint_height,
            merkle_root,
            merkle_paths,
            app_hash,
            prev_checkpoint_id,
            network_id: self.network_id.clone(),
            proto_version: self.proto_version,
            batch_finalized_at: now,
            proof_generated_at: now,
            validator_signatures: vec![], // Collected externally
            hsm_signature: self.generate_hsm_signature(&[0u8; 32])?, // Placeholder
            summary,
            authorized_parties,
        };

        // Generate HSM signature over canonical bytes
        proof.hsm_signature = self.generate_hsm_signature(&proof.canonical_bytes())?;

        Ok(proof)
    }

    /// Generate HSM signature
    fn generate_hsm_signature(&self, data: &[u8]) -> Result<HsmSignature> {
        let signature = self.hsm.sign(data)?;
        let public_key = self.hsm.public_key()?;

        Ok(HsmSignature {
            hsm_key_id: self.hsm.key_id().to_string(),
            key_epoch: self.hsm.key_epoch().to_string(),
            algorithm: SignatureAlgorithm::Ed25519,
            signature,
            public_key,
            signed_at: Utc::now(),
        })
    }

    /// Add validator signature
    pub fn add_validator_signature(
        &self,
        proof: &mut SettlementProof,
        validator_id: String,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    ) {
        proof.validator_signatures.push(ValidatorSignature {
            validator_id,
            public_key,
            signature,
            signed_at: Utc::now(),
        });
    }

    /// Verify settlement proof
    pub fn verify(&self, proof: &SettlementProof) -> Result<()> {
        // 1. Verify BFT quorum (5 of 7)
        if proof.validator_signatures.len() < 5 {
            return Err(Error::QuorumNotMet {
                actual: proof.validator_signatures.len(),
                required: 5,
            });
        }

        // 2. Verify HSM signature
        crate::hsm::verify_hsm_signature(
            &proof.canonical_bytes(),
            &proof.hsm_signature.signature,
            &proof.hsm_signature.public_key,
        )?;

        // 3. Verify each validator signature
        let canonical = proof.canonical_bytes();
        for sig in &proof.validator_signatures {
            crate::hsm::verify_hsm_signature(&canonical, &sig.signature, &sig.public_key)?;
        }

        // 4. Verify Merkle proofs
        for merkle_path in &proof.merkle_paths {
            let merkle_proof = MerkleProof {
                leaf_hash: merkle_path.leaf_hash,
                leaf_index: merkle_path.leaf_index as usize,
                sibling_hashes: merkle_path.sibling_hashes.clone(),
                root: proof.merkle_root,
            };

            if !merkle_proof.verify() {
                return Err(Error::MerkleProofInvalid(format!(
                    "Payment {} failed Merkle verification",
                    merkle_path.payment_id
                )));
            }
        }

        Ok(())
    }

    /// Check if party is authorized to access proof
    pub fn is_authorized(&self, proof: &SettlementProof, requester_id: &str) -> bool {
        proof.authorized_parties.contains(&requester_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hsm::HsmMock;
    use crate::merkle::hash_data;
    use rust_decimal::Decimal;

    fn create_test_hsm() -> Arc<dyn Hsm> {
        Arc::new(HsmMock::new("proof-key".into(), "2025-Q1".into()))
    }

    #[test]
    fn test_proof_generation() {
        let hsm = create_test_hsm();
        let generator = ProofGenerator::new(hsm, "deltran-testnet".into(), 1);

        let payment_hashes = vec![
            hash_data(b"payment1"),
            hash_data(b"payment2"),
            hash_data(b"payment3"),
        ];

        let summary = BatchSummary {
            corridor_id: "UAE-IND".into(),
            currency: "USD".into(),
            payment_count: 3,
            bank_count: 2,
            gross_amount: Decimal::new(100000, 2),
            net_amount: Decimal::new(30000, 2),
            netting_efficiency: 0.70,
            net_transfer_count: 1,
            partial_settlement: false,
            requeued_count: 0,
        };

        let proof = generator
            .generate_settlement_proof(
                Uuid::new_v4(),
                100,
                [0u8; 32],
                [1u8; 32],
                payment_hashes,
                summary,
                vec!["BANKGB2L".into(), "CHASUS33".into()],
            )
            .unwrap();

        assert_eq!(proof.summary.payment_count, 3);
        assert_eq!(proof.merkle_paths.len(), 3);
        assert!(proof.hsm_signature.signature.len() > 0);
    }

    #[test]
    fn test_proof_verification_without_validators() {
        let hsm = create_test_hsm();
        let generator = ProofGenerator::new(hsm, "deltran-testnet".into(), 1);

        let payment_hashes = vec![hash_data(b"payment1")];

        let summary = BatchSummary {
            corridor_id: "UAE-IND".into(),
            currency: "USD".into(),
            payment_count: 1,
            bank_count: 2,
            gross_amount: Decimal::ZERO,
            net_amount: Decimal::ZERO,
            netting_efficiency: 0.0,
            net_transfer_count: 0,
            partial_settlement: false,
            requeued_count: 0,
        };

        let proof = generator
            .generate_settlement_proof(
                Uuid::new_v4(),
                100,
                [0u8; 32],
                [1u8; 32],
                payment_hashes,
                summary,
                vec![],
            )
            .unwrap();

        // Should fail: no validator signatures
        assert!(generator.verify(&proof).is_err());
    }

    #[test]
    fn test_authorization_check() {
        let hsm = create_test_hsm();
        let generator = ProofGenerator::new(hsm, "deltran-testnet".into(), 1);

        let payment_hashes = vec![hash_data(b"payment1")];
        let summary = BatchSummary {
            corridor_id: "UAE-IND".into(),
            currency: "USD".into(),
            payment_count: 1,
            bank_count: 2,
            gross_amount: Decimal::ZERO,
            net_amount: Decimal::ZERO,
            netting_efficiency: 0.0,
            net_transfer_count: 0,
            partial_settlement: false,
            requeued_count: 0,
        };

        let proof = generator
            .generate_settlement_proof(
                Uuid::new_v4(),
                100,
                [0u8; 32],
                [1u8; 32],
                payment_hashes,
                summary,
                vec!["BANKGB2L".into()],
            )
            .unwrap();

        assert!(generator.is_authorized(&proof, "BANKGB2L"));
        assert!(!generator.is_authorized(&proof, "UNAUTHORIZEDBANK"));
    }
}