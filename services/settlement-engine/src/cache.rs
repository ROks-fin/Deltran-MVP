use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;
use rust_decimal::Decimal;

/// Cache TTL constants (in seconds)
pub mod ttl {
    pub const SETTLEMENT_STATUS: u64 = 60;      // 1 minute
    pub const BANK_ACCOUNT: u64 = 300;          // 5 minutes
    pub const SETTLEMENT_BATCH: u64 = 600;      // 10 minutes
    pub const RECONCILIATION: u64 = 1800;       // 30 minutes
}

/// Cache key prefixes
pub mod keys {
    pub const SETTLEMENT_STATUS: &str = "settlement:status";
    pub const BANK_ACCOUNT: &str = "settlement:account";
    pub const SETTLEMENT_BATCH: &str = "settlement:batch";
    pub const PENDING_SETTLEMENTS: &str = "settlement:pending";
    pub const RECONCILIATION: &str = "settlement:reconciliation";
}

#[derive(Clone)]
pub struct SettlementCache {
    redis: ConnectionManager,
    metrics: Arc<RwLock<CacheMetrics>>,
}

#[derive(Default, Debug)]
pub struct CacheMetrics {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub deletes: u64,
}

impl CacheMetrics {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            (self.hits as f64) / (total as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSettlementStatus {
    pub instruction_id: Uuid,
    pub status: String,
    pub amount: String,
    pub currency: String,
    pub sender_bank_id: Uuid,
    pub receiver_bank_id: Uuid,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedBankAccount {
    pub bank_id: Uuid,
    pub account_number: String,
    pub currency: String,
    pub balance: String,
    pub available_balance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSettlementBatch {
    pub batch_id: Uuid,
    pub clearing_window_id: i64,
    pub total_instructions: i32,
    pub total_amount: String,
    pub currency: String,
    pub status: String,
}

impl SettlementCache {
    pub fn new(redis: ConnectionManager) -> Self {
        SettlementCache {
            redis,
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        }
    }

    // =========================================================================
    // SETTLEMENT STATUS CACHE
    // =========================================================================

    /// Get settlement status from cache
    pub async fn get_settlement_status(&self, instruction_id: Uuid) -> Option<CachedSettlementStatus> {
        let key = format!("{}:{}", keys::SETTLEMENT_STATUS, instruction_id);

        match self.redis.clone().get::<_, Option<String>>(&key).await {
            Ok(Some(json)) => {
                self.record_hit().await;
                match serde_json::from_str(&json) {
                    Ok(status) => Some(status),
                    Err(e) => {
                        warn!("Failed to deserialize cached settlement status: {}", e);
                        None
                    }
                }
            }
            Ok(None) => {
                self.record_miss().await;
                None
            }
            Err(e) => {
                error!("Redis error getting settlement status: {}", e);
                self.record_miss().await;
                None
            }
        }
    }

    /// Cache settlement status
    pub async fn set_settlement_status(&self, status: &CachedSettlementStatus) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}", keys::SETTLEMENT_STATUS, status.instruction_id);
        let json = serde_json::to_string(status).map_err(|e| {
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Serialization failed",
                e.to_string(),
            ))
        })?;

        let _: () = self.redis.clone().set_ex(&key, json, ttl::SETTLEMENT_STATUS).await?;
        self.record_set().await;
        Ok(())
    }

    /// Invalidate settlement status cache
    pub async fn invalidate_settlement_status(&self, instruction_id: Uuid) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}", keys::SETTLEMENT_STATUS, instruction_id);
        let _: () = self.redis.clone().del(&key).await?;
        self.record_delete().await;
        Ok(())
    }

    // =========================================================================
    // BANK ACCOUNT CACHE
    // =========================================================================

    /// Get bank account balance from cache
    pub async fn get_bank_account(&self, bank_id: Uuid, currency: &str) -> Option<CachedBankAccount> {
        let key = format!("{}:{}:{}", keys::BANK_ACCOUNT, bank_id, currency);

        match self.redis.clone().get::<_, Option<String>>(&key).await {
            Ok(Some(json)) => {
                self.record_hit().await;
                serde_json::from_str(&json).ok()
            }
            Ok(None) => {
                self.record_miss().await;
                None
            }
            Err(e) => {
                error!("Redis error getting bank account: {}", e);
                self.record_miss().await;
                None
            }
        }
    }

    /// Cache bank account balance
    pub async fn set_bank_account(&self, account: &CachedBankAccount) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}:{}", keys::BANK_ACCOUNT, account.bank_id, account.currency);
        let json = serde_json::to_string(account).map_err(|e| {
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Serialization failed",
                e.to_string(),
            ))
        })?;

        let _: () = self.redis.clone().set_ex(&key, json, ttl::BANK_ACCOUNT).await?;
        self.record_set().await;
        Ok(())
    }

    /// Invalidate bank account cache
    pub async fn invalidate_bank_account(&self, bank_id: Uuid, currency: &str) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}:{}", keys::BANK_ACCOUNT, bank_id, currency);
        let _: () = self.redis.clone().del(&key).await?;
        self.record_delete().await;
        info!("Invalidated bank account cache for {} {}", bank_id, currency);
        Ok(())
    }

    // =========================================================================
    // SETTLEMENT BATCH CACHE
    // =========================================================================

    /// Get settlement batch from cache
    pub async fn get_settlement_batch(&self, batch_id: Uuid) -> Option<CachedSettlementBatch> {
        let key = format!("{}:{}", keys::SETTLEMENT_BATCH, batch_id);

        match self.redis.clone().get::<_, Option<String>>(&key).await {
            Ok(Some(json)) => {
                self.record_hit().await;
                serde_json::from_str(&json).ok()
            }
            Ok(None) => {
                self.record_miss().await;
                None
            }
            Err(e) => {
                error!("Redis error getting settlement batch: {}", e);
                self.record_miss().await;
                None
            }
        }
    }

    /// Cache settlement batch
    pub async fn set_settlement_batch(&self, batch: &CachedSettlementBatch) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}", keys::SETTLEMENT_BATCH, batch.batch_id);
        let json = serde_json::to_string(batch).map_err(|e| {
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Serialization failed",
                e.to_string(),
            ))
        })?;

        let _: () = self.redis.clone().set_ex(&key, json, ttl::SETTLEMENT_BATCH).await?;
        self.record_set().await;
        info!("Cached settlement batch {}", batch.batch_id);
        Ok(())
    }

    // =========================================================================
    // PENDING SETTLEMENTS QUEUE
    // =========================================================================

    /// Add instruction to pending settlements queue
    pub async fn add_pending_settlement(&self, clearing_window: i64, instruction_id: Uuid) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}", keys::PENDING_SETTLEMENTS, clearing_window);
        let _: () = self.redis.clone().sadd(&key, instruction_id.to_string()).await?;
        self.record_set().await;
        Ok(())
    }

    /// Get all pending settlements for a clearing window
    pub async fn get_pending_settlements(&self, clearing_window: i64) -> Vec<Uuid> {
        let key = format!("{}:{}", keys::PENDING_SETTLEMENTS, clearing_window);

        match self.redis.clone().smembers::<_, Vec<String>>(&key).await {
            Ok(members) => {
                self.record_hit().await;
                members
                    .iter()
                    .filter_map(|s| Uuid::parse_str(s).ok())
                    .collect()
            }
            Err(e) => {
                error!("Redis error getting pending settlements: {}", e);
                self.record_miss().await;
                Vec::new()
            }
        }
    }

    /// Remove instruction from pending settlements
    pub async fn remove_pending_settlement(&self, clearing_window: i64, instruction_id: Uuid) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}", keys::PENDING_SETTLEMENTS, clearing_window);
        let _: () = self.redis.clone().srem(&key, instruction_id.to_string()).await?;
        self.record_delete().await;
        Ok(())
    }

    // =========================================================================
    // METRICS
    // =========================================================================

    async fn record_hit(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.hits += 1;
    }

    async fn record_miss(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.misses += 1;
    }

    async fn record_set(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.sets += 1;
    }

    async fn record_delete(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.deletes += 1;
    }

    /// Get current cache metrics
    pub async fn get_metrics(&self) -> CacheMetrics {
        let metrics = self.metrics.read().await;
        CacheMetrics {
            hits: metrics.hits,
            misses: metrics.misses,
            sets: metrics.sets,
            deletes: metrics.deletes,
        }
    }

    /// Reset cache metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.write().await;
        *metrics = CacheMetrics::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_metrics_hit_rate() {
        let mut metrics = CacheMetrics::default();

        // 0 total = 0% hit rate
        assert_eq!(metrics.hit_rate(), 0.0);

        // 8 hits, 2 misses = 80% hit rate
        metrics.hits = 8;
        metrics.misses = 2;
        assert_eq!(metrics.hit_rate(), 80.0);
    }
}
