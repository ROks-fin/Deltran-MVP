//! Main ledger orchestration layer
//!
//! This module ties together storage, crypto, and actor components
//! into a high-level API for payment event processing.
//!
//! # Example
//!
//! ```no_run
//! use ledger_core::{Config, Ledger};
//!
//! #[tokio::main]
//! async fn main() -> ledger_core::Result<()> {
//!     let config = Config::default();
//!     let ledger = Ledger::open(config).await?;
//!
//!     // Append event
//!     // let event = ...;
//!     // let event_id = ledger.append_event(event).await?;
//!
//!     Ok(())
//! }
//! ```

use crate::{
    actor::{spawn_ledger_actor, LedgerHandle},
    crypto::{hash_event, merkle_root, KeyPair},
    types::{Block, EventType, LedgerEvent, PaymentState, PaymentStatus},
    Config, Error, Result, Storage,
};
use chrono::Utc;
use std::sync::Arc;
use tokio::time::Duration;
use uuid::Uuid;

/// Main ledger interface
pub struct Ledger {
    /// Actor handle for async operations
    handle: LedgerHandle,

    /// Direct storage access (for reads)
    storage: Arc<Storage>,

    /// Key pair for signing (if enabled)
    keypair: Option<KeyPair>,

    /// Configuration
    config: Config,
}

impl Ledger {
    /// Open ledger with configuration
    pub async fn open(config: Config) -> Result<Self> {
        // Open storage
        let storage = Arc::new(Storage::open(&config)?);

        // Spawn actor
        let handle = spawn_ledger_actor(
            storage.clone(),
            config.batching.max_batch_size,
            Duration::from_millis(config.batching.batch_timeout_ms),
            config.batching.enabled,
        );

        Ok(Self {
            handle,
            storage,
            keypair: None,
            config,
        })
    }

    /// Set signing key pair
    pub fn with_keypair(mut self, keypair: KeyPair) -> Self {
        self.keypair = Some(keypair);
        self
    }

    /// Append a new event
    ///
    /// This validates the event, signs it (if keypair present), and appends to the log.
    pub async fn append_event(&self, mut event: LedgerEvent) -> Result<Uuid> {
        // Sign event if keypair present
        if let Some(ref keypair) = self.keypair {
            let canonical_bytes = event.canonical_bytes();
            event.signature = keypair.sign(&canonical_bytes);
        }

        // Validate event
        self.validate_event(&event)?;

        // Append via actor
        self.handle.append_event(event).await
    }

    /// Get payment state
    pub async fn get_payment_state(&self, payment_id: Uuid) -> Result<PaymentState> {
        self.handle.get_payment_state(payment_id).await
    }

    /// Get payment events (full history)
    pub async fn get_payment_events(&self, payment_id: Uuid) -> Result<Vec<LedgerEvent>> {
        self.handle.get_payment_events(payment_id).await
    }

    /// Get event by ID
    pub async fn get_event(&self, event_id: Uuid) -> Result<LedgerEvent> {
        self.handle.get_event(event_id).await
    }

    /// Finalize block with Merkle root
    ///
    /// Takes a list of event IDs, computes Merkle root, and creates a finalized block.
    pub async fn finalize_block(&self, event_ids: Vec<Uuid>) -> Result<Block> {
        if event_ids.is_empty() {
            return Err(Error::InvalidEvent("Cannot finalize empty block".to_string()));
        }

        // Get all events
        let mut events = Vec::new();
        for event_id in &event_ids {
            let event = self.get_event(*event_id).await?;
            events.push(event);
        }

        // Compute Merkle root
        let event_hashes: Vec<[u8; 32]> = events.iter().map(|e| hash_event(e)).collect();
        let merkle_root_hash = merkle_root(&event_hashes);

        // Get previous block
        let previous_block = self.handle.get_latest_block().await?;
        let (block_height, previous_block_hash) = match previous_block {
            Some(prev) => (prev.block_height + 1, prev.block_hash),
            None => (0, [0u8; 32]), // Genesis block
        };

        // Create block
        let mut block = Block {
            block_id: Uuid::now_v7(),
            block_height,
            merkle_root: merkle_root_hash,
            previous_block_hash,
            block_hash: [0u8; 32], // Computed below
            event_ids: event_ids.clone(),
            event_count: event_ids.len() as u32,
            created_at: Utc::now(),
            proposer_signature: vec![],
            validator_signatures: vec![],
        };

        // Compute block hash
        block.block_hash = block.compute_hash();

        // Sign block if keypair present
        if let Some(ref keypair) = self.keypair {
            let signature = keypair.sign(&block.block_hash);
            block.proposer_signature = signature.as_bytes().to_vec();
        }

        // Store block
        self.handle.finalize_block(block.clone()).await?;

        Ok(block)
    }

    /// Get latest block
    pub async fn get_latest_block(&self) -> Result<Option<Block>> {
        self.handle.get_latest_block().await
    }

    /// Get block by height
    pub fn get_block_by_height(&self, height: u64) -> Result<Block> {
        self.storage.get_block(height)
    }

    /// Flush batch immediately (for testing/shutdown)
    pub async fn flush_batch(&self) -> Result<()> {
        self.handle.flush_batch().await
    }

    /// Shutdown ledger
    pub async fn shutdown(self) -> Result<()> {
        self.handle.shutdown().await
    }

    /// Validate event invariants
    fn validate_event(&self, event: &LedgerEvent) -> Result<()> {
        // Check amount is positive
        if event.amount <= rust_decimal::Decimal::ZERO {
            return Err(Error::InvalidEvent(
                "Amount must be positive".to_string(),
            ));
        }

        // Check timestamp is not in future
        let now = Utc::now().timestamp_nanos_opt().unwrap_or(0);
        if event.timestamp_nanos > now + 60_000_000_000 {
            // Allow 60s clock skew
            return Err(Error::InvalidEvent(
                "Timestamp is in the future".to_string(),
            ));
        }

        // Check signature if keypair present
        if let Some(ref keypair) = self.keypair {
            let public_key = keypair.public_key();
            if !event.verify_signature(&public_key) {
                return Err(Error::SignatureError(
                    "Invalid event signature".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Rebuild payment state from events (for verification)
    pub async fn rebuild_payment_state(&self, payment_id: Uuid) -> Result<PaymentState> {
        let events = self.get_payment_events(payment_id).await?;

        if events.is_empty() {
            return Err(Error::PaymentNotFound(payment_id.to_string()));
        }

        let first_event = &events[0];

        let mut state = PaymentState {
            payment_id,
            status: PaymentStatus::Initiated,
            amount: first_event.amount,
            currency: first_event.currency,
            debtor: first_event.debtor.clone(),
            creditor: first_event.creditor.clone(),
            created_at: chrono::DateTime::from_timestamp_nanos(first_event.timestamp_nanos),
            updated_at: chrono::DateTime::from_timestamp_nanos(first_event.timestamp_nanos),
            event_ids: vec![],
            current_block_id: None,
        };

        // Apply all events
        for event in &events {
            state.apply_event(event)?;
        }

        Ok(state)
    }

    /// Check money conservation invariant
    ///
    /// Verify that sum of all debits equals sum of all credits.
    /// This is a critical invariant for financial correctness.
    pub async fn check_money_conservation(&self, payment_id: Uuid) -> Result<bool> {
        let events = self.get_payment_events(payment_id).await?;

        let mut total_debits = rust_decimal::Decimal::ZERO;
        let mut total_credits = rust_decimal::Decimal::ZERO;

        for event in events {
            match event.event_type {
                EventType::PaymentInitiated => {
                    total_debits += event.amount;
                }
                EventType::PaymentCompleted => {
                    total_credits += event.amount;
                }
                _ => {
                    // Other events don't affect money conservation
                }
            }
        }

        Ok(total_debits == total_credits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AccountId, Currency, Signature};
    use rust_decimal::Decimal;
    use std::collections::HashMap;

    async fn create_test_ledger() -> Ledger {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.data_dir = temp_dir.path().to_path_buf();
        config.batching.enabled = false; // Disable batching for tests

        Ledger::open(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_ledger_open() {
        let ledger = create_test_ledger().await;
        ledger.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_append_and_retrieve_event() {
        let ledger = create_test_ledger().await;

        let event = LedgerEvent {
            event_id: Uuid::now_v7(),
            payment_id: Uuid::now_v7(),
            event_type: EventType::PaymentInitiated,
            amount: Decimal::new(10000, 2), // $100.00
            currency: Currency::USD,
            debtor: AccountId::new("US123456789"),
            creditor: AccountId::new("AE987654321"),
            timestamp_nanos: Utc::now().timestamp_nanos_opt().unwrap(),
            block_id: None,
            signature: Signature::from_bytes([0u8; 64]),
            previous_event_id: None,
            metadata: HashMap::new(),
        };

        let event_id = ledger.append_event(event.clone()).await.unwrap();
        assert_eq!(event_id, event.event_id);

        let retrieved = ledger.get_event(event_id).await.unwrap();
        assert_eq!(retrieved.payment_id, event.payment_id);
        assert_eq!(retrieved.amount, event.amount);

        ledger.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_payment_state_tracking() {
        let ledger = create_test_ledger().await;
        let payment_id = Uuid::now_v7();

        // Event 1: Payment initiated
        let event1 = LedgerEvent {
            event_id: Uuid::now_v7(),
            payment_id,
            event_type: EventType::PaymentInitiated,
            amount: Decimal::new(50000, 2), // $500.00
            currency: Currency::USD,
            debtor: AccountId::new("US123"),
            creditor: AccountId::new("AE456"),
            timestamp_nanos: Utc::now().timestamp_nanos_opt().unwrap(),
            block_id: None,
            signature: Signature::from_bytes([0u8; 64]),
            previous_event_id: None,
            metadata: HashMap::new(),
        };

        ledger.append_event(event1.clone()).await.unwrap();

        // Event 2: Validation passed
        let event2 = LedgerEvent {
            event_id: Uuid::now_v7(),
            payment_id,
            event_type: EventType::ValidationPassed,
            amount: event1.amount,
            currency: event1.currency,
            debtor: event1.debtor.clone(),
            creditor: event1.creditor.clone(),
            timestamp_nanos: Utc::now().timestamp_nanos_opt().unwrap(),
            block_id: None,
            signature: Signature::from_bytes([0u8; 64]),
            previous_event_id: Some(event1.event_id),
            metadata: HashMap::new(),
        };

        ledger.append_event(event2).await.unwrap();

        // Get payment events
        let events = ledger.get_payment_events(payment_id).await.unwrap();
        assert_eq!(events.len(), 2);

        // Rebuild state
        let state = ledger.rebuild_payment_state(payment_id).await.unwrap();
        assert_eq!(state.status, PaymentStatus::Validated);
        assert_eq!(state.event_ids.len(), 2);

        ledger.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_block_finalization() {
        let ledger = create_test_ledger().await;

        // Append 3 events
        let mut event_ids = Vec::new();
        for _ in 0..3 {
            let event = LedgerEvent {
                event_id: Uuid::now_v7(),
                payment_id: Uuid::now_v7(),
                event_type: EventType::PaymentInitiated,
                amount: Decimal::new(10000, 2),
                currency: Currency::USD,
                debtor: AccountId::new("US123"),
                creditor: AccountId::new("AE456"),
                timestamp_nanos: Utc::now().timestamp_nanos_opt().unwrap(),
                block_id: None,
                signature: Signature::from_bytes([0u8; 64]),
                previous_event_id: None,
                metadata: HashMap::new(),
            };

            let event_id = ledger.append_event(event).await.unwrap();
            event_ids.push(event_id);
        }

        // Finalize block
        let block = ledger.finalize_block(event_ids.clone()).await.unwrap();
        assert_eq!(block.event_count, 3);
        assert_eq!(block.event_ids.len(), 3);
        assert_eq!(block.block_height, 0); // First block

        // Get latest block
        let latest = ledger.get_latest_block().await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().block_id, block.block_id);

        ledger.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_validate_event_positive_amount() {
        let ledger = create_test_ledger().await;

        let event = LedgerEvent {
            event_id: Uuid::now_v7(),
            payment_id: Uuid::now_v7(),
            event_type: EventType::PaymentInitiated,
            amount: Decimal::ZERO, // Invalid: zero amount
            currency: Currency::USD,
            debtor: AccountId::new("US123"),
            creditor: AccountId::new("AE456"),
            timestamp_nanos: Utc::now().timestamp_nanos_opt().unwrap(),
            block_id: None,
            signature: Signature::from_bytes([0u8; 64]),
            previous_event_id: None,
            metadata: HashMap::new(),
        };

        let result = ledger.append_event(event).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("positive"));

        ledger.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_signed_event() {
        let ledger = create_test_ledger().await;
        let keypair = KeyPair::generate();
        let ledger = ledger.with_keypair(keypair);

        let event = LedgerEvent {
            event_id: Uuid::now_v7(),
            payment_id: Uuid::now_v7(),
            event_type: EventType::PaymentInitiated,
            amount: Decimal::new(10000, 2),
            currency: Currency::USD,
            debtor: AccountId::new("US123"),
            creditor: AccountId::new("AE456"),
            timestamp_nanos: Utc::now().timestamp_nanos_opt().unwrap(),
            block_id: None,
            signature: Signature::from_bytes([0u8; 64]), // Will be replaced
            previous_event_id: None,
            metadata: HashMap::new(),
        };

        // Append with automatic signing
        let event_id = ledger.append_event(event).await.unwrap();

        // Retrieve and verify signature
        let retrieved = ledger.get_event(event_id).await.unwrap();
        // Signature should be different from dummy signature
        assert_ne!(retrieved.signature.as_bytes(), &[0u8; 64]);

        ledger.shutdown().await.unwrap();
    }
}