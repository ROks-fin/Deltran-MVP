//! Core types for message bus

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message subject/topic
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subject(String);

impl Subject {
    /// Create new subject
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Subject {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Subject {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Generic message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message<T> {
    /// Message ID
    pub message_id: Uuid,

    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Payload
    pub payload: T,
}

impl<T> Message<T> {
    /// Create new message
    pub fn new(payload: T) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            payload,
        }
    }
}
