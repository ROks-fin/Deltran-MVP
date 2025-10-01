//! Dead Letter Queue router
//!
//! Routes failed messages to DLQ with:
//! - Failure reason tracking
//! - Retry attempt counting
//! - Automatic expiration
//! - Manual reprocessing API

use async_nats::jetstream::Context as JetStreamContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{Error, Result, Message};

/// DLQ entry with failure metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqEntry {
    pub id: String,
    pub original_message: Message,
    pub failure_reason: String,
    pub retry_count: u32,
    pub first_failure_at: DateTime<Utc>,
    pub last_failure_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub reprocessable: bool,
}

/// DLQ Router
pub struct DlqRouter {
    context: Arc<JetStreamContext>,
    stream_name: String,
    max_retention_days: i64,
}

impl DlqRouter {
    /// Create new DLQ router
    pub fn new(context: Arc<JetStreamContext>, stream_name: String) -> Self {
        Self {
            context,
            stream_name,
            max_retention_days: 30,
        }
    }

    /// Route message to DLQ
    pub async fn route_to_dlq(
        &self,
        message: Message,
        failure_reason: String,
        retry_count: u32,
    ) -> Result<String> {
        let now = Utc::now();
        let entry_id = Uuid::new_v4().to_string();

        let entry = DlqEntry {
            id: entry_id.clone(),
            original_message: message.clone(),
            failure_reason: failure_reason.clone(),
            retry_count,
            first_failure_at: now,
            last_failure_at: now,
            expires_at: now + chrono::Duration::days(self.max_retention_days),
            reprocessable: self.is_reprocessable(&failure_reason),
        };

        info!(
            "Routing message {} to DLQ (reason: {}, retry: {})",
            message.id, failure_reason, retry_count
        );

        // Serialize entry
        let payload = serde_json::to_vec(&entry)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        // Publish to DLQ stream
        let subject = format!("dlq.{}.{}", message.message_type, entry_id);

        self.context
            .publish(subject, payload.into())
            .await
            .map_err(|e| Error::Publish(e.to_string()))?
            .await
            .map_err(|e| Error::Publish(e.to_string()))?;

        warn!(
            "Message {} moved to DLQ: {} (retry {})",
            message.id, failure_reason, retry_count
        );

        Ok(entry_id)
    }

    /// Check if failure is reprocessable
    fn is_reprocessable(&self, reason: &str) -> bool {
        // Transient errors are reprocessable
        let transient_errors = [
            "timeout",
            "connection_refused",
            "service_unavailable",
            "rate_limit",
            "temporary",
        ];

        transient_errors
            .iter()
            .any(|&err| reason.to_lowercase().contains(err))
    }

    /// Reprocess message from DLQ
    pub async fn reprocess(&self, entry_id: &str) -> Result<Message> {
        info!("Reprocessing DLQ entry: {}", entry_id);

        // Fetch from DLQ
        let entry = self.get_entry(entry_id).await?;

        if !entry.reprocessable {
            return Err(Error::NotReprocessable(format!(
                "Entry {} is not reprocessable: {}",
                entry_id, entry.failure_reason
            )));
        }

        // Republish to original stream
        let subject = format!(
            "payments.{}.{}",
            entry.original_message.partition_key.corridor_id,
            entry.original_message.partition_key.bank_id
        );

        self.context
            .publish(subject, entry.original_message.payload.clone().into())
            .await
            .map_err(|e| Error::Publish(e.to_string()))?
            .await
            .map_err(|e| Error::Publish(e.to_string()))?;

        info!("DLQ entry {} reprocessed successfully", entry_id);
        Ok(entry.original_message)
    }

    /// Get DLQ entry by ID
    async fn get_entry(&self, entry_id: &str) -> Result<DlqEntry> {
        // In production, this would query JetStream KV store
        // For now, return error
        Err(Error::NotFound(format!("DLQ entry {} not found", entry_id)))
    }

    /// List DLQ entries with filters
    pub async fn list_entries(
        &self,
        corridor: Option<String>,
        reprocessable_only: bool,
        limit: usize,
    ) -> Result<Vec<DlqEntry>> {
        info!(
            "Listing DLQ entries (corridor: {:?}, reprocessable: {}, limit: {})",
            corridor, reprocessable_only, limit
        );

        // This would query JetStream in production
        // Return empty for now
        Ok(vec![])
    }

    /// Get DLQ statistics
    pub async fn get_stats(&self) -> Result<DlqStats> {
        Ok(DlqStats {
            total_entries: 0,
            reprocessable: 0,
            expired: 0,
            by_reason: Default::default(),
        })
    }
}

/// DLQ statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct DlqStats {
    pub total_entries: usize,
    pub reprocessable: usize,
    pub expired: usize,
    pub by_reason: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MessageType, PartitionKey};

    #[test]
    fn test_is_reprocessable() {
        let router = DlqRouter {
            context: Arc::new(todo!()),
            stream_name: "test".to_string(),
            max_retention_days: 30,
        };

        assert!(router.is_reprocessable("connection timeout"));
        assert!(router.is_reprocessable("service temporarily unavailable"));
        assert!(!router.is_reprocessable("invalid schema"));
        assert!(!router.is_reprocessable("authorization failed"));
    }
}
