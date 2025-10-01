//! Error types for message bus

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, Error>;

/// Message bus errors
#[derive(Debug, Error)]
pub enum Error {
    /// NATS connection error
    #[error("NATS connection error: {0}")]
    NatsConnection(String),

    /// NATS publish error
    #[error("NATS publish error: {0}")]
    NatsPublish(String),

    /// NATS subscribe error
    #[error("NATS subscribe error: {0}")]
    NatsSubscribe(String),

    /// JetStream error
    #[error("JetStream error: {0}")]
    JetStream(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Invalid partition key
    #[error("Invalid partition key: {0}")]
    InvalidPartitionKey(String),

    /// Message timeout
    #[error("Message timeout after {0}ms")]
    Timeout(u64),

    /// Consumer group error
    #[error("Consumer group error: {0}")]
    ConsumerGroup(String),

    /// Generic error
    #[error("{0}")]
    Generic(String),
}

impl From<async_nats::Error> for Error {
    fn from(err: async_nats::Error) -> Self {
        Error::NatsConnection(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}
