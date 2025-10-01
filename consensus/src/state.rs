//! Consensus state management

use crate::{Error, Result};
use ledger_core::types::LedgerEvent;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Consensus state
#[derive(Debug)]
pub struct ConsensusState {
    /// Current block height
    height: Arc<RwLock<u64>>,

    /// Current app hash (Merkle root)
    app_hash: Arc<RwLock<Vec<u8>>>,

    /// Pending transactions
    mempool: Arc<RwLock<Vec<Transaction>>>,

    /// Last committed block ID
    last_block_id: Arc<RwLock<Option<Uuid>>>,
}

impl ConsensusState {
    /// Create new consensus state
    pub fn new() -> Self {
        Self {
            height: Arc::new(RwLock::new(0)),
            app_hash: Arc::new(RwLock::new(vec![0u8; 32])),
            mempool: Arc::new(RwLock::new(Vec::new())),
            last_block_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Get current height
    pub async fn height(&self) -> u64 {
        *self.height.read().await
    }

    /// Increment height
    pub async fn increment_height(&self) {
        let mut height = self.height.write().await;
        *height += 1;
    }

    /// Get app hash
    pub async fn app_hash(&self) -> Vec<u8> {
        self.app_hash.read().await.clone()
    }

    /// Set app hash
    pub async fn set_app_hash(&self, hash: Vec<u8>) {
        let mut app_hash = self.app_hash.write().await;
        *app_hash = hash;
    }

    /// Add transaction to mempool
    pub async fn add_to_mempool(&self, tx: Transaction) {
        let mut mempool = self.mempool.write().await;
        mempool.push(tx);
    }

    /// Get mempool transactions
    pub async fn get_mempool(&self) -> Vec<Transaction> {
        self.mempool.read().await.clone()
    }

    /// Clear mempool
    pub async fn clear_mempool(&self) {
        let mut mempool = self.mempool.write().await;
        mempool.clear();
    }

    /// Set last block ID
    pub async fn set_last_block_id(&self, block_id: Uuid) {
        let mut last_block_id = self.last_block_id.write().await;
        *last_block_id = Some(block_id);
    }

    /// Get last block ID
    pub async fn last_block_id(&self) -> Option<Uuid> {
        *self.last_block_id.read().await
    }
}

impl Default for ConsensusState {
    fn default() -> Self {
        Self::new()
    }
}

/// Transaction wrapping ledger event
#[derive(Debug, Clone)]
pub struct Transaction {
    /// Transaction ID
    pub tx_id: Uuid,

    /// Ledger event
    pub event: LedgerEvent,

    /// Transaction hash
    pub hash: Vec<u8>,
}

impl Transaction {
    /// Create new transaction
    pub fn new(event: LedgerEvent) -> Self {
        let tx_id = Uuid::new_v4();
        let hash = Self::compute_hash(&event);

        Self {
            tx_id,
            event,
            hash,
        }
    }

    /// Compute transaction hash
    fn compute_hash(event: &LedgerEvent) -> Vec<u8> {
        use sha2::{Digest, Sha256};

        let bytes = bincode::serialize(event).unwrap_or_default();
        let hash = Sha256::digest(&bytes);
        hash.to_vec()
    }

    /// Serialize transaction
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self)
            .map_err(|e| Error::Serialization(format!("Failed to serialize tx: {}", e)))
    }

    /// Deserialize transaction
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes)
            .map_err(|e| Error::Serialization(format!("Failed to deserialize tx: {}", e)))
    }
}

/// Validator information
#[derive(Debug, Clone)]
pub struct Validator {
    /// Validator address
    pub address: Vec<u8>,

    /// Public key
    pub pub_key: Vec<u8>,

    /// Voting power
    pub power: u64,
}

impl Validator {
    /// Create new validator
    pub fn new(address: Vec<u8>, pub_key: Vec<u8>, power: u64) -> Self {
        Self {
            address,
            pub_key,
            power,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ledger_core::types::{AccountId, Currency, EventType, Signature};
    use rust_decimal::Decimal;

    fn create_test_event() -> LedgerEvent {
        LedgerEvent {
            event_id: Uuid::new_v4(),
            payment_id: Uuid::new_v4(),
            event_type: EventType::PaymentInitiated,
            amount: Decimal::new(10000, 2),
            currency: Currency::USD,
            debtor: AccountId::new("US123"),
            creditor: AccountId::new("AE456"),
            timestamp_nanos: chrono::Utc::now().timestamp_nanos_opt().unwrap(),
            block_id: None,
            signature: Signature::from_bytes([0u8; 64]),
            previous_event_id: None,
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn test_consensus_state() {
        let state = ConsensusState::new();

        assert_eq!(state.height().await, 0);

        state.increment_height().await;
        assert_eq!(state.height().await, 1);

        let hash = vec![1u8; 32];
        state.set_app_hash(hash.clone()).await;
        assert_eq!(state.app_hash().await, hash);
    }

    #[tokio::test]
    async fn test_mempool() {
        let state = ConsensusState::new();

        let event = create_test_event();
        let tx = Transaction::new(event);

        state.add_to_mempool(tx.clone()).await;

        let mempool = state.get_mempool().await;
        assert_eq!(mempool.len(), 1);

        state.clear_mempool().await;
        assert_eq!(state.get_mempool().await.len(), 0);
    }

    #[test]
    fn test_transaction_serialization() {
        let event = create_test_event();
        let tx = Transaction::new(event);

        let bytes = tx.to_bytes().unwrap();
        let tx2 = Transaction::from_bytes(&bytes).unwrap();

        assert_eq!(tx.tx_id, tx2.tx_id);
        assert_eq!(tx.hash, tx2.hash);
    }
}