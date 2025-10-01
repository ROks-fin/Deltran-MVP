//! Error types for settlement engine

use thiserror::Error;

/// Result type for settlement operations
pub type Result<T> = std::result::Result<T, Error>;

/// Settlement errors
#[derive(Error, Debug)]
pub enum Error {
    /// Ledger error
    #[error("Ledger error: {0}")]
    Ledger(#[from] ledger_core::Error),

    /// Netting algorithm error
    #[error("Netting error: {0}")]
    Netting(String),

    /// Window management error
    #[error("Window error: {0}")]
    Window(String),

    /// ISO 20022 generation error
    #[error("ISO 20022 error: {0}")]
    Iso20022(String),

    /// Insufficient liquidity
    #[error("Insufficient liquidity: {0}")]
    InsufficientLiquidity(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
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