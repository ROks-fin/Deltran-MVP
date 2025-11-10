use prometheus::{
    Encoder, IntCounter, IntCounterVec, IntGauge, Histogram, HistogramOpts,
    HistogramVec, Opts, Registry, TextEncoder,
};
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    // HTTP metrics
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("http_requests_total", "Total HTTP requests"),
        &["method", "path", "status"]
    ).expect("metric can be created");

    pub static ref HTTP_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request duration in seconds")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
        &["method", "path"]
    ).expect("metric can be created");

    // Business metrics - Token Engine specific
    pub static ref TOKENS_MINTED: IntCounter = IntCounter::new(
        "tokens_minted_total",
        "Total tokens minted"
    ).expect("metric can be created");

    pub static ref TOKENS_BURNED: IntCounter = IntCounter::new(
        "tokens_burned_total",
        "Total tokens burned"
    ).expect("metric can be created");

    pub static ref TOKENS_TRANSFERRED: IntCounter = IntCounter::new(
        "tokens_transferred_total",
        "Total tokens transferred"
    ).expect("metric can be created");

    pub static ref ACTIVE_TOKENS: IntGauge = IntGauge::new(
        "active_tokens",
        "Number of active tokens in circulation"
    ).expect("metric can be created");

    pub static ref TOKEN_VALUE: Histogram = Histogram::with_opts(
        HistogramOpts::new("token_value_distribution", "Distribution of token values")
            .buckets(vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0, 10000000.0])
    ).expect("metric can be created");

    // Database metrics
    pub static ref DB_QUERIES_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("db_queries_total", "Total database queries"),
        &["operation", "table"]
    ).expect("metric can be created");

    pub static ref DB_QUERY_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("db_query_duration_seconds", "Database query duration in seconds")
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
        &["operation", "table"]
    ).expect("metric can be created");

    // NATS metrics
    pub static ref NATS_MESSAGES_PUBLISHED: IntCounterVec = IntCounterVec::new(
        Opts::new("nats_messages_published_total", "Total NATS messages published"),
        &["subject", "status"]
    ).expect("metric can be created");

    // Redis cache metrics
    pub static ref CACHE_HITS: IntCounter = IntCounter::new(
        "cache_hits_total",
        "Total cache hits"
    ).expect("metric can be created");

    pub static ref CACHE_MISSES: IntCounter = IntCounter::new(
        "cache_misses_total",
        "Total cache misses"
    ).expect("metric can be created");
}

/// Register all metrics with the given registry
pub fn register_metrics(registry: &Registry) -> Result<(), Box<dyn std::error::Error>> {
    // HTTP metrics
    registry.register(Box::new(HTTP_REQUESTS_TOTAL.clone()))?;
    registry.register(Box::new(HTTP_REQUEST_DURATION.clone()))?;

    // Business metrics
    registry.register(Box::new(TOKENS_MINTED.clone()))?;
    registry.register(Box::new(TOKENS_BURNED.clone()))?;
    registry.register(Box::new(TOKENS_TRANSFERRED.clone()))?;
    registry.register(Box::new(ACTIVE_TOKENS.clone()))?;
    registry.register(Box::new(TOKEN_VALUE.clone()))?;

    // Database metrics
    registry.register(Box::new(DB_QUERIES_TOTAL.clone()))?;
    registry.register(Box::new(DB_QUERY_DURATION.clone()))?;

    // NATS metrics
    registry.register(Box::new(NATS_MESSAGES_PUBLISHED.clone()))?;

    // Cache metrics
    registry.register(Box::new(CACHE_HITS.clone()))?;
    registry.register(Box::new(CACHE_MISSES.clone()))?;

    Ok(())
}

/// Generate metrics output in Prometheus text format
pub fn metrics_handler() -> Result<String, Box<dyn std::error::Error>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registration() {
        let registry = Registry::new();
        let result = register_metrics(&registry);
        assert!(result.is_ok());
    }

    #[test]
    fn test_metrics_handler() {
        TOKENS_MINTED.inc();
        let result = metrics_handler();
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("tokens_minted_total"));
    }
}
