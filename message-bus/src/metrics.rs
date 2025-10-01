//! Prometheus metrics for message bus

use lazy_static::lazy_static;
use prometheus::{register_counter_vec, register_histogram_vec, CounterVec, HistogramVec};

lazy_static! {
    /// Total messages published
    pub static ref MESSAGE_PUBLISH_TOTAL: CounterVec = register_counter_vec!(
        "message_bus_publish_total",
        "Total messages published",
        &["message_type", "status"]
    )
    .unwrap();

    /// Message publish duration
    pub static ref MESSAGE_PUBLISH_DURATION: HistogramVec = register_histogram_vec!(
        "message_bus_publish_duration_seconds",
        "Message publish duration in seconds",
        &["message_type"]
    )
    .unwrap();

    /// Total messages received
    pub static ref MESSAGE_RECEIVE_TOTAL: CounterVec = register_counter_vec!(
        "message_bus_receive_total",
        "Total messages received",
        &["message_type", "status"]
    )
    .unwrap();

    /// Message processing duration
    pub static ref MESSAGE_PROCESS_DURATION: HistogramVec = register_histogram_vec!(
        "message_bus_process_duration_seconds",
        "Message processing duration in seconds",
        &["message_type"]
    )
    .unwrap();

    /// NATS connection status
    pub static ref NATS_CONNECTION_STATUS: CounterVec = register_counter_vec!(
        "nats_connection_status",
        "NATS connection status (connected/disconnected)",
        &["status"]
    )
    .unwrap();
}
