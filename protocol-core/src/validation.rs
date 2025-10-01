//! Validation logic for protocol messages
//!
//! - Eligibility token validation
//! - Replay protection (nonce + TTL)
//! - BIC/IBAN validation
//! - Signature verification
//! - Threshold checks (netting)

use crate::{types::*, Error, Result, DEFAULT_TTL_SECONDS};
use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use regex::Regex;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

/// Validator for protocol messages
pub struct ProtocolValidator {
    /// Nonce tracker (per sender_bank_id)
    nonce_tracker: HashMap<String, u64>,
    /// BIC regex pattern
    bic_regex: Regex,
    /// IBAN regex pattern (basic)
    iban_regex: Regex,
}

impl ProtocolValidator {
    /// Create new validator
    pub fn new() -> Self {
        Self {
            nonce_tracker: HashMap::new(),
            bic_regex: Regex::new(r"^[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?$").unwrap(),
            iban_regex: Regex::new(r"^[A-Z]{2}[0-9]{2}[A-Z0-9]+$").unwrap(),
        }
    }

    // =========================================================================
    // INSTRUCT_PAYMENT VALIDATION
    // =========================================================================

    /// Validate InstructPayment message
    pub fn validate_instruct_payment(&mut self, msg: &InstructPayment) -> Result<()> {
        // 1. Verify canonical hash
        msg.verify_canonical_hash()?;

        // 2. Verify sender signature
        self.verify_signature(
            &msg.canonical_hash,
            &msg.sender_signature,
            &msg.sender_public_key,
        )?;

        // 3. Check TTL (5 minute window)
        self.check_ttl(msg.timestamp, msg.ttl_seconds)?;

        // 4. Check nonce (anti-replay)
        self.check_nonce(&msg.sender_bank_id, msg.nonce)?;

        // 5. Validate payment details
        self.validate_payment_details(&msg.payment)?;

        // 6. Validate eligibility tokens
        self.validate_eligibility_token(&msg.debit_token, &msg.payment, TokenType::Debit)?;
        self.validate_eligibility_token(&msg.credit_token, &msg.payment, TokenType::Credit)?;

        // 7. BIC validation
        self.validate_bic(&msg.payment.debtor.bic)?;
        self.validate_bic(&msg.payment.creditor.bic)?;

        // 8. IBAN validation (if applicable)
        if msg.payment.debtor.account_id.starts_with(|c: char| c.is_alphabetic()) {
            self.validate_iban(&msg.payment.debtor.account_id)?;
        }
        if msg.payment.creditor.account_id.starts_with(|c: char| c.is_alphabetic()) {
            self.validate_iban(&msg.payment.creditor.account_id)?;
        }

        Ok(())
    }

    /// Validate payment details
    fn validate_payment_details(&self, payment: &PaymentDetails) -> Result<()> {
        // Amount must be positive
        if payment.amount <= Decimal::ZERO {
            return Err(Error::Protocol("Payment amount must be positive".into()));
        }

        // Currency must be ISO 4217
        if payment.currency.len() != 3 {
            return Err(Error::Protocol(format!(
                "Invalid currency code: {}",
                payment.currency
            )));
        }

        // Scale validation
        if payment.scale > 8 {
            return Err(Error::Protocol(format!(
                "Invalid scale: {} (max 8)",
                payment.scale
            )));
        }

        // Debtor != Creditor
        if payment.debtor.bic == payment.creditor.bic
            && payment.debtor.account_id == payment.creditor.account_id
        {
            return Err(Error::Protocol(
                "Debtor and creditor cannot be the same".into(),
            ));
        }

        Ok(())
    }

    // =========================================================================
    // ELIGIBILITY TOKEN VALIDATION
    // =========================================================================

    /// Validate eligibility token
    fn validate_eligibility_token(
        &self,
        token: &EligibilityToken,
        payment: &PaymentDetails,
        expected_type: TokenType,
    ) -> Result<()> {
        // Type check
        if token.token_type != expected_type {
            return Err(Error::EligibilityTokenInvalid(format!(
                "Expected {:?} token, got {:?}",
                expected_type, token.token_type
            )));
        }

        // Expiry check (15 minute window)
        let now = Utc::now();
        if now > token.expires_at {
            return Err(Error::EligibilityTokenInvalid(format!(
                "Token expired at {}",
                token.expires_at
            )));
        }

        // Amount match
        if token.amount != payment.amount {
            return Err(Error::EligibilityTokenInvalid(format!(
                "Amount mismatch: token={}, payment={}",
                token.amount, payment.amount
            )));
        }

        // Currency match
        if token.currency != payment.currency {
            return Err(Error::EligibilityTokenInvalid(format!(
                "Currency mismatch: token={}, payment={}",
                token.currency, payment.currency
            )));
        }

        // Verify token signature
        let token_hash = token.canonical_hash();
        self.verify_signature(&token_hash, &token.signature, &token.public_key)?;

        Ok(())
    }

    // =========================================================================
    // REPLAY PROTECTION
    // =========================================================================

    /// Check TTL (time-to-live)
    fn check_ttl(&self, timestamp: chrono::DateTime<Utc>, ttl_seconds: u32) -> Result<()> {
        let now = Utc::now();
        let age = now.signed_duration_since(timestamp);

        let ttl = if ttl_seconds == 0 {
            DEFAULT_TTL_SECONDS
        } else {
            ttl_seconds
        };

        if age.num_seconds() > ttl as i64 {
            return Err(Error::TtlExpired(format!(
                "Message age {}s exceeds TTL {}s",
                age.num_seconds(),
                ttl
            )));
        }

        // Also reject future timestamps (clock skew tolerance: 5 seconds)
        if age.num_seconds() < -5 {
            return Err(Error::ReplayAttack(format!(
                "Timestamp is {}s in the future",
                -age.num_seconds()
            )));
        }

        Ok(())
    }

    /// Check nonce (monotonically increasing per sender)
    fn check_nonce(&mut self, sender_id: &str, nonce: u64) -> Result<()> {
        let last_nonce = self.nonce_tracker.get(sender_id).copied().unwrap_or(0);

        if nonce <= last_nonce {
            return Err(Error::InvalidNonce {
                expected: last_nonce + 1,
                actual: nonce,
            });
        }

        // Update tracker
        self.nonce_tracker.insert(sender_id.to_string(), nonce);

        Ok(())
    }

    // =========================================================================
    // SIGNATURE VERIFICATION
    // =========================================================================

    /// Verify Ed25519 signature
    fn verify_signature(&self, message: &[u8], signature: &[u8], public_key: &[u8]) -> Result<()> {
        // Parse public key
        let pk_bytes: [u8; 32] = public_key
            .try_into()
            .map_err(|_| Error::SignatureInvalid("Invalid public key length".into()))?;
        let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
            .map_err(|e| Error::SignatureInvalid(e.to_string()))?;

        // Parse signature
        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| Error::SignatureInvalid("Invalid signature length".into()))?;
        let signature = Signature::from_bytes(&sig_bytes);

        // Verify
        verifying_key
            .verify(message, &signature)
            .map_err(|e| Error::SignatureInvalid(e.to_string()))?;

        Ok(())
    }

    // =========================================================================
    // BIC / IBAN VALIDATION
    // =========================================================================

    /// Validate BIC (SWIFT code)
    fn validate_bic(&self, bic: &str) -> Result<()> {
        if !self.bic_regex.is_match(bic) {
            return Err(Error::Protocol(format!("Invalid BIC: {}", bic)));
        }
        Ok(())
    }

    /// Validate IBAN (basic check, not full mod-97)
    fn validate_iban(&self, iban: &str) -> Result<()> {
        if !self.iban_regex.is_match(iban) {
            return Err(Error::Protocol(format!("Invalid IBAN format: {}", iban)));
        }

        // TODO: Full mod-97 checksum validation
        // For MVP, pattern matching is sufficient

        Ok(())
    }

    // =========================================================================
    // NETTING VALIDATION
    // =========================================================================

    /// Validate netting proposal meets thresholds
    pub fn validate_netting_proposal(&self, proposal: &NettingProposal) -> Result<()> {
        // Minimum volume check ($100k)
        let min_volume =
            Decimal::from_str(crate::MIN_NETTING_VOLUME).expect("Invalid MIN_NETTING_VOLUME");
        if proposal.gross_amount < min_volume {
            return Err(Error::NettingThresholdNotMet {
                reason: format!(
                    "Gross amount {} below minimum {}",
                    proposal.gross_amount, min_volume
                ),
            });
        }

        // Minimum efficiency check (15%)
        if proposal.netting_efficiency < crate::MIN_NETTING_EFFICIENCY {
            return Err(Error::NettingThresholdNotMet {
                reason: format!(
                    "Netting efficiency {:.2}% below minimum {:.2}%",
                    proposal.netting_efficiency * 100.0,
                    crate::MIN_NETTING_EFFICIENCY * 100.0
                ),
            });
        }

        // Minimum participants (2)
        if proposal.participant_count < crate::MIN_NETTING_PARTICIPANTS {
            return Err(Error::NettingThresholdNotMet {
                reason: format!(
                    "Participant count {} below minimum {}",
                    proposal.participant_count,
                    crate::MIN_NETTING_PARTICIPANTS
                ),
            });
        }

        Ok(())
    }

    /// Validate bank confirmation signature
    pub fn validate_bank_confirmation(&self, confirmation: &BankConfirmation) -> Result<()> {
        // Serialize confirmation data (batch_id + bank_id + status)
        let mut data = Vec::new();
        data.extend_from_slice(confirmation.batch_id.as_bytes());
        data.extend_from_slice(confirmation.bank_id.as_bytes());
        data.push(confirmation.status as u8);

        self.verify_signature(&data, &confirmation.signature, &confirmation.public_key)?;

        Ok(())
    }

    // =========================================================================
    // SETTLEMENT PROOF VALIDATION
    // =========================================================================

    /// Validate BFT quorum (5 of 7)
    pub fn validate_bft_quorum(&self, signatures: &[ValidatorSignature]) -> Result<()> {
        let required = (crate::BFT_QUORUM_NUMERATOR as usize * 7)
            / (crate::BFT_QUORUM_DENOMINATOR as usize);

        if signatures.len() < required {
            return Err(Error::QuorumNotMet {
                actual: signatures.len(),
                required,
            });
        }

        Ok(())
    }

    /// Reset nonce tracker (for testing)
    #[cfg(test)]
    pub fn reset_nonces(&mut self) {
        self.nonce_tracker.clear();
    }
}

impl Default for ProtocolValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_bic_validation() {
        let validator = ProtocolValidator::new();

        assert!(validator.validate_bic("BANKGB2L").is_ok());
        assert!(validator.validate_bic("CHASUS33XXX").is_ok());
        assert!(validator.validate_bic("invalid").is_err());
        assert!(validator.validate_bic("BANK").is_err());
    }

    #[test]
    fn test_iban_validation() {
        let validator = ProtocolValidator::new();

        assert!(validator.validate_iban("GB29NWBK60161331926819").is_ok());
        assert!(validator.validate_iban("DE89370400440532013000").is_ok());
        assert!(validator.validate_iban("invalid").is_err());
    }

    #[test]
    fn test_nonce_anti_replay() {
        let mut validator = ProtocolValidator::new();
        let bank_id = "BANKGB2L";

        assert!(validator.check_nonce(bank_id, 1).is_ok());
        assert!(validator.check_nonce(bank_id, 2).is_ok());
        assert!(validator.check_nonce(bank_id, 3).is_ok());

        // Replay attack
        assert!(validator.check_nonce(bank_id, 2).is_err());
        assert!(validator.check_nonce(bank_id, 3).is_err());

        // Valid continuation
        assert!(validator.check_nonce(bank_id, 4).is_ok());
    }

    #[test]
    fn test_ttl_check() {
        let validator = ProtocolValidator::new();

        // Current timestamp (valid)
        let now = Utc::now();
        assert!(validator.check_ttl(now, 300).is_ok());

        // Old timestamp (expired)
        let old = now - chrono::Duration::seconds(400);
        assert!(validator.check_ttl(old, 300).is_err());

        // Future timestamp (invalid)
        let future = now + chrono::Duration::seconds(10);
        assert!(validator.check_ttl(future, 300).is_err());
    }

    #[test]
    fn test_netting_threshold_validation() {
        let validator = ProtocolValidator::new();

        let valid_proposal = NettingProposal {
            batch_id: uuid::Uuid::new_v4(),
            window_id: "test".into(),
            corridor_id: "UAE-IND".into(),
            currency: "USD".into(),
            window_start: Utc::now(),
            window_end: Utc::now(),
            proposed_at: Utc::now(),
            obligations: vec![],
            net_transfers: vec![],
            merkle_root: [0u8; 32],
            gross_amount: dec!(150000.00), // Above $100k
            net_amount: dec!(50000.00),
            netting_efficiency: 0.67, // Above 15%
            requires_confirmations: vec![],
            meets_min_volume: true,
            meets_min_efficiency: true,
            participant_count: 3, // Above 2
        };

        assert!(validator.validate_netting_proposal(&valid_proposal).is_ok());

        // Test failures
        let mut low_volume = valid_proposal.clone();
        low_volume.gross_amount = dec!(50000.00); // Below $100k
        assert!(validator.validate_netting_proposal(&low_volume).is_err());

        let mut low_efficiency = valid_proposal.clone();
        low_efficiency.netting_efficiency = 0.10; // Below 15%
        assert!(validator
            .validate_netting_proposal(&low_efficiency)
            .is_err());
    }
}