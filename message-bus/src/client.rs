//! Message bus client

use crate::{Error, Result, Message, Subject};
use serde::{Deserialize, Serialize};

/// Message bus client (stub for now - will connect to NATS in production)
pub struct MessageBusClient {
    // Will hold NATS connection
    _connection_url: String,
}

impl MessageBusClient {
    /// Create new client
    pub async fn connect(url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            _connection_url: url.into(),
        })
    }

    /// Publish message
    pub async fn publish<T: Serialize>(
        &self,
        subject: impl Into<Subject>,
        message: &Message<T>,
    ) -> Result<()> {
        let _subject = subject.into();
        let _payload = serde_json::to_vec(message)?;

        // TODO: Actual NATS publish
        tracing::debug!("Message published (stub)");

        Ok(())
    }

    /// Subscribe to subject
    pub async fn subscribe(
        &self,
        subject: impl Into<Subject>,
    ) -> Result<()> {
        let _subject = subject.into();

        // TODO: Actual NATS subscribe
        tracing::debug!("Subscribed (stub)");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = MessageBusClient::connect("nats://localhost:4222").await;
        assert!(client.is_ok());
    }
}
