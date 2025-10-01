//! Partitioning strategies for message routing

use crate::types::PartitionKey;

/// Partitioning strategy
pub trait PartitioningStrategy: Send + Sync {
    /// Compute partition number for given key
    fn partition(&self, key: &PartitionKey) -> u32;

    /// Total number of partitions
    fn num_partitions(&self) -> u32;
}

/// Hash-based partitioning (default)
#[derive(Debug, Clone)]
pub struct HashPartitioning {
    num_partitions: u32,
}

impl HashPartitioning {
    /// Create new hash-based partitioning with given partition count
    pub fn new(num_partitions: u32) -> Self {
        assert!(num_partitions > 0, "num_partitions must be > 0");
        Self { num_partitions }
    }
}

impl Default for HashPartitioning {
    fn default() -> Self {
        Self::new(32) // 32 partitions by default
    }
}

impl PartitioningStrategy for HashPartitioning {
    fn partition(&self, key: &PartitionKey) -> u32 {
        key.partition_number(self.num_partitions)
    }

    fn num_partitions(&self) -> u32 {
        self.num_partitions
    }
}

/// Round-robin partitioning
#[derive(Debug)]
pub struct RoundRobinPartitioning {
    num_partitions: u32,
    counter: std::sync::atomic::AtomicU32,
}

impl RoundRobinPartitioning {
    /// Create new round-robin partitioning
    pub fn new(num_partitions: u32) -> Self {
        assert!(num_partitions > 0, "num_partitions must be > 0");
        Self {
            num_partitions,
            counter: std::sync::atomic::AtomicU32::new(0),
        }
    }
}

impl PartitioningStrategy for RoundRobinPartitioning {
    fn partition(&self, _key: &PartitionKey) -> u32 {
        let current = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        current % self.num_partitions
    }

    fn num_partitions(&self) -> u32 {
        self.num_partitions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_partitioning() {
        let strategy = HashPartitioning::new(8);
        let key = PartitionKey::Corridor("USD-EUR".to_string());

        let p1 = strategy.partition(&key);
        let p2 = strategy.partition(&key);

        assert_eq!(p1, p2); // Same key -> same partition
        assert!(p1 < 8);
    }

    #[test]
    fn test_round_robin_partitioning() {
        let strategy = RoundRobinPartitioning::new(4);
        let key = PartitionKey::Corridor("USD-EUR".to_string());

        let p1 = strategy.partition(&key);
        let p2 = strategy.partition(&key);
        let p3 = strategy.partition(&key);
        let p4 = strategy.partition(&key);
        let p5 = strategy.partition(&key);

        assert_eq!(p1, 0);
        assert_eq!(p2, 1);
        assert_eq!(p3, 2);
        assert_eq!(p4, 3);
        assert_eq!(p5, 0); // Wraps around
    }
}
