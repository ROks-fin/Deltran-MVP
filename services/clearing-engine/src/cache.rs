use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Cache TTL constants (in seconds)
pub mod ttl {
    pub const CLEARING_WINDOW: u64 = 300;       // 5 minutes
    pub const NET_POSITION: u64 = 600;           // 10 minutes
    pub const ISO_MESSAGE: u64 = 3600;           // 1 hour
    pub const BANK_BALANCE: u64 = 60;            // 1 minute
    pub const NETTING_RESULT: u64 = 1800;        // 30 minutes
}

/// Cache key prefixes
pub mod keys {
    pub const CLEARING_WINDOW: &str = "clearing:window";
    pub const ACTIVE_WINDOW: &str = "clearing:window:active";
    pub const NET_POSITION: &str = "netting:bilateral";
    pub const MULTILATERAL_POSITION: &str = "netting:multilateral";
    pub const ISO_MESSAGE: &str = "iso20022";
    pub const BANK_BALANCE: &str = "balance";
    pub const NETTING_RESULT: &str = "netting:result";
}

#[derive(Clone)]
pub struct ClearingCache {
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
pub struct CachedClearingWindow {
    pub id: Uuid,
    pub status: String,
    pub start_time: String,
    pub end_time: String,
    pub obligations_count: i32,
    pub total_volume: String,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedNetPosition {
    pub bank_a_id: Uuid,
    pub bank_b_id: Uuid,
    pub currency: String,
    pub net_amount: String,
    pub direction: String,
    pub obligation_count: i32,
}

impl ClearingCache {
    pub fn new(redis: ConnectionManager) -> Self {
        ClearingCache {
            redis,
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        }
    }

    // =========================================================================
    // CLEARING WINDOW CACHE
    // =========================================================================

    /// Get active clearing window from cache
    pub async fn get_active_window(&self, region: &str) -> Option<CachedClearingWindow> {
        let key = format!("{}:{}", keys::ACTIVE_WINDOW, region);

        match self.redis.clone().get::<_, Option<String>>(&key).await {
            Ok(Some(json)) => {
                self.record_hit().await;
                match serde_json::from_str(&json) {
                    Ok(window) => Some(window),
                    Err(e) => {
                        warn!("Failed to deserialize cached window: {}", e);
                        None
                    }
                }
            }
            Ok(None) => {
                self.record_miss().await;
                None
            }
            Err(e) => {
                error!("Redis error getting active window: {}", e);
                self.record_miss().await;
                None
            }
        }
    }

    /// Cache active clearing window
    pub async fn set_active_window(&self, region: &str, window: &CachedClearingWindow) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}", keys::ACTIVE_WINDOW, region);
        let json = serde_json::to_string(window).map_err(|e| {
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Serialization failed",
                e.to_string(),
            ))
        })?;

        let _: () = self.redis.clone().set_ex(&key, json, ttl::CLEARING_WINDOW).await?;
        self.record_set().await;
        info!("Cached active window for region {}", region);
        Ok(())
    }

    /// Invalidate active window cache
    pub async fn invalidate_active_window(&self, region: &str) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}", keys::ACTIVE_WINDOW, region);
        let _: () = self.redis.clone().del(&key).await?;
        self.record_delete().await;
        info!("Invalidated active window cache for region {}", region);
        Ok(())
    }

    // =========================================================================
    // NET POSITION CACHE
    // =========================================================================

    /// Get bilateral net position from cache
    pub async fn get_bilateral_net_position(
        &self,
        window_id: Uuid,
        currency: &str,
        bank_a: Uuid,
        bank_b: Uuid,
    ) -> Option<CachedNetPosition> {
        let key = format!(
            "{}:{}:{}:{}:{}",
            keys::NET_POSITION, window_id, currency, bank_a, bank_b
        );

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
                error!("Redis error getting net position: {}", e);
                self.record_miss().await;
                None
            }
        }
    }

    /// Cache bilateral net position
    pub async fn set_bilateral_net_position(
        &self,
        window_id: Uuid,
        currency: &str,
        position: &CachedNetPosition,
    ) -> Result<(), redis::RedisError> {
        let key = format!(
            "{}:{}:{}:{}:{}",
            keys::NET_POSITION, window_id, currency, position.bank_a_id, position.bank_b_id
        );
        let json = serde_json::to_string(position).map_err(|e| {
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Serialization failed",
                e.to_string(),
            ))
        })?;

        let _: () = self.redis.clone().set_ex(&key, json, ttl::NET_POSITION).await?;
        self.record_set().await;
        Ok(())
    }

    /// Invalidate all net positions for a clearing window
    pub async fn invalidate_window_net_positions(&self, window_id: Uuid) -> Result<u64, redis::RedisError> {
        let pattern = format!("{}:{}:*", keys::NET_POSITION, window_id);
        let keys: Vec<String> = self.redis.clone().keys(&pattern).await?;

        let count = keys.len() as u64;
        if !keys.is_empty() {
            let _: () = self.redis.clone().del(&keys).await?;
            for _ in 0..count {
                self.record_delete().await;
            }
        }

        info!("Invalidated {} net position caches for window {}", count, window_id);
        Ok(count)
    }

    // =========================================================================
    // ISO 20022 MESSAGE CACHE
    // =========================================================================

    /// Get cached ISO 20022 message
    pub async fn get_iso_message<T: for<'de> Deserialize<'de>>(
        &self,
        message_type: &str,
        transaction_id: Uuid,
    ) -> Option<T> {
        let key = format!("{}:{}:{}", keys::ISO_MESSAGE, message_type, transaction_id);

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
                error!("Redis error getting ISO message: {}", e);
                self.record_miss().await;
                None
            }
        }
    }

    /// Cache ISO 20022 message
    pub async fn set_iso_message<T: Serialize>(
        &self,
        message_type: &str,
        transaction_id: Uuid,
        message: &T,
    ) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}:{}", keys::ISO_MESSAGE, message_type, transaction_id);
        let json = serde_json::to_string(message).map_err(|e| {
            redis::RedisError::from((
                redis::ErrorKind::TypeError,
                "Serialization failed",
                e.to_string(),
            ))
        })?;

        let _: () = self.redis.clone().set_ex(&key, json, ttl::ISO_MESSAGE).await?;
        self.record_set().await;
        Ok(())
    }

    // =========================================================================
    // NETTING RESULT CACHE
    // =========================================================================

    /// Get cached netting result
    pub async fn get_netting_result(&self, window_id: Uuid, currency: &str) -> Option<String> {
        let key = format!("{}:{}:{}", keys::NETTING_RESULT, window_id, currency);

        match self.redis.clone().get::<_, Option<String>>(&key).await {
            Ok(Some(json)) => {
                self.record_hit().await;
                Some(json)
            }
            Ok(None) => {
                self.record_miss().await;
                None
            }
            Err(e) => {
                error!("Redis error getting netting result: {}", e);
                self.record_miss().await;
                None
            }
        }
    }

    /// Cache netting result
    pub async fn set_netting_result(
        &self,
        window_id: Uuid,
        currency: &str,
        result: &str,
    ) -> Result<(), redis::RedisError> {
        let key = format!("{}:{}:{}", keys::NETTING_RESULT, window_id, currency);
        let _: () = self.redis.clone().set_ex(&key, result, ttl::NETTING_RESULT).await?;
        self.record_set().await;
        info!("Cached netting result for window {} currency {}", window_id, currency);
        Ok(())
    }

    // =========================================================================
    // CACHE WARMING
    // =========================================================================

    /// Warm cache for a clearing window
    pub async fn warm_window_cache(
        &self,
        window: &CachedClearingWindow,
        net_positions: &[CachedNetPosition],
    ) -> Result<(), redis::RedisError> {
        info!("Warming cache for window {}", window.id);

        // Cache the window
        self.set_active_window(&window.region, window).await?;

        // Cache all net positions
        for position in net_positions {
            self.set_bilateral_net_position(
                window.id,
                &position.currency,
                position,
            ).await?;
        }

        info!(
            "Cache warmed: window {} with {} net positions",
            window.id,
            net_positions.len()
        );

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

        // 100% hit rate
        metrics.hits = 10;
        metrics.misses = 0;
        assert_eq!(metrics.hit_rate(), 100.0);
    }
}
