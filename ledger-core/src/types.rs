//! Core types for the ledger
//!
//! All types are designed for:
//! - Deterministic serialization (bincode)
//! - Memory safety (no unsafe code)
//! - Exact arithmetic (Decimal for money)

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Account identifier (IBAN, account number, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(String);

impl AccountId {
    /// Create new account ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get as string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extract country code (first 2 chars)
    pub fn country_code(&self) -> Option<&str> {
        if self.0.len() >= 2 {
            Some(&self.0[..2])
        } else {
            None
        }
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// ISO 4217 currency code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Currency {
    /// US Dollar
    USD,
    /// Euro
    EUR,
    /// British Pound
    GBP,
    /// UAE Dirham
    AED,
    /// Indian Rupee
    INR,
}

impl Currency {
    /// ISO 4217 code
    pub fn code(&self) -> &'static str {
        match self {
            Currency::USD => "USD",
            Currency::EUR => "EUR",
            Currency::GBP => "GBP",
            Currency::AED => "AED",
            Currency::INR => "INR",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "USD" => Some(Currency::USD),
            "EUR" => Some(Currency::EUR),
            "GBP" => Some(Currency::GBP),
            "AED" => Some(Currency::AED),
            "INR" => Some(Currency::INR),
            _ => None,
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Ledger event representing a state transition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEvent {
    /// Unique event ID (UUIDv7 for time-ordering)
    pub event_id: Uuid,

    /// Payment this event belongs to
    pub payment_id: Uuid,

    /// Type of event
    pub event_type: EventType,

    /// Payment amount (exact decimal)
    pub amount: Decimal,

    /// Currency
    pub currency: Currency,

    /// Debtor (sender) account
    pub debtor: AccountId,

    /// Creditor (receiver) account
    pub creditor: AccountId,

    /// Event timestamp (nanoseconds since Unix epoch)
    pub timestamp_nanos: i64,

    /// Block ID (null until finalized)
    pub block_id: Option<Uuid>,

    /// Digital signature (Ed25519)
    pub signature: Signature,

    /// Previous event ID in this payment (for chaining)
    pub previous_event_id: Option<Uuid>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl LedgerEvent {
    /// Create canonical bytes for signing
    pub fn canonical_bytes(&self) -> Vec<u8> {
        // Deterministic serialization for signature verification
        bincode::serialize(self).expect("serialization cannot fail")
    }

    /// Verify signature
    pub fn verify_signature(&self, public_key: &[u8; 32]) -> bool {
        self.signature.verify(&self.canonical_bytes(), public_key)
    }
}

/// Event type (state transition)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EventType {
    /// Payment initiated
    PaymentInitiated = 1,
    /// Validation passed
    ValidationPassed = 2,
    /// Validation failed
    ValidationFailed = 3,
    /// Sanctions check cleared
    SanctionsCleared = 4,
    /// Sanctions hit
    SanctionsHit = 5,
    /// Risk assessment approved
    RiskApproved = 6,
    /// Risk assessment rejected
    RiskRejected = 7,
    /// Queued for settlement
    QueuedForSettlement = 8,
    /// Settlement started
    SettlementStarted = 9,
    /// Settlement completed
    SettlementCompleted = 10,
    /// Payment completed (final state)
    PaymentCompleted = 11,
    /// Payment rejected
    PaymentRejected = 12,
    /// Payment failed
    PaymentFailed = 13,
}

/// Payment state (derived from events)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentState {
    /// Payment ID
    pub payment_id: Uuid,

    /// Current status
    pub status: PaymentStatus,

    /// Payment amount
    pub amount: Decimal,

    /// Currency
    pub currency: Currency,

    /// Debtor account
    pub debtor: AccountId,

    /// Creditor account
    pub creditor: AccountId,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Event IDs that created this state
    pub event_ids: Vec<Uuid>,

    /// Current block ID (if finalized)
    pub current_block_id: Option<Uuid>,
}

impl PaymentState {
    /// Apply event to derive new state
    pub fn apply_event(&mut self, event: &LedgerEvent) -> crate::Result<()> {
        // Verify event belongs to this payment
        if event.payment_id != self.payment_id {
            return Err(crate::Error::InvalidEvent(
                "Event payment_id mismatch".to_string(),
            ));
        }

        // Update status based on event type
        self.status = match event.event_type {
            EventType::PaymentInitiated => PaymentStatus::Initiated,
            EventType::ValidationPassed => PaymentStatus::Validated,
            EventType::ValidationFailed => PaymentStatus::Rejected,
            EventType::SanctionsCleared => PaymentStatus::Screened,
            EventType::SanctionsHit => PaymentStatus::Rejected,
            EventType::RiskApproved => PaymentStatus::Approved,
            EventType::RiskRejected => PaymentStatus::Rejected,
            EventType::QueuedForSettlement => PaymentStatus::Queued,
            EventType::SettlementStarted => PaymentStatus::Settling,
            EventType::SettlementCompleted => PaymentStatus::Settled,
            EventType::PaymentCompleted => PaymentStatus::Completed,
            EventType::PaymentRejected => PaymentStatus::Rejected,
            EventType::PaymentFailed => PaymentStatus::Failed,
        };

        // Update timestamp
        self.updated_at = DateTime::from_timestamp_nanos(event.timestamp_nanos);

        // Add event ID to history
        self.event_ids.push(event.event_id);

        // Update block ID if present
        if let Some(block_id) = event.block_id {
            self.current_block_id = Some(block_id);
        }

        Ok(())
    }

    /// Check if payment is in terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            PaymentStatus::Completed | PaymentStatus::Rejected | PaymentStatus::Failed
        )
    }
}

/// Payment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PaymentStatus {
    /// Initial state
    Initiated = 1,
    /// Validation passed
    Validated = 2,
    /// Sanctions/AML screening complete
    Screened = 3,
    /// Risk assessment approved
    Approved = 4,
    /// Queued for settlement
    Queued = 5,
    /// Settlement in progress
    Settling = 6,
    /// Settlement complete
    Settled = 7,
    /// Payment completed (terminal)
    Completed = 8,
    /// Payment rejected (terminal)
    Rejected = 9,
    /// Payment failed (terminal)
    Failed = 10,
}

/// Finalized block with Merkle root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    /// Unique block ID
    pub block_id: Uuid,

    /// Block height (sequential)
    pub block_height: u64,

    /// Merkle root of all events in block
    pub merkle_root: [u8; 32],

    /// Hash of previous block
    pub previous_block_hash: [u8; 32],

    /// Hash of this block's contents
    pub block_hash: [u8; 32],

    /// Event IDs in this block
    pub event_ids: Vec<Uuid>,

    /// Number of events
    pub event_count: u32,

    /// Block creation timestamp
    pub created_at: DateTime<Utc>,

    /// Proposer signature (from consensus)
    pub proposer_signature: Vec<u8>,

    /// Validator signatures
    pub validator_signatures: Vec<ValidatorSignature>,
}

impl Block {
    /// Compute block hash
    pub fn compute_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(&self.block_height.to_be_bytes());
        hasher.update(&self.merkle_root);
        hasher.update(&self.previous_block_hash);
        hasher.update(&self.event_count.to_be_bytes());
        hasher.update(self.created_at.timestamp_nanos_opt().unwrap_or(0).to_be_bytes());

        hasher.finalize().into()
    }
}

/// Validator signature for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSignature {
    /// Validator ID
    pub validator_id: String,
    /// Public key
    pub public_key: Vec<u8>,
    /// Signature
    pub signature: Vec<u8>,
}

/// Digital signature (Ed25519)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    /// Signature bytes (64 bytes)
    #[serde(with = "serde_bytes")]
    bytes: [u8; 64],
}

impl Signature {
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self { bytes }
    }

    /// Get bytes
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.bytes
    }

    /// Verify signature
    pub fn verify(&self, message: &[u8], public_key: &[u8; 32]) -> bool {
        use ed25519_dalek::{Signature as DalekSignature, Verifier, VerifyingKey};

        let signature = DalekSignature::from_bytes(&self.bytes);

        let verifying_key = match VerifyingKey::from_bytes(public_key) {
            Ok(key) => key,
            Err(_) => return false,
        };

        verifying_key.verify(message, &signature).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_id_country_code() {
        let account = AccountId::new("US1234567890");
        assert_eq!(account.country_code(), Some("US"));
    }

    #[test]
    fn test_currency_from_str() {
        assert_eq!(Currency::from_str("USD"), Some(Currency::USD));
        assert_eq!(Currency::from_str("EUR"), Some(Currency::EUR));
        assert_eq!(Currency::from_str("INVALID"), None);
    }

    #[test]
    fn test_payment_status_terminal() {
        let mut state = PaymentState {
            payment_id: Uuid::new_v4(),
            status: PaymentStatus::Initiated,
            amount: Decimal::from(1000),
            currency: Currency::USD,
            debtor: AccountId::new("US123"),
            creditor: AccountId::new("AE456"),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            event_ids: vec![],
            current_block_id: None,
        };

        assert!(!state.is_terminal());

        state.status = PaymentStatus::Completed;
        assert!(state.is_terminal());

        state.status = PaymentStatus::Rejected;
        assert!(state.is_terminal());
    }
}