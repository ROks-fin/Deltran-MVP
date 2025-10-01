//! Error types for consensus

use thiserror::Error;

/// Result type for consensus operations
pub type Result<T> = std::result::Result<T, Error>;

/// Consensus errors
#[derive(Error, Debug)]
pub enum Error {
    /// Ledger error
    #[error("Ledger error: {0}")]
    Ledger(#[from] ledger_core::Error),

    /// ABCI error
    #[error("ABCI error: {0}")]
    Abci(String),

    /// Invalid transaction
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    /// Consensus error
    #[error("Consensus error: {0}")]
    Consensus(String),

    /// State error
    #[error("State error: {0}")]
    State(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error
    #[error("{0}")]
    Other(String),
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