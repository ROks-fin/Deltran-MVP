//! Storage layer using RocksDB
//!
//! # Column Families
//!
//! - `events` - Append-only event log (key: event_id)
//! - `blocks` - Finalized blocks (key: block_height)
//! - `state` - Payment states (key: payment_id)
//! - `indices` - Secondary indices for fast lookups
//! - `merkle` - Merkle tree nodes (key: level || index)
//! - `snapshots` - Snapshot metadata (key: snapshot_id)

use crate::{
    error::{Error, Result},
    types::{Block, LedgerEvent, PaymentState},
    Config,
};
use parking_lot::RwLock;
use rocksdb::{
    ColumnFamily, ColumnFamilyDescriptor, DBCompactionStyle, IteratorMode, Options, WriteBatch, DB,
};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Column family names
const CF_EVENTS: &str = "events";
const CF_BLOCKS: &str = "blocks";
const CF_STATE: &str = "state";
const CF_INDICES: &str = "indices";
const CF_MERKLE: &str = "merkle";
const CF_SNAPSHOTS: &str = "snapshots";

/// Storage wrapper for RocksDB
pub struct Storage {
    db: Arc<DB>,
    // Column family handles are stored in DB, accessed by name
}

impl Storage {
    /// Open or create database
    pub fn open(config: &Config) -> Result<Self> {
        let path = &config.data_dir;

        // Create directory if not exists
        std::fs::create_dir_all(path)?;

        // Database options
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);

        // Tuning from config
        db_opts.set_write_buffer_size(config.rocksdb.write_buffer_size_mb * 1024 * 1024);
        db_opts.set_max_write_buffer_number(config.rocksdb.max_write_buffer_number);
        db_opts.set_target_file_size_base(config.rocksdb.target_file_size_mb * 1024 * 1024);
        db_opts.set_max_background_jobs(config.rocksdb.max_background_jobs);
        db_opts.set_level_zero_file_num_compaction_trigger(
            config.rocksdb.level0_file_num_compaction_trigger,
        );

        // Universal compaction for write-heavy workload
        db_opts.set_compaction_style(DBCompactionStyle::Universal);

        // Enable statistics
        if config.rocksdb.enable_statistics {
            db_opts.enable_statistics();
        }

        // Column family descriptors
        let cf_descriptors = vec![
            ColumnFamilyDescriptor::new(CF_EVENTS, Self::cf_options_events()),
            ColumnFamilyDescriptor::new(CF_BLOCKS, Self::cf_options_blocks()),
            ColumnFamilyDescriptor::new(CF_STATE, Self::cf_options_state()),
            ColumnFamilyDescriptor::new(CF_INDICES, Self::cf_options_indices()),
            ColumnFamilyDescriptor::new(CF_MERKLE, Self::cf_options_merkle()),
            ColumnFamilyDescriptor::new(CF_SNAPSHOTS, Self::cf_options_snapshots()),
        ];

        // Open database
        let db = DB::open_cf_descriptors(&db_opts, path, cf_descriptors)?;

        tracing::info!(
            "Opened RocksDB at {:?} with {} column families",
            path,
            db.cf_handle(CF_EVENTS).is_some() as usize
                + db.cf_handle(CF_BLOCKS).is_some() as usize
                + db.cf_handle(CF_STATE).is_some() as usize
                + db.cf_handle(CF_INDICES).is_some() as usize
                + db.cf_handle(CF_MERKLE).is_some() as usize
                + db.cf_handle(CF_SNAPSHOTS).is_some() as usize
        );

        Ok(Self { db: Arc::new(db) })
    }

    // Column family options

    fn cf_options_events() -> Options {
        let mut opts = Options::default();
        opts.set_compression_type(rocksdb::DBCompressionType::Zstd);
        opts.set_bottommost_compression_type(rocksdb::DBCompressionType::Zstd);
        opts
    }

    fn cf_options_blocks() -> Options {
        let mut opts = Options::default();
        opts.set_compression_type(rocksdb::DBCompressionType::Zstd);
        opts
    }

    fn cf_options_state() -> Options {
        let mut opts = Options::default();
        // State is frequently read, use LZ4 for speed
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        opts
    }

    fn cf_options_indices() -> Options {
        let mut opts = Options::default();
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        // Indices benefit from bloom filters
        let mut block_opts = rocksdb::BlockBasedOptions::default();
        block_opts.set_bloom_filter(10.0, false); // 10 bits per key
        opts.set_block_based_table_factory(&block_opts);
        opts
    }

    fn cf_options_merkle() -> Options {
        let mut opts = Options::default();
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        opts
    }

    fn cf_options_snapshots() -> Options {
        let mut opts = Options::default();
        opts.set_compression_type(rocksdb::DBCompressionType::Zstd);
        opts
    }

    // Helper: get column family handle

    fn cf_handle(&self, name: &str) -> Result<&ColumnFamily> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| Error::Storage(format!("Column family {} not found", name)))
    }

    // Event operations

    /// Append event (single, unbatched)
    pub fn append_event(&self, event: &LedgerEvent) -> Result<()> {
        let cf = self.cf_handle(CF_EVENTS)?;
        let key = event.event_id.as_bytes();
        let value = bincode::serialize(event)?;

        self.db.put_cf(cf, key, &value)?;

        tracing::debug!(
            event_id = %event.event_id,
            payment_id = %event.payment_id,
            "Event appended"
        );

        Ok(())
    }

    /// Get event by ID
    pub fn get_event(&self, event_id: Uuid) -> Result<LedgerEvent> {
        let cf = self.cf_handle(CF_EVENTS)?;
        let key = event_id.as_bytes();

        let value = self
            .db
            .get_cf(cf, key)?
            .ok_or_else(|| Error::EventNotFound(event_id.to_string()))?;

        let event: LedgerEvent = bincode::deserialize(&value)?;
        Ok(event)
    }

    /// Get events by payment ID (via index)
    pub fn get_payment_events(&self, payment_id: Uuid) -> Result<Vec<LedgerEvent>> {
        let cf_indices = self.cf_handle(CF_INDICES)?;

        // Scan index: payment_id || event_id
        let prefix = Self::index_key_payment_event(&payment_id, None);
        let prefix_bytes = &prefix[..16]; // First 16 bytes = payment_id

        let iter = self.db.prefix_iterator_cf(cf_indices, prefix_bytes);

        let mut events = Vec::new();
        for item in iter {
            let (key, _) = item?;

            // Extract event_id from key (bytes 16..32)
            if key.len() >= 32 {
                let event_id_bytes: [u8; 16] = key[16..32].try_into().unwrap();
                let event_id = Uuid::from_bytes(event_id_bytes);

                let event = self.get_event(event_id)?;
                events.push(event);
            }
        }

        Ok(events)
    }

    // Payment state operations

    /// Put payment state
    pub fn put_payment_state(&self, state: &PaymentState) -> Result<()> {
        let cf = self.cf_handle(CF_STATE)?;
        let key = state.payment_id.as_bytes();
        let value = bincode::serialize(state)?;

        self.db.put_cf(cf, key, &value)?;

        Ok(())
    }

    /// Get payment state by ID
    pub fn get_payment_state(&self, payment_id: Uuid) -> Result<PaymentState> {
        let cf = self.cf_handle(CF_STATE)?;
        let key = payment_id.as_bytes();

        let value = self
            .db
            .get_cf(cf, key)?
            .ok_or_else(|| Error::PaymentNotFound(payment_id.to_string()))?;

        let state: PaymentState = bincode::deserialize(&value)?;
        Ok(state)
    }

    // Block operations

    /// Put block
    pub fn put_block(&self, block: &Block) -> Result<()> {
        let cf = self.cf_handle(CF_BLOCKS)?;
        let key = block.block_height.to_be_bytes();
        let value = bincode::serialize(block)?;

        self.db.put_cf(cf, &key, &value)?;

        tracing::info!(
            block_id = %block.block_id,
            block_height = block.block_height,
            event_count = block.event_count,
            "Block finalized"
        );

        Ok(())
    }

    /// Get block by height
    pub fn get_block(&self, height: u64) -> Result<Block> {
        let cf = self.cf_handle(CF_BLOCKS)?;
        let key = height.to_be_bytes();

        let value = self
            .db
            .get_cf(cf, &key)?
            .ok_or_else(|| Error::BlockNotFound(height.to_string()))?;

        let block: Block = bincode::deserialize(&value)?;
        Ok(block)
    }

    /// Get latest block
    pub fn get_latest_block(&self) -> Result<Option<Block>> {
        let cf = self.cf_handle(CF_BLOCKS)?;

        let iter = self.db.iterator_cf(cf, IteratorMode::End);

        for item in iter {
            let (_, value) = item?;
            let block: Block = bincode::deserialize(&value)?;
            return Ok(Some(block));
        }

        Ok(None)
    }

    // Batch operations (atomic)

    /// Append event with state update and indices (atomic)
    pub fn append_event_atomic(
        &self,
        event: &LedgerEvent,
        state: &PaymentState,
    ) -> Result<()> {
        let mut batch = WriteBatch::default();

        // 1. Event
        let cf_events = self.cf_handle(CF_EVENTS)?;
        let event_key = event.event_id.as_bytes();
        let event_value = bincode::serialize(event)?;
        batch.put_cf(cf_events, event_key, &event_value);

        // 2. Payment state
        let cf_state = self.cf_handle(CF_STATE)?;
        let state_key = state.payment_id.as_bytes();
        let state_value = bincode::serialize(state)?;
        batch.put_cf(cf_state, state_key, &state_value);

        // 3. Indices
        let cf_indices = self.cf_handle(CF_INDICES)?;

        // Index: payment_id || event_id -> empty
        let idx_payment_event = Self::index_key_payment_event(&event.payment_id, Some(event.event_id));
        batch.put_cf(cf_indices, &idx_payment_event, &[]);

        // Index: account (debtor) || payment_id -> empty
        let idx_debtor = Self::index_key_account_payment(&event.debtor, event.payment_id);
        batch.put_cf(cf_indices, &idx_debtor, &[]);

        // Index: account (creditor) || payment_id -> empty
        let idx_creditor = Self::index_key_account_payment(&event.creditor, event.payment_id);
        batch.put_cf(cf_indices, &idx_creditor, &[]);

        // Index: status || payment_id -> empty
        let idx_status = Self::index_key_status_payment(state.status, state.payment_id);
        batch.put_cf(cf_indices, &idx_status, &[]);

        // Atomic commit
        self.db.write(batch)?;

        Ok(())
    }

    // Index key helpers

    fn index_key_payment_event(payment_id: &Uuid, event_id: Option<Uuid>) -> Vec<u8> {
        let mut key = payment_id.as_bytes().to_vec();
        if let Some(eid) = event_id {
            key.extend_from_slice(eid.as_bytes());
        }
        key
    }

    fn index_key_account_payment(account: &crate::types::AccountId, payment_id: Uuid) -> Vec<u8> {
        let mut key = account.as_str().as_bytes().to_vec();
        key.push(b'|'); // Separator
        key.extend_from_slice(payment_id.as_bytes());
        key
    }

    fn index_key_status_payment(
        status: crate::types::PaymentStatus,
        payment_id: Uuid,
    ) -> Vec<u8> {
        let mut key = vec![status as u8];
        key.extend_from_slice(payment_id.as_bytes());
        key
    }

    // Statistics

    /// Get storage statistics
    pub fn get_stats(&self) -> Result<StorageStats> {
        let cf_events = self.cf_handle(CF_EVENTS)?;
        let cf_blocks = self.cf_handle(CF_BLOCKS)?;
        let cf_state = self.cf_handle(CF_STATE)?;

        // Count events (approximate, fast)
        let event_count = self.approximate_count(cf_events)?;

        // Count blocks
        let mut block_count = 0u64;
        let iter = self.db.iterator_cf(cf_blocks, IteratorMode::Start);
        for _ in iter {
            block_count += 1;
        }

        // Count payments
        let payment_count = self.approximate_count(cf_state)?;

        Ok(StorageStats {
            total_events: event_count,
            total_blocks: block_count,
            total_payments: payment_count,
        })
    }

    fn approximate_count(&self, cf: &ColumnFamily) -> Result<u64> {
        // RocksDB property for approximate count
        let prop = self
            .db
            .property_int_value_cf(cf, "rocksdb.estimate-num-keys")?
            .unwrap_or(0);

        Ok(prop)
    }

    /// Close database (graceful shutdown)
    pub fn close(self) -> Result<()> {
        drop(self.db);
        tracing::info!("RocksDB closed gracefully");
        Ok(())
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_events: u64,
    pub total_blocks: u64,
    pub total_payments: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Currency, EventType, PaymentStatus, Signature};
    use crate::Config;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use tempfile::TempDir;

    fn test_config() -> (Config, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.data_dir = temp_dir.path().to_path_buf();
        (config, temp_dir)
    }

    fn test_event() -> LedgerEvent {
        LedgerEvent {
            event_id: Uuid::new_v4(),
            payment_id: Uuid::new_v4(),
            event_type: EventType::PaymentInitiated,
            amount: Decimal::new(100000, 2), // 1000.00
            currency: Currency::USD,
            debtor: crate::types::AccountId::new("US1234567890"),
            creditor: crate::types::AccountId::new("AE9876543210"),
            timestamp_nanos: Utc::now().timestamp_nanos_opt().unwrap(),
            block_id: None,
            signature: Signature::from_bytes([0u8; 64]),
            previous_event_id: None,
            metadata: Default::default(),
        }
    }

    fn test_payment_state(payment_id: Uuid) -> PaymentState {
        PaymentState {
            payment_id,
            status: PaymentStatus::Initiated,
            amount: Decimal::new(100000, 2),
            currency: Currency::USD,
            debtor: crate::types::AccountId::new("US1234567890"),
            creditor: crate::types::AccountId::new("AE9876543210"),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            event_ids: vec![],
            current_block_id: None,
        }
    }

    #[test]
    fn test_storage_open() {
        let (config, _temp) = test_config();
        let storage = Storage::open(&config).unwrap();
        assert!(storage.db.cf_handle(CF_EVENTS).is_some());
        assert!(storage.db.cf_handle(CF_BLOCKS).is_some());
    }

    #[test]
    fn test_append_and_get_event() {
        let (config, _temp) = test_config();
        let storage = Storage::open(&config).unwrap();

        let event = test_event();
        let event_id = event.event_id;

        storage.append_event(&event).unwrap();

        let retrieved = storage.get_event(event_id).unwrap();
        assert_eq!(retrieved.event_id, event_id);
        assert_eq!(retrieved.payment_id, event.payment_id);
    }

    #[test]
    fn test_payment_state() {
        let (config, _temp) = test_config();
        let storage = Storage::open(&config).unwrap();

        let payment_id = Uuid::new_v4();
        let state = test_payment_state(payment_id);

        storage.put_payment_state(&state).unwrap();

        let retrieved = storage.get_payment_state(payment_id).unwrap();
        assert_eq!(retrieved.payment_id, payment_id);
        assert_eq!(retrieved.status, PaymentStatus::Initiated);
    }

    #[test]
    fn test_atomic_append() {
        let (config, _temp) = test_config();
        let storage = Storage::open(&config).unwrap();

        let event = test_event();
        let mut state = test_payment_state(event.payment_id);
        state.event_ids.push(event.event_id);

        storage.append_event_atomic(&event, &state).unwrap();

        // Verify event
        let retrieved_event = storage.get_event(event.event_id).unwrap();
        assert_eq!(retrieved_event.event_id, event.event_id);

        // Verify state
        let retrieved_state = storage.get_payment_state(event.payment_id).unwrap();
        assert_eq!(retrieved_state.payment_id, event.payment_id);
        assert_eq!(retrieved_state.event_ids.len(), 1);
    }

    #[test]
    fn test_get_payment_events() {
        let (config, _temp) = test_config();
        let storage = Storage::open(&config).unwrap();

        let payment_id = Uuid::new_v4();

        // Create 3 events for same payment
        for _ in 0..3 {
            let mut event = test_event();
            event.payment_id = payment_id;

            let mut state = test_payment_state(payment_id);
            state.event_ids.push(event.event_id);

            storage.append_event_atomic(&event, &state).unwrap();
        }

        // Get all events
        let events = storage.get_payment_events(payment_id).unwrap();
        assert_eq!(events.len(), 3);
        assert!(events.iter().all(|e| e.payment_id == payment_id));
    }
}