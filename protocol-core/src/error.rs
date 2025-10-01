//! Error types for protocol operations

use thiserror::Error;

/// Protocol result type
pub type Result<T> = std::result::Result<T, Error>;

/// Protocol errors
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid protocol state transition
    #[error("Invalid state transition from {from:?} to {to:?}: {reason}")]
    InvalidStateTransition {
        /// Current state
        from: String,
        /// Target state
        to: String,
        /// Reason for rejection
        reason: String,
    },

    /// Eligibility token validation failed
    #[error("Eligibility token validation failed: {0}")]
    EligibilityTokenInvalid(String),

    /// Signature verification failed
    #[error("Signature verification failed: {0}")]
    SignatureInvalid(String),

    /// HSM operation failed
    #[error("HSM operation failed: {0}")]
    HsmError(String),

    /// Replay attack detected
    #[error("Replay attack detected: {0}")]
    ReplayAttack(String),

    /// TTL expired
    #[error("TTL expired: {0}")]
    TtlExpired(String),

    /// Nonce validation failed
    #[error("Invalid nonce: expected > {expected}, got {actual}")]
    InvalidNonce {
        /// Expected nonce (monotonic)
        expected: u64,
        /// Actual nonce
        actual: u64,
    },

    /// Canonical hash mismatch
    #[error("Canonical hash mismatch: expected {expected}, got {actual}")]
    CanonicalHashMismatch {
        /// Expected hash
        expected: String,
        /// Actual hash
        actual: String,
    },

    /// Merkle proof verification failed
    #[error("Merkle proof verification failed: {0}")]
    MerkleProofInvalid(String),

    /// BFT quorum not met
    #[error("BFT quorum not met: {actual}/{required} signatures")]
    QuorumNotMet {
        /// Actual signatures
        actual: usize,
        /// Required signatures
        required: usize,
    },

    /// Netting threshold not met
    #[error("Netting threshold not met: {reason}")]
    NettingThresholdNotMet {
        /// Reason for rejection
        reason: String,
    },

    /// 2PC timeout
    #[error("2PC timeout after {seconds}s")]
    TwoPCTimeout {
        /// Timeout duration
        seconds: u32,
    },

    /// Bank confirmation missing
    #[error("Bank confirmation missing: {bank_id}")]
    BankConfirmationMissing {
        /// Bank ID (BIC/LEI)
        bank_id: String,
    },

    /// Partial settlement decomposition failed
    #[error("Partial settlement decomposition failed: {0}")]
    PartialSettlementFailed(String),

    /// Checkpoint generation failed
    #[error("Checkpoint generation failed: {0}")]
    CheckpointFailed(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Cryptographic error
    #[error("Cryptographic error: {0}")]
    Crypto(String),

    /// Generic protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),
}

impl From<ed25519_dalek::SignatureError> for Error {
    fn from(e: ed25519_dalek::SignatureError) -> Self {
        Error::SignatureInvalid(e.to_string())
    }
}