//! Canonical serialization for cryptographic hashing
//!
//! Ensures deterministic byte representation for signing and verification.
//! Uses fixed field order, fixed-scale decimals, and sorted collections.

use crate::{types::*, Error, Result};
use rust_decimal::Decimal;
use sha3::{Digest, Sha3_256};
use std::io::Write;

/// Canonical serializer
pub struct CanonicalSerializer {
    buffer: Vec<u8>,
}

impl CanonicalSerializer {
    /// Create new serializer
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Write bytes
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.buffer
            .write_all(bytes)
            .map_err(|e| Error::Protocol(format!("Write error: {}", e)))
    }

    /// Write string (length-prefixed)
    fn write_string(&mut self, s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        self.write_u32(bytes.len() as u32)?;
        self.write_bytes(bytes)
    }

    /// Write u32 (big-endian)
    fn write_u32(&mut self, n: u32) -> Result<()> {
        self.write_bytes(&n.to_be_bytes())
    }

    /// Write u64 (big-endian)
    fn write_u64(&mut self, n: u64) -> Result<()> {
        self.write_bytes(&n.to_be_bytes())
    }

    /// Write i64 (big-endian)
    fn write_i64(&mut self, n: i64) -> Result<()> {
        self.write_bytes(&n.to_be_bytes())
    }

    /// Write decimal (as string with fixed scale)
    fn write_decimal(&mut self, d: &Decimal, scale: u32) -> Result<()> {
        let scaled = d.round_dp(scale);
        self.write_string(&scaled.to_string())
    }

    /// Write optional string
    fn write_option_string(&mut self, opt: &Option<String>) -> Result<()> {
        match opt {
            Some(s) => {
                self.write_bytes(&[1])?; // Present marker
                self.write_string(s)
            }
            None => self.write_bytes(&[0]), // Absent marker
        }
    }

    /// Finalize and return bytes
    pub fn finalize(self) -> Vec<u8> {
        self.buffer
    }

    /// Compute SHA3-256 hash
    pub fn hash(self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(&self.buffer);
        hasher.finalize().into()
    }
}

impl Default for CanonicalSerializer {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// CANONICAL SERIALIZATION FOR PAYMENT TYPES
// =========================================================================

impl InstructPayment {
    /// Serialize to canonical bytes (for signing)
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut ser = CanonicalSerializer::new();

        // Fixed field order (as per protobuf field numbers)
        ser.write_string(&self.payment_id.to_string()).unwrap();
        ser.write_string(&self.uetr).unwrap();
        ser.write_string(&self.idempotency_key).unwrap();
        ser.write_u64(self.nonce).unwrap();
        ser.write_i64(self.timestamp.timestamp_nanos_opt().unwrap_or(0))
            .unwrap();
        ser.write_u32(self.ttl_seconds).unwrap();
        ser.write_string(&self.network_id).unwrap();
        ser.write_string(&self.corridor_id).unwrap();

        // Payment details
        self.payment.write_canonical(&mut ser).unwrap();

        // Eligibility tokens
        self.debit_token.write_canonical(&mut ser).unwrap();
        self.credit_token.write_canonical(&mut ser).unwrap();

        ser.finalize()
    }

    /// Compute canonical hash (SHA3-256)
    pub fn canonical_hash(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(self.canonical_bytes());
        hasher.finalize().into()
    }

    /// Verify canonical hash matches
    pub fn verify_canonical_hash(&self) -> Result<()> {
        let computed = self.canonical_hash();
        if computed != self.canonical_hash {
            return Err(Error::CanonicalHashMismatch {
                expected: hex::encode(self.canonical_hash),
                actual: hex::encode(computed),
            });
        }
        Ok(())
    }
}

impl PaymentDetails {
    fn write_canonical(&self, ser: &mut CanonicalSerializer) -> Result<()> {
        ser.write_decimal(&self.amount, self.scale)?;
        ser.write_string(&self.currency)?;
        ser.write_u32(self.scale)?;
        self.debtor.write_canonical(ser)?;
        self.creditor.write_canonical(ser)?;
        ser.write_string(&self.purpose_code)?;
        ser.write_option_string(&self.remittance_info)?;
        ser.write_u32(self.settlement_method as u32)?;
        ser.write_i64(self.value_date.timestamp_nanos_opt().unwrap_or(0))?;
        Ok(())
    }
}

impl Account {
    fn write_canonical(&self, ser: &mut CanonicalSerializer) -> Result<()> {
        ser.write_string(&self.account_id)?;
        ser.write_string(&self.account_name)?;
        ser.write_string(&self.bic)?;
        ser.write_option_string(&self.lei)?;
        ser.write_string(&self.bank_name)?;
        ser.write_string(&self.country_code)?;

        // Address (optional)
        match &self.address {
            Some(addr) => {
                ser.write_bytes(&[1])?; // Present
                addr.write_canonical(ser)?;
            }
            None => ser.write_bytes(&[0])?, // Absent
        }

        Ok(())
    }
}

impl Address {
    fn write_canonical(&self, ser: &mut CanonicalSerializer) -> Result<()> {
        ser.write_string(&self.address_line_1)?;
        ser.write_option_string(&self.address_line_2)?;
        ser.write_string(&self.city)?;
        ser.write_string(&self.postal_code)?;
        ser.write_string(&self.country_code)?;
        Ok(())
    }
}

impl EligibilityToken {
    fn write_canonical(&self, ser: &mut CanonicalSerializer) -> Result<()> {
        ser.write_string(&self.token_id.to_string())?;
        ser.write_string(&self.bank_id)?;
        ser.write_string(&self.payment_id.to_string())?;
        ser.write_decimal(&self.amount, 2)?; // Assume 2 decimal places
        ser.write_string(&self.currency)?;
        ser.write_i64(self.issued_at.timestamp_nanos_opt().unwrap_or(0))?;
        ser.write_i64(self.expires_at.timestamp_nanos_opt().unwrap_or(0))?;
        ser.write_u32(self.token_type as u32)?;
        ser.write_string(&self.account_id)?;
        Ok(())
    }

    /// Canonical hash for token verification
    pub fn canonical_hash(&self) -> [u8; 32] {
        let mut ser = CanonicalSerializer::new();
        self.write_canonical(&mut ser).unwrap();
        ser.hash()
    }
}

impl NettingProposal {
    /// Canonical bytes for signing
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut ser = CanonicalSerializer::new();

        ser.write_string(&self.batch_id.to_string()).unwrap();
        ser.write_string(&self.window_id).unwrap();
        ser.write_string(&self.corridor_id).unwrap();
        ser.write_string(&self.currency).unwrap();
        ser.write_i64(self.window_start.timestamp_nanos_opt().unwrap_or(0))
            .unwrap();
        ser.write_i64(self.window_end.timestamp_nanos_opt().unwrap_or(0))
            .unwrap();
        ser.write_i64(self.proposed_at.timestamp_nanos_opt().unwrap_or(0))
            .unwrap();

        // Obligations (sorted by obligation_id for determinism)
        let mut obligations = self.obligations.clone();
        obligations.sort_by_key(|o| o.obligation_id);
        ser.write_u32(obligations.len() as u32).unwrap();
        for obl in &obligations {
            obl.write_canonical(&mut ser).unwrap();
        }

        // Net transfers (sorted by transfer_id)
        let mut transfers = self.net_transfers.clone();
        transfers.sort_by_key(|t| t.transfer_id);
        ser.write_u32(transfers.len() as u32).unwrap();
        for transfer in &transfers {
            transfer.write_canonical(&mut ser).unwrap();
        }

        ser.write_bytes(&self.merkle_root).unwrap();
        ser.write_decimal(&self.gross_amount, 2).unwrap();
        ser.write_decimal(&self.net_amount, 2).unwrap();

        ser.finalize()
    }

    /// Canonical hash
    pub fn canonical_hash(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(self.canonical_bytes());
        hasher.finalize().into()
    }
}

impl BilateralObligation {
    fn write_canonical(&self, ser: &mut CanonicalSerializer) -> Result<()> {
        ser.write_string(&self.obligation_id.to_string())?;
        ser.write_string(&self.debtor_bank)?;
        ser.write_string(&self.creditor_bank)?;
        ser.write_decimal(&self.gross_amount, 2)?;
        ser.write_string(&self.currency)?;
        ser.write_u32(self.payment_count as u32)?;

        // Payment IDs (sorted)
        let mut ids = self.payment_ids.clone();
        ids.sort();
        ser.write_u32(ids.len() as u32)?;
        for id in ids {
            ser.write_string(&id.to_string())?;
        }

        Ok(())
    }
}

impl NetTransfer {
    fn write_canonical(&self, ser: &mut CanonicalSerializer) -> Result<()> {
        ser.write_string(&self.transfer_id.to_string())?;
        ser.write_string(&self.from_bank)?;
        ser.write_string(&self.to_bank)?;
        ser.write_decimal(&self.net_amount, 2)?;
        ser.write_string(&self.currency)?;
        Ok(())
    }
}

impl SettlementProof {
    /// Canonical bytes for HSM signing
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut ser = CanonicalSerializer::new();

        ser.write_string(&self.proof_id.to_string()).unwrap();
        ser.write_string(&self.batch_id.to_string()).unwrap();
        ser.write_u64(self.checkpoint_height).unwrap();
        ser.write_bytes(&self.merkle_root).unwrap();
        ser.write_bytes(&self.app_hash).unwrap();
        ser.write_bytes(&self.prev_checkpoint_id).unwrap();
        ser.write_string(&self.network_id).unwrap();
        ser.write_u32(self.proto_version as u32).unwrap();
        ser.write_i64(self.batch_finalized_at.timestamp_nanos_opt().unwrap_or(0))
            .unwrap();
        ser.write_i64(self.proof_generated_at.timestamp_nanos_opt().unwrap_or(0))
            .unwrap();

        ser.finalize()
    }

    /// Canonical hash
    pub fn canonical_hash(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(self.canonical_bytes());
        hasher.finalize().into()
    }
}

impl Checkpoint {
    /// Canonical bytes for multi-sig
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut ser = CanonicalSerializer::new();

        ser.write_u64(self.height).unwrap();
        ser.write_bytes(&self.prev_checkpoint_id).unwrap();
        ser.write_bytes(&self.app_hash).unwrap();
        ser.write_bytes(&self.merkle_root).unwrap();
        ser.write_string(&self.network_id).unwrap();
        ser.write_u32(self.proto_version as u32).unwrap();
        ser.write_i64(self.timestamp.timestamp_nanos_opt().unwrap_or(0))
            .unwrap();

        ser.finalize()
    }

    /// Compute checkpoint ID (SHA3-256 of canonical bytes)
    pub fn compute_id(&self) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(self.canonical_bytes());
        hasher.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    #[test]
    fn test_canonical_determinism() {
        let token = EligibilityToken {
            token_id: Uuid::nil(),
            bank_id: "BANKGB2L".to_string(),
            payment_id: Uuid::nil(),
            amount: dec!(1000.50),
            currency: "USD".to_string(),
            issued_at: Utc::now(),
            expires_at: Utc::now(),
            token_type: TokenType::Debit,
            account_id: "GB29NWBK60161331926819".to_string(),
            signature: vec![],
            public_key: vec![],
        };

        // Hash twice should be identical
        let hash1 = token.canonical_hash();
        let hash2 = token.canonical_hash();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_decimal_rounding() {
        let mut ser = CanonicalSerializer::new();

        // Test fixed-scale decimal serialization
        let amount = dec!(1000.123456);
        ser.write_decimal(&amount, 2).unwrap(); // Round to 2 decimal places

        let bytes = ser.finalize();
        let s = String::from_utf8_lossy(&bytes[4..]); // Skip length prefix

        assert_eq!(s, "1000.12");
    }
}