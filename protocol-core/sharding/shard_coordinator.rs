// sharding/shard_coordinator.rs
// Corridor-based sharding coordinator for 10k TPS scalability

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

pub type ShardId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub payment_id: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub debtor_country: String,
    pub creditor_country: String,
    pub debtor_account: String,
    pub creditor_account: String,
}

#[derive(Debug, Clone)]
pub struct ShardInfo {
    pub shard_id: ShardId,
    pub corridor: String,
    pub currency_pairs: Vec<String>,
    pub ledger_endpoint: String,
    pub consensus_endpoints: Vec<String>,
    pub db_pool: PgPool,
    pub status: ShardStatus,
    pub capacity_tps: u32,
    pub current_tps: Arc<RwLock<u32>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShardStatus {
    Active,
    ReadOnly,
    Offline,
    Draining,
}

#[derive(Debug, Clone)]
pub struct RoutingTable {
    shard_map: HashMap<String, ShardId>,
    consistent_hash_ring: ConsistentHashRing,
}

pub struct ConsistentHashRing {
    nodes: Vec<(u64, ShardId)>, // (hash, shard_id)
    replicas: usize,
}

impl ConsistentHashRing {
    pub fn new(shards: &[ShardId], replicas: usize) -> Self {
        let mut nodes = Vec::new();

        for &shard_id in shards {
            for i in 0..replicas {
                let key = format!("shard-{}-replica-{}", shard_id, i);
                let hash = Self::hash_key(&key);
                nodes.push((hash, shard_id));
            }
        }

        nodes.sort_by_key(|&(hash, _)| hash);

        Self { nodes, replicas }
    }

    pub fn get_shard(&self, key: &str) -> ShardId {
        if self.nodes.is_empty() {
            return 0;
        }

        let hash = Self::hash_key(key);

        // Binary search for first node >= hash
        match self.nodes.binary_search_by_key(&hash, |&(h, _)| h) {
            Ok(idx) => self.nodes[idx].1,
            Err(idx) => {
                if idx == self.nodes.len() {
                    self.nodes[0].1 // Wrap around
                } else {
                    self.nodes[idx].1
                }
            }
        }
    }

    fn hash_key(key: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    pub fn add_shard(&mut self, shard_id: ShardId) {
        for i in 0..self.replicas {
            let key = format!("shard-{}-replica-{}", shard_id, i);
            let hash = Self::hash_key(&key);
            self.nodes.push((hash, shard_id));
        }
        self.nodes.sort_by_key(|&(hash, _)| hash);
    }

    pub fn remove_shard(&mut self, shard_id: ShardId) {
        self.nodes.retain(|&(_, id)| id != shard_id);
    }
}

pub struct ShardCoordinator {
    shards: Arc<RwLock<HashMap<ShardId, ShardInfo>>>,
    routing_table: Arc<RwLock<RoutingTable>>,
    num_shards: u32,
}

impl ShardCoordinator {
    pub fn new(num_shards: u32) -> Self {
        let shard_ids: Vec<ShardId> = (0..num_shards).collect();
        let consistent_hash_ring = ConsistentHashRing::new(&shard_ids, 150); // 150 virtual nodes

        let routing_table = RoutingTable {
            shard_map: HashMap::new(),
            consistent_hash_ring,
        };

        Self {
            shards: Arc::new(RwLock::new(HashMap::new())),
            routing_table: Arc::new(RwLock::new(routing_table)),
            num_shards,
        }
    }

    /// Register a shard
    pub async fn register_shard(&self, shard_info: ShardInfo) {
        let shard_id = shard_info.shard_id;
        self.shards.write().await.insert(shard_id, shard_info.clone());

        // Update routing table
        let mut routing = self.routing_table.write().await;

        // Add corridor mappings
        for corridor in self.get_corridors(&shard_info) {
            routing.shard_map.insert(corridor, shard_id);
        }

        info!("Registered shard {} with corridor: {}", shard_id, shard_info.corridor);
    }

    /// Route payment to appropriate shard
    pub async fn route_payment(&self, payment: &Payment) -> ShardId {
        let shard_key = self.compute_shard_key(payment);
        let routing = self.routing_table.read().await;

        // Try direct mapping first
        if let Some(&shard_id) = routing.shard_map.get(&shard_key) {
            return shard_id;
        }

        // Fall back to consistent hashing
        routing.consistent_hash_ring.get_shard(&shard_key)
    }

    /// Compute shard key from payment
    fn compute_shard_key(&self, payment: &Payment) -> String {
        // Primary key: corridor (country pair + currency)
        let corridor = format!(
            "{}->{}/{}",
            payment.debtor_country,
            payment.creditor_country,
            payment.currency
        );

        corridor
    }

    /// Get corridors for a shard
    fn get_corridors(&self, shard_info: &ShardInfo) -> Vec<String> {
        // Parse corridor string
        // Example: "AED->USD,AED->EUR" or "UAE_OUTBOUND"
        let mut corridors = Vec::new();

        if shard_info.corridor.contains("->") {
            // Explicit corridor list
            for pair in shard_info.corridor.split(',') {
                corridors.push(pair.trim().to_string());
            }
        } else {
            // Named corridor - expand to currency pairs
            match shard_info.corridor.as_str() {
                "UAE_OUTBOUND" => {
                    for currency_pair in &shard_info.currency_pairs {
                        corridors.push(format!("AE->{}", currency_pair));
                    }
                }
                "UAE_INBOUND" => {
                    for currency_pair in &shard_info.currency_pairs {
                        corridors.push(format!("{}->AE", currency_pair));
                    }
                }
                "INDIA_CORRIDOR" => {
                    corridors.push("AE->IN/INR".to_string());
                    corridors.push("IN->AE/AED".to_string());
                }
                "G10_MAJOR" => {
                    corridors.push("US->EU/EUR".to_string());
                    corridors.push("EU->US/USD".to_string());
                    corridors.push("GB->US/USD".to_string());
                }
                _ => {
                    // Default: use corridor as-is
                    corridors.push(shard_info.corridor.clone());
                }
            }
        }

        corridors
    }

    /// Execute payment on shard
    pub async fn execute_payment(
        &self,
        payment: Payment,
    ) -> Result<PaymentResult, ShardError> {
        let shard_id = self.route_payment(&payment).await;

        let shards = self.shards.read().await;
        let shard = shards
            .get(&shard_id)
            .ok_or(ShardError::ShardNotFound(shard_id))?;

        // Check shard status
        if shard.status != ShardStatus::Active {
            return Err(ShardError::ShardUnavailable(shard_id));
        }

        // Check capacity
        let current_tps = *shard.current_tps.read().await;
        if current_tps >= shard.capacity_tps {
            warn!("Shard {} at capacity: {} TPS", shard_id, current_tps);
            return Err(ShardError::ShardOverloaded(shard_id));
        }

        info!(
            "Routing payment {} to shard {} (corridor: {})",
            payment.payment_id, shard_id, shard.corridor
        );

        // Execute on shard (call ledger-core)
        self.execute_on_shard(shard, payment).await
    }

    /// Execute payment on specific shard
    async fn execute_on_shard(
        &self,
        shard: &ShardInfo,
        payment: Payment,
    ) -> Result<PaymentResult, ShardError> {
        // In production: gRPC call to ledger-core
        // For now: database write

        let payment_id = payment.payment_id;

        // Write to shard database
        sqlx::query(
            r#"
            INSERT INTO payments
            (payment_id, amount, currency, debtor_country, creditor_country, debtor_account, creditor_account, shard_id, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending')
            "#
        )
        .bind(payment_id)
        .bind(payment.amount)
        .bind(&payment.currency)
        .bind(&payment.debtor_country)
        .bind(&payment.creditor_country)
        .bind(&payment.debtor_account)
        .bind(&payment.creditor_account)
        .bind(shard.shard_id as i32)
        .execute(&shard.db_pool)
        .await
        .map_err(|e| ShardError::DatabaseError(e.to_string()))?;

        // Increment TPS counter
        let mut current_tps = shard.current_tps.write().await;
        *current_tps += 1;

        Ok(PaymentResult {
            payment_id,
            shard_id: shard.shard_id,
            status: PaymentStatus::Accepted,
        })
    }

    /// Execute cross-shard payment (2PC)
    pub async fn execute_cross_shard_payment(
        &self,
        payment: Payment,
        shard_ids: Vec<ShardId>,
    ) -> Result<PaymentResult, ShardError> {
        let txn_id = Uuid::new_v4();

        info!(
            "Executing cross-shard payment {} across shards: {:?}",
            payment.payment_id, shard_ids
        );

        // Phase 1: Prepare on all shards
        let mut prepared_shards = Vec::new();

        for &shard_id in &shard_ids {
            match self.prepare_on_shard(shard_id, &payment, txn_id).await {
                Ok(_) => prepared_shards.push(shard_id),
                Err(e) => {
                    error!("Prepare failed on shard {}: {:?}", shard_id, e);

                    // Abort all prepared shards
                    for &abort_shard in &prepared_shards {
                        if let Err(abort_err) = self.abort_on_shard(abort_shard, txn_id).await {
                            error!("Abort failed on shard {}: {:?}", abort_shard, abort_err);
                        }
                    }

                    return Err(e);
                }
            }
        }

        // Phase 2: Commit on all shards
        for &shard_id in &shard_ids {
            if let Err(e) = self.commit_on_shard(shard_id, txn_id).await {
                error!("Commit failed on shard {}: {:?}", shard_id, e);

                // Log inconsistency - requires manual intervention
                error!("INCONSISTENT STATE: Transaction {} partially committed", txn_id);

                return Err(e);
            }
        }

        info!("Cross-shard payment {} committed on all shards", payment.payment_id);

        Ok(PaymentResult {
            payment_id: payment.payment_id,
            shard_id: shard_ids[0], // Primary shard
            status: PaymentStatus::Accepted,
        })
    }

    /// Prepare phase of 2PC
    async fn prepare_on_shard(
        &self,
        shard_id: ShardId,
        payment: &Payment,
        txn_id: Uuid,
    ) -> Result<(), ShardError> {
        let shards = self.shards.read().await;
        let shard = shards
            .get(&shard_id)
            .ok_or(ShardError::ShardNotFound(shard_id))?;

        // Write to prepared_transactions table
        sqlx::query(
            r#"
            INSERT INTO prepared_transactions
            (txn_id, shard_id, payment_id, payment_data, state, created_at)
            VALUES ($1, $2, $3, $4, 'prepared', NOW())
            "#
        )
        .bind(txn_id)
        .bind(shard_id as i32)
        .bind(payment.payment_id)
        .bind(serde_json::to_value(payment).unwrap())
        .execute(&shard.db_pool)
        .await
        .map_err(|e| ShardError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Commit phase of 2PC
    async fn commit_on_shard(
        &self,
        shard_id: ShardId,
        txn_id: Uuid,
    ) -> Result<(), ShardError> {
        let shards = self.shards.read().await;
        let shard = shards
            .get(&shard_id)
            .ok_or(ShardError::ShardNotFound(shard_id))?;

        // Fetch prepared transaction
        let row = sqlx::query(
            r#"
            SELECT payment_id, payment_data
            FROM prepared_transactions
            WHERE txn_id = $1 AND shard_id = $2 AND state = 'prepared'
            "#
        )
        .bind(txn_id)
        .bind(shard_id as i32)
        .fetch_one(&shard.db_pool)
        .await
        .map_err(|e| ShardError::DatabaseError(e.to_string()))?;

        let payment_id: Uuid = row.get("payment_id");
        let payment_data: serde_json::Value = row.get("payment_data");
        let payment: Payment = serde_json::from_value(payment_data).unwrap();

        // Execute payment
        self.execute_on_shard(shard, payment).await?;

        // Mark as committed
        sqlx::query(
            r#"
            UPDATE prepared_transactions
            SET state = 'committed', committed_at = NOW()
            WHERE txn_id = $1 AND shard_id = $2
            "#
        )
        .bind(txn_id)
        .bind(shard_id as i32)
        .execute(&shard.db_pool)
        .await
        .map_err(|e| ShardError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Abort phase of 2PC
    async fn abort_on_shard(
        &self,
        shard_id: ShardId,
        txn_id: Uuid,
    ) -> Result<(), ShardError> {
        let shards = self.shards.read().await;
        let shard = shards
            .get(&shard_id)
            .ok_or(ShardError::ShardNotFound(shard_id))?;

        sqlx::query(
            r#"
            UPDATE prepared_transactions
            SET state = 'aborted', aborted_at = NOW()
            WHERE txn_id = $1 AND shard_id = $2
            "#
        )
        .bind(txn_id)
        .bind(shard_id as i32)
        .execute(&shard.db_pool)
        .await
        .map_err(|e| ShardError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get shard statistics
    pub async fn get_shard_stats(&self) -> Vec<ShardStats> {
        let shards = self.shards.read().await;

        let mut stats = Vec::new();
        for (shard_id, shard) in shards.iter() {
            let current_tps = *shard.current_tps.read().await;

            stats.push(ShardStats {
                shard_id: *shard_id,
                corridor: shard.corridor.clone(),
                status: shard.status.clone(),
                capacity_tps: shard.capacity_tps,
                current_tps,
                utilization: (current_tps as f32 / shard.capacity_tps as f32) * 100.0,
            });
        }

        stats
    }

    /// Rebalance shards (future)
    pub async fn rebalance_shards(&self) {
        // TODO: Implement shard rebalancing
        // - Detect hot shards
        // - Migrate corridors to new shards
        // - Use consistent hashing for smooth migration
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResult {
    pub payment_id: Uuid,
    pub shard_id: ShardId,
    pub status: PaymentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentStatus {
    Accepted,
    Rejected,
    Pending,
}

#[derive(Debug, Clone)]
pub struct ShardStats {
    pub shard_id: ShardId,
    pub corridor: String,
    pub status: ShardStatus,
    pub capacity_tps: u32,
    pub current_tps: u32,
    pub utilization: f32,
}

#[derive(Debug)]
pub enum ShardError {
    ShardNotFound(ShardId),
    ShardUnavailable(ShardId),
    ShardOverloaded(ShardId),
    DatabaseError(String),
    CrossShardError(String),
}

impl std::fmt::Display for ShardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardError::ShardNotFound(id) => write!(f, "Shard {} not found", id),
            ShardError::ShardUnavailable(id) => write!(f, "Shard {} unavailable", id),
            ShardError::ShardOverloaded(id) => write!(f, "Shard {} overloaded", id),
            ShardError::DatabaseError(e) => write!(f, "Database error: {}", e),
            ShardError::CrossShardError(e) => write!(f, "Cross-shard error: {}", e),
        }
    }
}

impl std::error::Error for ShardError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hashing() {
        let shards = vec![0, 1, 2, 3, 4];
        let ring = ConsistentHashRing::new(&shards, 150);

        // Same key should always map to same shard
        let shard1 = ring.get_shard("AE->US/USD");
        let shard2 = ring.get_shard("AE->US/USD");
        assert_eq!(shard1, shard2);

        // Different keys should distribute
        let mut distribution = HashMap::new();
        for i in 0..1000 {
            let key = format!("payment-{}", i);
            let shard = ring.get_shard(&key);
            *distribution.entry(shard).or_insert(0) += 1;
        }

        // Check reasonable distribution (within 50% of ideal)
        let ideal = 1000 / shards.len();
        for count in distribution.values() {
            let deviation = (*count as f32 - ideal as f32).abs() / ideal as f32;
            assert!(deviation < 0.5, "Distribution deviation too high: {}", deviation);
        }
    }

    #[test]
    fn test_shard_key_computation() {
        let coordinator = ShardCoordinator::new(5);

        let payment = Payment {
            payment_id: Uuid::new_v4(),
            amount: Decimal::new(1000, 0),
            currency: "USD".to_string(),
            debtor_country: "AE".to_string(),
            creditor_country: "US".to_string(),
            debtor_account: "ACC001".to_string(),
            creditor_account: "ACC002".to_string(),
        };

        let key = coordinator.compute_shard_key(&payment);
        assert_eq!(key, "AE->US/USD");
    }
}
