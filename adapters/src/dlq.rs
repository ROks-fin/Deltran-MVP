//! Dead Letter Queue (DLQ) with retry logic

use crate::{types::TransferRequest, Error, Result};
use async_channel::{bounded, Receiver, Sender};
use backoff::{exponential::ExponentialBackoff, SystemClock};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// DLQ message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqMessage {
    /// Original request
    pub request: TransferRequest,
    /// Failed at
    pub failed_at: DateTime<Utc>,
    /// Last error
    pub last_error: String,
    /// Retry count
    pub retry_count: u32,
    /// Next retry at
    pub next_retry_at: DateTime<Utc>,
}

/// Dead Letter Queue
pub struct DeadLetterQueue {
    /// Channel sender
    sender: Sender<DlqMessage>,
    /// Channel receiver
    receiver: Receiver<DlqMessage>,
    /// Storage (corridor_id -> messages)
    storage: Arc<RwLock<HashMap<String, Vec<DlqMessage>>>>,
    /// Max size per corridor
    max_size: usize,
    /// Max retry attempts
    max_retry_attempts: u32,
}

impl DeadLetterQueue {
    /// Create new DLQ
    pub fn new(max_size: usize, max_retry_attempts: u32) -> Self {
        let (sender, receiver) = bounded(max_size * 10);

        Self {
            sender,
            receiver,
            storage: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            max_retry_attempts,
        }
    }

    /// Push message to DLQ
    pub async fn push(&self, request: TransferRequest, error: String) -> Result<()> {
        let corridor_id = request.corridor_id.clone();

        // Check size limit
        let storage = self.storage.read().await;
        if let Some(messages) = storage.get(&corridor_id) {
            if messages.len() >= self.max_size {
                return Err(Error::DlqFull {
                    current: messages.len(),
                    max: self.max_size,
                });
            }
        }
        drop(storage);

        let msg = DlqMessage {
            request,
            failed_at: Utc::now(),
            last_error: error,
            retry_count: 0,
            next_retry_at: Self::calculate_next_retry(0),
        };

        self.sender
            .send(msg)
            .await
            .map_err(|e| Error::Generic(format!("Failed to send to DLQ: {}", e)))?;

        info!("Message pushed to DLQ for corridor {}", corridor_id);
        Ok(())
    }

    /// Start DLQ processor
    pub async fn start_processor(self: Arc<Self>) {
        info!("Starting DLQ processor");

        loop {
            tokio::select! {
                // Receive new messages
                msg = self.receiver.recv() => {
                    if let Ok(msg) = msg {
                        let corridor_id = msg.request.corridor_id.clone();
                        let mut storage = self.storage.write().await;
                        storage.entry(corridor_id.clone()).or_insert_with(Vec::new).push(msg);
                        info!("Message stored in DLQ for corridor {}", corridor_id);
                    }
                }

                // Process retries (every 10 seconds)
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    self.process_retries().await;
                }
            }
        }
    }

    /// Process pending retries
    async fn process_retries(&self) {
        let now = Utc::now();
        let mut storage = self.storage.write().await;

        for (corridor_id, messages) in storage.iter_mut() {
            let mut i = 0;
            while i < messages.len() {
                if messages[i].next_retry_at <= now {
                    let mut msg = messages.remove(i);

                    if msg.retry_count >= self.max_retry_attempts {
                        warn!(
                            "Max retries exceeded for corridor {}, transfer {}",
                            corridor_id, msg.request.transfer_id
                        );
                        // Move to permanent failure storage (TODO)
                        continue;
                    }

                    msg.retry_count += 1;
                    msg.next_retry_at = Self::calculate_next_retry(msg.retry_count);

                    info!(
                        "Retrying transfer {} for corridor {} (attempt {}/{})",
                        msg.request.transfer_id,
                        corridor_id,
                        msg.retry_count,
                        self.max_retry_attempts
                    );

                    // TODO: Trigger actual retry via adapter manager
                    // For now, just re-queue
                    messages.push(msg);
                } else {
                    i += 1;
                }
            }
        }
    }

    /// Calculate next retry time (exponential backoff)
    fn calculate_next_retry(attempt: u32) -> DateTime<Utc> {
        let base_delay_secs = 2u64.pow(attempt.min(6)); // Max 64 seconds
        let delay = Duration::from_secs(base_delay_secs);
        Utc::now() + chrono::Duration::from_std(delay).unwrap()
    }

    /// Get DLQ size for corridor
    pub async fn size(&self, corridor_id: &str) -> usize {
        let storage = self.storage.read().await;
        storage.get(corridor_id).map(|m| m.len()).unwrap_or(0)
    }

    /// Get all messages for corridor
    pub async fn get_messages(&self, corridor_id: &str) -> Vec<DlqMessage> {
        let storage = self.storage.read().await;
        storage.get(corridor_id).cloned().unwrap_or_default()
    }

    /// Clear DLQ for corridor
    pub async fn clear(&self, corridor_id: &str) {
        let mut storage = self.storage.write().await;
        storage.remove(corridor_id);
        info!("Cleared DLQ for corridor {}", corridor_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol_core::SettlementInstruction;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_dlq_push() {
        let dlq = Arc::new(DeadLetterQueue::new(100, 3));

        // Start processor in background
        let dlq_clone = dlq.clone();
        tokio::spawn(async move {
            dlq_clone.start_processor().await;
        });

        let request = TransferRequest {
            transfer_id: Uuid::new_v4(),
            instruction: SettlementInstruction {
                instruction_id: Uuid::new_v4(),
                from_bank: "BANKA".to_string(),
                to_bank: "BANKB".to_string(),
                amount: dec!(100.00),
                currency: "USD".to_string(),
                iso20022_pacs008: None,
                status: protocol_core::InstructionStatus::Pending,
                executed_at: None,
            },
            corridor_id: "TEST-CORRIDOR".to_string(),
            adapter_type: crate::AdapterType::Swift,
            created_at: Utc::now(),
            retry_count: 0,
        };

        assert!(dlq.push(request, "Test error".to_string()).await.is_ok());

        // Wait for processor to receive and store message
        tokio::time::sleep(Duration::from_millis(200)).await;

        assert_eq!(dlq.size("TEST-CORRIDOR").await, 1);
    }
}