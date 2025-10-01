//! Error types for adapters

use thiserror::Error;

/// Result type for adapter operations
pub type Result<T> = std::result::Result<T, Error>;

/// Adapter errors
#[derive(Error, Debug)]
pub enum Error {
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Circuit breaker open
    #[error("Circuit breaker open for corridor {corridor_id}: {reason}")]
    CircuitBreakerOpen {
        /// Corridor ID
        corridor_id: String,
        /// Reason
        reason: String,
    },

    /// Kill switch activated
    #[error("Kill switch activated for corridor {corridor_id}: {reason}")]
    KillSwitchActive {
        /// Corridor ID
        corridor_id: String,
        /// Reason
        reason: String,
    },

    /// Timeout
    #[error("Timeout after {seconds}s: {operation}")]
    Timeout {
        /// Timeout duration
        seconds: u64,
        /// Operation
        operation: String,
    },

    /// ISO 20022 serialization error
    #[error("ISO 20022 serialization error: {0}")]
    Iso20022Serialization(String),

    /// ISO 20022 validation error
    #[error("ISO 20022 validation error: {0}")]
    Iso20022Validation(String),

    /// Bank API error
    #[error("Bank API error {status_code}: {message}")]
    BankApi {
        /// HTTP status code
        status_code: u16,
        /// Error message
        message: String,
    },

    /// DLQ full
    #[error("DLQ full: {current}/{max} messages")]
    DlqFull {
        /// Current size
        current: usize,
        /// Max size
        max: usize,
    },

    /// Retry exhausted
    #[error("Retry exhausted after {attempts} attempts: {last_error}")]
    RetryExhausted {
        /// Attempts
        attempts: u32,
        /// Last error
        last_error: String,
    },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Unsupported adapter
    #[error("Unsupported adapter type: {0}")]
    UnsupportedAdapter(String),

    /// HTTP client error
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// XML error
    #[error("XML error: {0}")]
    Xml(String),

    /// Generic error
    #[error("Adapter error: {0}")]
    Generic(String),
}