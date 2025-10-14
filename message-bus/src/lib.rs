//! Message Bus for DelTran
//!
//! Provides pub/sub messaging using NATS/JetStream

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod client;
pub mod types;

pub use error::{Error, Result};
pub use client::MessageBusClient;
pub use types::{Message, Subject};
