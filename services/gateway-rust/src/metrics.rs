// Prometheus Metrics for DelTran Gateway
// Tracks: throughput, latency, errors, payment status transitions

use prometheus::{
    Registry, Counter, Histogram, IntGauge, Opts, HistogramOpts,
    register_counter_with_registry, register_histogram_with_registry,
    register_int_gauge_with_registry, TextEncoder, Encoder,
};
use once_cell::sync::Lazy;
use std::sync::Arc;

pub struct Metrics {
    pub registry: Registry,

    // Request metrics
    pub http_requests_total: Counter,
    pub http_request_duration_seconds: Histogram,
    pub http_requests_in_flight: IntGauge,

    // Payment metrics
    pub payments_total: Counter,
    pub payments_received: Counter,
    pub payments_validated: Counter,
    pub payments_funded: Counter,
    pub payments_completed: Counter,
    pub payments_failed: Counter,
    pub payments_rejected: Counter,

    // ISO message metrics (by type)
    pub iso_messages_total: Counter,
    pub iso_pain001_total: Counter,
    pub iso_pacs008_total: Counter,
    pub iso_camt054_total: Counter,
    pub iso_pacs002_total: Counter,
    pub iso_pain002_total: Counter,
    pub iso_camt053_total: Counter,

    // Processing metrics
    pub payment_processing_duration_seconds: Histogram,
    pub iso_parse_duration_seconds: Histogram,
    pub iso_parse_errors_total: Counter,

    // Database metrics
    pub db_operations_total: Counter,
    pub db_operation_duration_seconds: Histogram,
    pub db_errors_total: Counter,

    // NATS metrics
    pub nats_messages_published_total: Counter,
    pub nats_publish_errors_total: Counter,
    pub nats_publish_duration_seconds: Histogram,

    // Business metrics
    pub total_transaction_volume: Counter,
    pub clearing_batches_total: Counter,
    pub settlement_executions_total: Counter,
}

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();

        // HTTP Request metrics
        let http_requests_total = register_counter_with_registry!(
            Opts::new("deltran_http_requests_total", "Total HTTP requests processed"),
            registry
        )?;

        let http_request_duration_seconds = register_histogram_with_registry!(
            HistogramOpts::new(
                "deltran_http_request_duration_seconds",
                "HTTP request duration in seconds"
            ).buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
            registry
        )?;

        let http_requests_in_flight = register_int_gauge_with_registry!(
            Opts::new("deltran_http_requests_in_flight", "Current HTTP requests being processed"),
            registry
        )?;

        // Payment status metrics
        let payments_total = register_counter_with_registry!(
            Opts::new("deltran_payments_total", "Total payments processed"),
            registry
        )?;

        let payments_received = register_counter_with_registry!(
            Opts::new("deltran_payments_received", "Payments received"),
            registry
        )?;

        let payments_validated = register_counter_with_registry!(
            Opts::new("deltran_payments_validated", "Payments validated"),
            registry
        )?;

        let payments_funded = register_counter_with_registry!(
            Opts::new("deltran_payments_funded", "Payments funded"),
            registry
        )?;

        let payments_completed = register_counter_with_registry!(
            Opts::new("deltran_payments_completed", "Payments completed"),
            registry
        )?;

        let payments_failed = register_counter_with_registry!(
            Opts::new("deltran_payments_failed", "Payments failed"),
            registry
        )?;

        let payments_rejected = register_counter_with_registry!(
            Opts::new("deltran_payments_rejected", "Payments rejected"),
            registry
        )?;

        // ISO message type metrics
        let iso_messages_total = register_counter_with_registry!(
            Opts::new("deltran_iso_messages_total", "Total ISO 20022 messages received"),
            registry
        )?;

        let iso_pain001_total = register_counter_with_registry!(
            Opts::new("deltran_iso_pain001_total", "pain.001 messages received"),
            registry
        )?;

        let iso_pacs008_total = register_counter_with_registry!(
            Opts::new("deltran_iso_pacs008_total", "pacs.008 messages received"),
            registry
        )?;

        let iso_camt054_total = register_counter_with_registry!(
            Opts::new("deltran_iso_camt054_total", "camt.054 messages received (FUNDING)"),
            registry
        )?;

        let iso_pacs002_total = register_counter_with_registry!(
            Opts::new("deltran_iso_pacs002_total", "pacs.002 messages received"),
            registry
        )?;

        let iso_pain002_total = register_counter_with_registry!(
            Opts::new("deltran_iso_pain002_total", "pain.002 messages received"),
            registry
        )?;

        let iso_camt053_total = register_counter_with_registry!(
            Opts::new("deltran_iso_camt053_total", "camt.053 messages received"),
            registry
        )?;

        // Processing duration metrics
        let payment_processing_duration_seconds = register_histogram_with_registry!(
            HistogramOpts::new(
                "deltran_payment_processing_duration_seconds",
                "Payment processing duration in seconds"
            ).buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0]),
            registry
        )?;

        let iso_parse_duration_seconds = register_histogram_with_registry!(
            HistogramOpts::new(
                "deltran_iso_parse_duration_seconds",
                "ISO message parsing duration in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
            registry
        )?;

        let iso_parse_errors_total = register_counter_with_registry!(
            Opts::new("deltran_iso_parse_errors_total", "Total ISO parsing errors"),
            registry
        )?;

        // Database metrics
        let db_operations_total = register_counter_with_registry!(
            Opts::new("deltran_db_operations_total", "Total database operations"),
            registry
        )?;

        let db_operation_duration_seconds = register_histogram_with_registry!(
            HistogramOpts::new(
                "deltran_db_operation_duration_seconds",
                "Database operation duration in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
            registry
        )?;

        let db_errors_total = register_counter_with_registry!(
            Opts::new("deltran_db_errors_total", "Total database errors"),
            registry
        )?;

        // NATS metrics
        let nats_messages_published_total = register_counter_with_registry!(
            Opts::new("deltran_nats_messages_published_total", "Total NATS messages published"),
            registry
        )?;

        let nats_publish_errors_total = register_counter_with_registry!(
            Opts::new("deltran_nats_publish_errors_total", "Total NATS publish errors"),
            registry
        )?;

        let nats_publish_duration_seconds = register_histogram_with_registry!(
            HistogramOpts::new(
                "deltran_nats_publish_duration_seconds",
                "NATS publish duration in seconds"
            ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1]),
            registry
        )?;

        // Business metrics
        let total_transaction_volume = register_counter_with_registry!(
            Opts::new("deltran_total_transaction_volume", "Total transaction volume in cents"),
            registry
        )?;

        let clearing_batches_total = register_counter_with_registry!(
            Opts::new("deltran_clearing_batches_total", "Total clearing batches"),
            registry
        )?;

        let settlement_executions_total = register_counter_with_registry!(
            Opts::new("deltran_settlement_executions_total", "Total settlement executions"),
            registry
        )?;

        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            http_requests_in_flight,
            payments_total,
            payments_received,
            payments_validated,
            payments_funded,
            payments_completed,
            payments_failed,
            payments_rejected,
            iso_messages_total,
            iso_pain001_total,
            iso_pacs008_total,
            iso_camt054_total,
            iso_pacs002_total,
            iso_pain002_total,
            iso_camt053_total,
            payment_processing_duration_seconds,
            iso_parse_duration_seconds,
            iso_parse_errors_total,
            db_operations_total,
            db_operation_duration_seconds,
            db_errors_total,
            nats_messages_published_total,
            nats_publish_errors_total,
            nats_publish_duration_seconds,
            total_transaction_volume,
            clearing_batches_total,
            settlement_executions_total,
        })
    }

    /// Export all metrics in Prometheus text format
    pub fn export(&self) -> anyhow::Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    /// Track ISO message by type
    pub fn track_iso_message(&self, message_type: &str) {
        self.iso_messages_total.inc();
        match message_type {
            "pain.001" => self.iso_pain001_total.inc(),
            "pacs.008" => self.iso_pacs008_total.inc(),
            "camt.054" => self.iso_camt054_total.inc(),
            "pacs.002" => self.iso_pacs002_total.inc(),
            "pain.002" => self.iso_pain002_total.inc(),
            "camt.053" => self.iso_camt053_total.inc(),
            _ => {}
        }
    }
}

// Global metrics instance
pub static METRICS: Lazy<Arc<Metrics>> = Lazy::new(|| {
    Arc::new(Metrics::new().expect("Failed to initialize metrics"))
});
