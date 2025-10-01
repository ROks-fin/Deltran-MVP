//! Adapter metrics

use prometheus::{
    register_counter_vec, register_histogram_vec, register_int_gauge_vec, CounterVec,
    HistogramVec, IntGaugeVec,
};

lazy_static::lazy_static! {
    pub static ref ADAPTER_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "adapter_requests_total",
        "Total adapter requests",
        &["corridor_id", "adapter_type", "status"]
    )
    .unwrap();

    pub static ref ADAPTER_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "adapter_request_duration_seconds",
        "Adapter request duration",
        &["corridor_id", "adapter_type"]
    )
    .unwrap();

    pub static ref DLQ_SIZE: IntGaugeVec = register_int_gauge_vec!(
        "adapter_dlq_size",
        "DLQ size per corridor",
        &["corridor_id"]
    )
    .unwrap();

    pub static ref CIRCUIT_BREAKER_STATE: IntGaugeVec = register_int_gauge_vec!(
        "adapter_circuit_breaker_state",
        "Circuit breaker state (0=closed, 1=half-open, 2=open)",
        &["corridor_id"]
    )
    .unwrap();
}