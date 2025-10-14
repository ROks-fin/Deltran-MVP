//! Sharding coordinator for horizontal scalability
//!
//! Implements consistent hashing for distributing load across multiple nodes.
//! Target: 5000+ TPS with horizontal scaling.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Shard identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ShardId(pub u16);

impl ShardId {
    pub fn new(id: u16) -> Self {
        Self(id)
    }

    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

/// Node in the shard ring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardNode {
    pub shard_id: ShardId,
    pub node_id: String,
    pub address: String,
    pub weight: u32,
    pub healthy: bool,
}

impl ShardNode {
    pub fn new(shard_id: ShardId, node_id: String, address: String, weight: u32) -> Self {
        Self {
            shard_id,
            node_id,
            address,
            weight,
            healthy: true,
        }
    }
}

/// Consistent hash ring for shard distribution
pub struct ConsistentHashRing {
    /// Virtual nodes on the ring (hash -> ShardNode)
    ring: BTreeMap<u64, Arc<ShardNode>>,

    /// Number of virtual nodes per physical node
    virtual_nodes: usize,

    /// All registered nodes
    nodes: Vec<Arc<ShardNode>>,
}

impl ConsistentHashRing {
    /// Create new consistent hash ring
    pub fn new(virtual_nodes: usize) -> Self {
        Self {
            ring: BTreeMap::new(),
            virtual_nodes,
            nodes: Vec::new(),
        }
    }

    /// Add node to the ring
    pub fn add_node(&mut self, node: ShardNode) {
        let node_arc = Arc::new(node);

        // Add virtual nodes to the ring
        for i in 0..self.virtual_nodes {
            let vnode_key = format!("{}:{}", node_arc.node_id, i);
            let hash = self.hash_key(&vnode_key);
            self.ring.insert(hash, Arc::clone(&node_arc));
        }

        self.nodes.push(node_arc);
        info!(
            "Added shard node {} (shard_id: {}) with {} virtual nodes",
            self.nodes.last().unwrap().node_id,
            self.nodes.last().unwrap().shard_id.as_u16(),
            self.virtual_nodes
        );
    }

    /// Remove node from the ring
    pub fn remove_node(&mut self, node_id: &str) -> bool {
        // Remove virtual nodes
        for i in 0..self.virtual_nodes {
            let vnode_key = format!("{}:{}", node_id, i);
            let hash = self.hash_key(&vnode_key);
            self.ring.remove(&hash);
        }

        // Remove from nodes list
        let original_len = self.nodes.len();
        self.nodes.retain(|n| n.node_id != node_id);

        if self.nodes.len() < original_len {
            info!("Removed shard node {}", node_id);
            true
        } else {
            false
        }
    }

    /// Get shard for a key (consistent hashing)
    pub fn get_shard(&self, key: &str) -> Option<Arc<ShardNode>> {
        if self.ring.is_empty() {
            return None;
        }

        let hash = self.hash_key(key);

        // Find first node with hash >= key hash
        for (node_hash, node) in self.ring.range(hash..) {
            if node.healthy {
                return Some(Arc::clone(node));
            }
        }

        // Wrap around to first node
        self.ring.values().find(|n| n.healthy).map(Arc::clone)
    }

    /// Get all healthy nodes
    pub fn get_healthy_nodes(&self) -> Vec<Arc<ShardNode>> {
        self.nodes.iter().filter(|n| n.healthy).cloned().collect()
    }

    /// Get total node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Hash function (FNV-1a)
    fn hash_key(&self, key: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

/// Sharding coordinator manages shard topology
pub struct ShardingCoordinator {
    /// Consistent hash ring
    ring: Arc<RwLock<ConsistentHashRing>>,

    /// Number of shards
    num_shards: u16,

    /// Replication factor
    replication_factor: usize,
}

impl ShardingCoordinator {
    /// Create new sharding coordinator
    pub fn new(num_shards: u16, replication_factor: usize) -> Self {
        let virtual_nodes = 150; // Good balance between distribution and memory

        Self {
            ring: Arc::new(RwLock::new(ConsistentHashRing::new(virtual_nodes))),
            num_shards,
            replication_factor,
        }
    }

    /// Register a new shard node
    pub async fn register_node(&self, node: ShardNode) {
        let mut ring = self.ring.write().await;
        ring.add_node(node);
    }

    /// Unregister a shard node
    pub async fn unregister_node(&self, node_id: &str) -> bool {
        let mut ring = self.ring.write().await;
        ring.remove_node(node_id)
    }

    /// Get shard for a payment ID (or any key)
    pub async fn get_shard_for_key(&self, key: &str) -> Option<ShardId> {
        let ring = self.ring.read().await;
        ring.get_shard(key).map(|node| node.shard_id)
    }

    /// Get node for a payment ID (with address)
    pub async fn get_node_for_key(&self, key: &str) -> Option<Arc<ShardNode>> {
        let ring = self.ring.read().await;
        ring.get_shard(key)
    }

    /// Get replicas for a key (for replication)
    pub async fn get_replicas_for_key(&self, key: &str) -> Vec<Arc<ShardNode>> {
        let ring = self.ring.read().await;
        let hash = ring.hash_key(key);

        let mut replicas = Vec::new();
        let mut seen_shards = std::collections::HashSet::new();

        // Find primary and replicas
        for (_, node) in ring.ring.range(hash..) {
            if node.healthy && !seen_shards.contains(&node.shard_id) {
                replicas.push(Arc::clone(node));
                seen_shards.insert(node.shard_id);

                if replicas.len() >= self.replication_factor {
                    break;
                }
            }
        }

        // Wrap around if needed
        if replicas.len() < self.replication_factor {
            for (_, node) in &ring.ring {
                if node.healthy && !seen_shards.contains(&node.shard_id) {
                    replicas.push(Arc::clone(node));
                    seen_shards.insert(node.shard_id);

                    if replicas.len() >= self.replication_factor {
                        break;
                    }
                }
            }
        }

        replicas
    }

    /// Get cluster status
    pub async fn get_cluster_status(&self) -> ClusterStatus {
        let ring = self.ring.read().await;
        let total_nodes = ring.node_count();
        let healthy_nodes = ring.get_healthy_nodes().len();

        ClusterStatus {
            total_shards: self.num_shards,
            total_nodes,
            healthy_nodes,
            replication_factor: self.replication_factor,
            cluster_healthy: healthy_nodes >= self.replication_factor,
        }
    }

    /// Mark node as unhealthy
    pub async fn mark_node_unhealthy(&self, node_id: &str) {
        let mut ring = self.ring.write().await;
        for node in &mut ring.nodes {
            if node.node_id == node_id {
                Arc::get_mut(node).map(|n| n.healthy = false);
                warn!("Marked node {} as unhealthy", node_id);
                break;
            }
        }
    }

    /// Mark node as healthy
    pub async fn mark_node_healthy(&self, node_id: &str) {
        let mut ring = self.ring.write().await;
        for node in &mut ring.nodes {
            if node.node_id == node_id {
                Arc::get_mut(node).map(|n| n.healthy = true);
                info!("Marked node {} as healthy", node_id);
                break;
            }
        }
    }
}

/// Cluster health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    pub total_shards: u16,
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub replication_factor: usize,
    pub cluster_healthy: bool,
}

/// Shard-aware key generator
pub trait ShardKey {
    fn shard_key(&self) -> String;
}

// Example implementation for payment IDs
impl ShardKey for uuid::Uuid {
    fn shard_key(&self) -> String {
        self.to_string()
    }
}

impl ShardKey for String {
    fn shard_key(&self) -> String {
        self.clone()
    }
}

impl ShardKey for &str {
    fn shard_key(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consistent_hash_ring() {
        let mut ring = ConsistentHashRing::new(100);

        let node1 = ShardNode::new(ShardId(1), "node1".to_string(), "10.0.0.1:8080".to_string(), 1);
        let node2 = ShardNode::new(ShardId(2), "node2".to_string(), "10.0.0.2:8080".to_string(), 1);

        ring.add_node(node1);
        ring.add_node(node2);

        assert_eq!(ring.node_count(), 2);

        // Same key should always map to same node
        let key = "test-payment-123";
        let shard1 = ring.get_shard(key).unwrap();
        let shard2 = ring.get_shard(key).unwrap();
        assert_eq!(shard1.node_id, shard2.node_id);
    }

    #[test]
    fn test_node_removal() {
        let mut ring = ConsistentHashRing::new(100);

        let node1 = ShardNode::new(ShardId(1), "node1".to_string(), "10.0.0.1:8080".to_string(), 1);
        let node2 = ShardNode::new(ShardId(2), "node2".to_string(), "10.0.0.2:8080".to_string(), 1);

        ring.add_node(node1);
        ring.add_node(node2);

        assert_eq!(ring.node_count(), 2);

        ring.remove_node("node1");
        assert_eq!(ring.node_count(), 1);

        let remaining = ring.get_healthy_nodes();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].node_id, "node2");
    }

    #[tokio::test]
    async fn test_sharding_coordinator() {
        let coordinator = ShardingCoordinator::new(4, 3);

        // Register nodes
        for i in 1..=4 {
            let node = ShardNode::new(
                ShardId(i),
                format!("node{}", i),
                format!("10.0.0.{}:8080", i),
                1,
            );
            coordinator.register_node(node).await;
        }

        let status = coordinator.get_cluster_status().await;
        assert_eq!(status.total_nodes, 4);
        assert_eq!(status.healthy_nodes, 4);
        assert!(status.cluster_healthy);

        // Get shard for key
        let key = "payment-12345";
        let shard = coordinator.get_shard_for_key(key).await.unwrap();
        assert!(shard.as_u16() >= 1 && shard.as_u16() <= 4);

        // Same key should always map to same shard
        let shard2 = coordinator.get_shard_for_key(key).await.unwrap();
        assert_eq!(shard.as_u16(), shard2.as_u16());
    }

    #[tokio::test]
    async fn test_replication() {
        let coordinator = ShardingCoordinator::new(3, 2);

        for i in 1..=3 {
            let node = ShardNode::new(
                ShardId(i),
                format!("node{}", i),
                format!("10.0.0.{}:8080", i),
                1,
            );
            coordinator.register_node(node).await;
        }

        let key = "payment-67890";
        let replicas = coordinator.get_replicas_for_key(key).await;

        assert_eq!(replicas.len(), 2);
        assert_ne!(replicas[0].shard_id, replicas[1].shard_id);
    }

    #[tokio::test]
    async fn test_node_health() {
        let coordinator = ShardingCoordinator::new(2, 1);

        let node1 = ShardNode::new(ShardId(1), "node1".to_string(), "10.0.0.1:8080".to_string(), 1);
        let node2 = ShardNode::new(ShardId(2), "node2".to_string(), "10.0.0.2:8080".to_string(), 1);

        coordinator.register_node(node1).await;
        coordinator.register_node(node2).await;

        coordinator.mark_node_unhealthy("node1").await;

        let status = coordinator.get_cluster_status().await;
        assert_eq!(status.total_nodes, 2);
        assert_eq!(status.healthy_nodes, 1);
    }
}
