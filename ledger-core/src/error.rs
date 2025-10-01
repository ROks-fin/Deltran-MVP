//! Error types for the ledger

use thiserror::Error;

/// Result type for ledger operations
pub type Result<T> = std::result::Result<T, Error>;

/// Ledger errors
#[derive(Error, Debug)]
pub enum Error {
    /// Storage error (RocksDB)
    #[error("Storage error: {0}")]
    Storage(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    /// Invalid event
    #[error("Invalid event: {0}")]
    InvalidEvent(String),

    /// Payment not found
    #[error("Payment not found: {0}")]
    PaymentNotFound(String),

    /// Event not found
    #[error("Event not found: {0}")]
    EventNotFound(String),

    /// Block not found
    #[error("Block not found: {0}")]
    BlockNotFound(String),

    /// Invariant violation (money conservation, etc.)
    #[error("Invariant violation: {0}")]
    InvariantViolation(String),

    /// Signature verification failed
    #[error("Signature verification failed: {0}")]
    SignatureError(String),

    /// Merkle proof invalid
    #[error("Merkle proof invalid: {0}")]
    MerkleError(String),

    /// Concurrency error (actor mailbox closed, etc.)
    #[error("Concurrency error: {0}")]
    Concurrency(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<rocksdb::Error> for Error {
    fn from(err: rocksdb::Error) -> Self {
        Error::Storage(err.to_string())
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::Other(msg)
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::Other(msg.to_string())
    }
}