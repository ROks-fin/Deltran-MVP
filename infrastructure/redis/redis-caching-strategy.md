# DelTran MVP - Redis Caching Strategy
## Performance Optimization for Obligation and Clearing Engines

This document outlines the multi-layer caching strategy for high-throughput clearing and netting operations.

---

## Overview

The caching strategy is designed to minimize database queries during critical clearing operations while maintaining data consistency. We use Redis with different TTL (Time-To-Live) values for each cache layer based on data volatility.

---

## Cache Layers

### Layer 1: Active Clearing Window Cache
**Purpose**: Cache the currently active clearing window to avoid repeated database lookups.

**Key Pattern**: `clearing:window:active:{region}`

**Example**:
```redis
SET clearing:window:active:Global "{\"id\":\"uuid-here\",\"status\":\"Open\",\"start_time\":\"2025-11-21T00:00:00Z\",\"end_time\":\"2025-11-21T23:59:59Z\"}"
EXPIRE clearing:window:active:Global 300
```

**TTL**: 5 minutes (300 seconds)

**When to Invalidate**:
- When clearing window status changes (Scheduled → Open → InProgress → Completed)
- When a new clearing window is created

**Rust Implementation**:
```rust
use redis::{AsyncCommands, RedisResult};

pub struct ClearingWindowCache {
    redis: redis::aio::MultiplexedConnection,
}

impl ClearingWindowCache {
    pub async fn get_active_window(&mut self, region: &str) -> RedisResult<Option<ClearingWindow>> {
        let key = format!("clearing:window:active:{}", region);
        let result: Option<String> = self.redis.get(&key).await?;

        match result {
            Some(json) => Ok(serde_json::from_str(&json).ok()),
            None => Ok(None),
        }
    }

    pub async fn set_active_window(&mut self, region: &str, window: &ClearingWindow) -> RedisResult<()> {
        let key = format!("clearing:window:active:{}", region);
        let json = serde_json::to_string(window).unwrap();

        self.redis.set_ex(&key, json, 300).await
    }

    pub async fn invalidate_window(&mut self, region: &str) -> RedisResult<()> {
        let key = format!("clearing:window:active:{}", region);
        self.redis.del(&key).await
    }
}
```

---

### Layer 2: Net Position Cache
**Purpose**: Cache pre-calculated net positions for bilateral and multilateral netting.

**Key Patterns**:
- Bilateral: `netting:bilateral:{window_id}:{currency}:{bank_a}:{bank_b}`
- Multilateral: `netting:multilateral:{window_id}:{currency}:{bank_id}`

**Example**:
```redis
HSET netting:bilateral:uuid-window:USD:bank-icici:bank-hdfc net_amount "50000.00"
HSET netting:bilateral:uuid-window:USD:bank-icici:bank-hdfc direction "ICICI_TO_HDFC"
HSET netting:bilateral:uuid-window:USD:bank-icici:bank-hdfc obligation_count "15"
EXPIRE netting:bilateral:uuid-window:USD:bank-icici:bank-hdfc 600
```

**TTL**: 10 minutes (600 seconds)

**When to Invalidate**:
- When new obligations are added to the clearing window
- When netting is recalculated
- When clearing window closes

**Rust Implementation**:
```rust
pub struct NettingCache {
    redis: redis::aio::MultiplexedConnection,
}

impl NettingCache {
    pub async fn get_bilateral_net(
        &mut self,
        window_id: &str,
        currency: &str,
        bank_a: &str,
        bank_b: &str,
    ) -> RedisResult<Option<BilateralNetPosition>> {
        let key = format!("netting:bilateral:{}:{}:{}:{}", window_id, currency, bank_a, bank_b);

        let result: Vec<(String, String)> = self.redis.hgetall(&key).await?;
        if result.is_empty() {
            return Ok(None);
        }

        // Parse hash into struct
        let mut net_pos = BilateralNetPosition::default();
        for (field, value) in result {
            match field.as_str() {
                "net_amount" => net_pos.net_amount = value.parse().ok(),
                "direction" => net_pos.direction = Some(value),
                "obligation_count" => net_pos.obligation_count = value.parse().ok(),
                _ => {}
            }
        }

        Ok(Some(net_pos))
    }

    pub async fn set_bilateral_net(
        &mut self,
        window_id: &str,
        currency: &str,
        bank_a: &str,
        bank_b: &str,
        net_pos: &BilateralNetPosition,
    ) -> RedisResult<()> {
        let key = format!("netting:bilateral:{}:{}:{}:{}", window_id, currency, bank_a, bank_b);

        self.redis.hset_multiple(&key, &[
            ("net_amount", net_pos.net_amount.to_string()),
            ("direction", net_pos.direction.clone().unwrap_or_default()),
            ("obligation_count", net_pos.obligation_count.to_string()),
        ]).await?;

        self.redis.expire(&key, 600).await
    }

    pub async fn invalidate_window_netting(&mut self, window_id: &str) -> RedisResult<()> {
        let pattern = format!("netting:*:{}:*", window_id);
        let keys: Vec<String> = self.redis.keys(&pattern).await?;

        if !keys.is_empty() {
            self.redis.del(keys).await?;
        }

        Ok(())
    }
}
```

---

### Layer 3: ISO 20022 Message Cache
**Purpose**: Cache parsed ISO 20022 messages to avoid re-parsing XML.

**Key Pattern**: `iso20022:{message_type}:{transaction_id}`

**Example**:
```redis
SET iso20022:pacs.008:txn-uuid-123 "{\"debtor\":{\"name\":\"ICICI Bank\"},\"creditor\":{\"name\":\"HDFC Bank\"},\"amount\":\"50000.00\"}"
EXPIRE iso20022:pacs.008:txn-uuid-123 3600
```

**TTL**: 1 hour (3600 seconds)

**When to Invalidate**:
- After transaction is fully processed
- Manual flush for reprocessing

**Rust Implementation**:
```rust
pub struct Iso20022Cache {
    redis: redis::aio::MultiplexedConnection,
}

impl Iso20022Cache {
    pub async fn get_message<T: serde::de::DeserializeOwned>(
        &mut self,
        message_type: &str,
        transaction_id: &str,
    ) -> RedisResult<Option<T>> {
        let key = format!("iso20022:{}:{}", message_type, transaction_id);
        let result: Option<String> = self.redis.get(&key).await?;

        match result {
            Some(json) => Ok(serde_json::from_str(&json).ok()),
            None => Ok(None),
        }
    }

    pub async fn set_message<T: serde::Serialize>(
        &mut self,
        message_type: &str,
        transaction_id: &str,
        message: &T,
    ) -> RedisResult<()> {
        let key = format!("iso20022:{}:{}", message_type, transaction_id);
        let json = serde_json::to_string(message).unwrap();

        self.redis.set_ex(&key, json, 3600).await
    }
}
```

---

## Cache Warming Strategy

### On Clearing Window Open
When a clearing window transitions to "Open" status, pre-warm the cache:

```rust
pub async fn warm_clearing_window_cache(
    window_id: &str,
    db_pool: &PgPool,
    redis: &mut MultiplexedConnection,
) -> Result<()> {
    // 1. Load all pending obligations for this window
    let obligations = sqlx::query_as::<_, Obligation>(
        "SELECT * FROM obligations WHERE clearing_window_id = $1 AND status = 'Pending'"
    )
    .bind(window_id)
    .fetch_all(db_pool)
    .await?;

    // 2. Pre-calculate bilateral net positions
    let mut bilateral_nets = HashMap::new();
    for obl in &obligations {
        let key = (obl.debtor_bank_id, obl.creditor_bank_id, obl.currency.clone());
        bilateral_nets.entry(key).or_insert_with(Vec::new).push(obl);
    }

    // 3. Store in cache
    let mut netting_cache = NettingCache { redis: redis.clone() };
    for ((debtor, creditor, currency), obls) in bilateral_nets {
        let net_pos = calculate_bilateral_net(&obls);
        netting_cache.set_bilateral_net(
            window_id,
            &currency,
            &debtor.to_string(),
            &creditor.to_string(),
            &net_pos,
        ).await?;
    }

    Ok(())
}
```

---

## Cache Invalidation Patterns

### Event-Driven Invalidation
Use NATS JetStream events to trigger cache invalidation across services:

```rust
pub async fn handle_obligation_event(
    event: ObligationEvent,
    redis: &mut MultiplexedConnection,
) -> Result<()> {
    match event.event_type {
        ObligationEventType::Created | ObligationEventType::Updated => {
            // Invalidate net position cache for this window
            let mut netting_cache = NettingCache { redis: redis.clone() };
            netting_cache.invalidate_window_netting(&event.clearing_window_id).await?;
        }
        ObligationEventType::Settled => {
            // Invalidate all caches for this window
            invalidate_all_window_caches(redis, &event.clearing_window_id).await?;
        }
        _ => {}
    }
    Ok(())
}
```

---

## Performance Metrics

### Expected Performance Improvements

| Operation | Without Cache | With Cache | Improvement |
|-----------|---------------|------------|-------------|
| Get Active Window | 15ms | 2ms | 7.5x |
| Bilateral Netting Lookup | 45ms | 3ms | 15x |
| ISO 20022 Parse | 25ms | 1ms | 25x |
| **Total Clearing Cycle** | **500ms** | **100ms** | **5x** |

### Cache Hit Rate Targets

- Active Window Cache: 95%+
- Net Position Cache: 85%+
- ISO 20022 Message Cache: 70%+

---

## Redis Configuration

### redis.conf Optimizations

```conf
# Memory
maxmemory 2gb
maxmemory-policy allkeys-lru

# Persistence (optional for cache)
save ""
appendonly no

# Performance
tcp-backlog 511
timeout 0
tcp-keepalive 300

# Latency monitoring
latency-monitor-threshold 100

# Slow log
slowlog-log-slower-than 10000
slowlog-max-len 128
```

---

## Monitoring and Alerting

### Key Metrics to Monitor

1. **Cache Hit Rate**
   ```redis
   INFO stats | grep keyspace_hits
   INFO stats | grep keyspace_misses
   ```

2. **Memory Usage**
   ```redis
   INFO memory | grep used_memory_human
   ```

3. **Slow Operations**
   ```redis
   SLOWLOG GET 10
   ```

### Alert Thresholds

- Cache hit rate < 70%: Warning
- Cache hit rate < 50%: Critical
- Memory usage > 80%: Warning
- Memory usage > 95%: Critical

---

## Implementation Checklist

- [x] Define cache key patterns
- [x] Implement cache layer structs
- [x] Create cache warming functions
- [x] Implement event-driven invalidation
- [ ] Add cache monitoring to Prometheus
- [ ] Configure Redis in docker-compose
- [ ] Update services to use caching
- [ ] Load test with K6

---

## Next Steps

1. Integrate caching into Obligation Engine service
2. Integrate caching into Clearing Engine service
3. Add Prometheus metrics for cache performance
4. Run performance tests comparing cached vs non-cached operations
5. Tune TTL values based on real-world usage patterns
