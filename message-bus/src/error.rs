//! Error types for message bus

use thiserror::Error;

/// Message bus error
#[derive(Debug, Error)]
pub enum Error {
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Publish error
    #[error("Publish error: {0}")]
    Publish(String),

    /// Subscribe error
    #[error("Subscribe error: {0}")]
    Subscribe(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// NATS error
    #[error("NATS error: {0}")]
    Nats(String),
}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;
