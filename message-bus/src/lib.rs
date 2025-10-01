//! Message Bus with NATS support
//!
//! Provides pub/sub messaging with:
//! - Partitioning by corridor_id and bank_id
//! - JetStream for persistence and guaranteed delivery
//! - Consumer groups for load balancing
//! - Retry logic with exponential backoff
//! - Observability via Prometheus metrics

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub mod client;
pub mod error;
pub mod message;
pub mod metrics;
pub mod partitioning;
pub mod publisher;
pub mod subscriber;
pub mod types;

pub use client::NatsClient;
pub use error::{Error, Result};
pub use message::Message;
pub use publisher::Publisher;
pub use subscriber::Subscriber;
pub use types::{MessageType, PartitionKey};
