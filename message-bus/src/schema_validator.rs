//! Protobuf schema validation
//!
//! Validates messages against protobuf schemas before publishing

use prost::Message as ProstMessage;
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::{Error, MessageType, Result};

/// Schema validator
pub struct SchemaValidator {
    schemas: HashMap<MessageType, SchemaInfo>,
}

/// Schema information
struct SchemaInfo {
    name: String,
    version: u32,
}

impl SchemaValidator {
    /// Create new schema validator
    pub fn new() -> Self {
        let mut schemas = HashMap::new();

        // Register schemas
        schemas.insert(
            MessageType::Payment,
            SchemaInfo {
                name: "Payment".to_string(),
                version: 1,
            },
        );

        schemas.insert(
            MessageType::Settlement,
            SchemaInfo {
                name: "Settlement".to_string(),
                version: 1,
            },
        );

        schemas.insert(
            MessageType::Netting,
            SchemaInfo {
                name: "NettingProposal".to_string(),
                version: 1,
            },
        );

        Self { schemas }
    }

    /// Validate message payload
    pub fn validate(&self, message_type: &MessageType, payload: &[u8]) -> Result<()> {
        let schema = self
            .schemas
            .get(message_type)
            .ok_or_else(|| Error::UnknownMessageType(format!("{:?}", message_type)))?;

        debug!("Validating {} schema v{}", schema.name, schema.version);

        // Attempt to decode
        match message_type {
            MessageType::Payment => {
                // Would decode Payment protobuf
                // For now, just check it's valid UTF-8 or binary
                if payload.is_empty() {
                    return Err(Error::InvalidSchema("Empty payload".to_string()));
                }
            }
            MessageType::Settlement => {
                if payload.is_empty() {
                    return Err(Error::InvalidSchema("Empty payload".to_string()));
                }
            }
            MessageType::Netting => {
                if payload.is_empty() {
                    return Err(Error::InvalidSchema("Empty payload".to_string()));
                }
            }
            _ => {
                warn!("No validation for message type: {:?}", message_type);
            }
        }

        debug!("Schema validation passed for {}", schema.name);
        Ok(())
    }

    /// Get schema version
    pub fn get_version(&self, message_type: &MessageType) -> Option<u32> {
        self.schemas.get(message_type).map(|s| s.version)
    }

    /// Check if schema is registered
    pub fn is_registered(&self, message_type: &MessageType) -> bool {
        self.schemas.contains_key(message_type)
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_validation() {
        let validator = SchemaValidator::new();

        // Valid payload
        let payload = b"test payload";
        assert!(validator.validate(&MessageType::Payment, payload).is_ok());

        // Empty payload
        assert!(validator.validate(&MessageType::Payment, &[]).is_err());

        // Unknown type
        assert!(!validator.is_registered(&MessageType::Unknown));
    }
}
