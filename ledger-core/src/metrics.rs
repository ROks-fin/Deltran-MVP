//! Metrics collection for observability
//!
//! This module provides Prometheus metrics for monitoring the ledger.
//!
//! # Metrics
//!
//! - `ledger_events_total` - Total number of events appended
//! - `ledger_events_batch_size` - Histogram of batch sizes
//! - `ledger_append_duration_seconds` - Histogram of append latencies
//! - `ledger_blocks_total` - Total number of finalized blocks
//! - `ledger_storage_size_bytes` - Storage size estimate

use prometheus::{
    register_counter, register_histogram, register_int_counter, register_int_gauge, Counter,
    Histogram, IntCounter, IntGauge, Registry,
};
use std::sync::Arc;

/// Metrics collector
#[derive(Clone)]
pub struct Metrics {
    /// Total events appended
    pub events_total: IntCounter,

    /// Batch size histogram
    pub batch_size: Histogram,

    /// Append duration histogram
    pub append_duration: Histogram,

    /// Total blocks finalized
    pub blocks_total: IntCounter,

    /// Storage size estimate
    pub storage_size: IntGauge,

    /// Prometheus registry
    pub registry: Arc<Registry>,
}

impl Metrics {
    /// Create new metrics collector
    pub fn new() -> prometheus::Result<Self> {
        let registry = Arc::new(Registry::new());

        let events_total = register_int_counter!(
            "ledger_events_total",
            "Total number of events appended"
        )?;
        registry.register(Box::new(events_total.clone()))?;

        let batch_size = register_histogram!(
            "ledger_events_batch_size",
            "Histogram of batch sizes",
            vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0]
        )?;
        registry.register(Box::new(batch_size.clone()))?;

        let append_duration = register_histogram!(
            "ledger_append_duration_seconds",
            "Histogram of append latencies",
            vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0]
        )?;
        registry.register(Box::new(append_duration.clone()))?;

        let blocks_total = register_int_counter!(
            "ledger_blocks_total",
            "Total number of finalized blocks"
        )?;
        registry.register(Box::new(blocks_total.clone()))?;

        let storage_size = register_int_gauge!(
            "ledger_storage_size_bytes",
            "Storage size estimate"
        )?;
        registry.register(Box::new(storage_size.clone()))?;

        Ok(Self {
            events_total,
            batch_size,
            append_duration,
            blocks_total,
            storage_size,
            registry,
        })
    }

    /// Record event append
    pub fn record_event_append(&self) {
        self.events_total.inc();
    }

    /// Record batch flush
    pub fn record_batch_flush(&self, batch_size: usize) {
        self.batch_size.observe(batch_size as f64);
    }

    /// Record append duration
    pub fn record_append_duration(&self, duration_seconds: f64) {
        self.append_duration.observe(duration_seconds);
    }

    /// Record block finalization
    pub fn record_block_finalized(&self) {
        self.blocks_total.inc();
    }

    /// Update storage size estimate
    pub fn update_storage_size(&self, size_bytes: i64) {
        self.storage_size.set(size_bytes);
    }

    /// Get metrics registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new().unwrap();
        assert_eq!(metrics.events_total.get(), 0);
        assert_eq!(metrics.blocks_total.get(), 0);
    }

    #[test]
    fn test_record_event_append() {
        let metrics = Metrics::new().unwrap();
        metrics.record_event_append();
        assert_eq!(metrics.events_total.get(), 1);

        metrics.record_event_append();
        assert_eq!(metrics.events_total.get(), 2);
    }

    #[test]
    fn test_record_batch_flush() {
        let metrics = Metrics::new().unwrap();
        metrics.record_batch_flush(10);
        metrics.record_batch_flush(25);
        metrics.record_batch_flush(50);
        // Histogram recorded successfully (no assertion on histogram internals)
    }

    #[test]
    fn test_record_block_finalized() {
        let metrics = Metrics::new().unwrap();
        metrics.record_block_finalized();
        assert_eq!(metrics.blocks_total.get(), 1);
    }

    #[test]
    fn test_update_storage_size() {
        let metrics = Metrics::new().unwrap();
        metrics.update_storage_size(1024 * 1024 * 100); // 100 MB
        assert_eq!(metrics.storage_size.get(), 1024 * 1024 * 100);
    }
}