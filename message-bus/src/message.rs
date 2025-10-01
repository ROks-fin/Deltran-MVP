//! Message envelope for pub/sub

use crate::types::{MessageType, PartitionKey};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID (UUIDv7 for ordering)
    pub id: Uuid,

    /// Message type
    pub message_type: MessageType,

    /// Partition key for routing
    pub partition_key: PartitionKey,

    /// Payload (JSON-serialized)
    pub payload: serde_json::Value,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Correlation ID (for tracing)
    pub correlation_id: Option<String>,

    /// Reply subject (for request-reply pattern)
    pub reply_to: Option<String>,

    /// Headers (metadata)
    pub headers: std::collections::HashMap<String, String>,
}

impl Message {
    /// Create new message
    pub fn new(
        message_type: MessageType,
        partition_key: PartitionKey,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            message_type,
            partition_key,
            payload,
            timestamp: Utc::now(),
            correlation_id: None,
            reply_to: None,
            headers: std::collections::HashMap::new(),
        }
    }

    /// Set correlation ID
    pub fn with_correlation_id(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Set reply-to subject
    pub fn with_reply_to(mut self, reply_to: String) -> Self {
        self.reply_to = Some(reply_to);
        self
    }

    /// Add header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| crate::Error::Serialization(e.to_string()))
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> crate::Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| crate::Error::Deserialization(e.to_string()))
    }

    /// Get NATS subject for this message
    pub fn subject(&self) -> String {
        format!(
            "{}.{}",
            self.message_type.subject_prefix(),
            self.partition_key.to_subject_segment()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_message_creation() {
        let msg = Message::new(
            MessageType::PaymentInstruction,
            PartitionKey::Corridor("USD-EUR".to_string()),
            json!({"amount": 1000}),
        );

        assert_eq!(msg.message_type, MessageType::PaymentInstruction);
        assert_eq!(msg.payload["amount"], 1000);
    }

    #[test]
    fn test_message_subject() {
        let msg = Message::new(
            MessageType::PaymentInstruction,
            PartitionKey::Corridor("USD-EUR".to_string()),
            json!({}),
        );

        assert_eq!(msg.subject(), "deltran.payment.instruction.corridor.USD-EUR");
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::new(
            MessageType::PaymentInstruction,
            PartitionKey::Bank("BANKGB2L".to_string()),
            json!({"test": "data"}),
        );

        let bytes = msg.to_bytes().unwrap();
        let deserialized = Message::from_bytes(&bytes).unwrap();

        assert_eq!(msg.id, deserialized.id);
        assert_eq!(msg.message_type, deserialized.message_type);
        assert_eq!(msg.payload, deserialized.payload);
    }
}
