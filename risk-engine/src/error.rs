//! Error types for risk engine

use thiserror::Error;

/// Risk engine error
#[derive(Debug, Error)]
pub enum Error {
    /// Limit exceeded
    #[error("Limit exceeded: {0}")]
    LimitExceeded(String),

    /// Velocity limit exceeded
    #[error("Velocity limit exceeded: {0}")]
    VelocityLimitExceeded(String),

    /// Corridor limit exceeded
    #[error("Corridor limit exceeded: {0}")]
    CorridorLimitExceeded(String),

    /// High risk detected
    #[error("High risk detected: {0}")]
    HighRisk(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Calculation error
    #[error("Calculation error: {0}")]
    Calculation(String),

    /// ML model error
    #[error("ML model error: {0}")]
    ModelError(String),
}

/// Result type
pub type Result<T> = std::result::Result<T, Error>;
