//! # DelTran Adapters
//!
//! Bank/PSP/CBDC connectivity layer with:
//! - ISO 20022 pacs.008 full support
//! - Dead Letter Queue (DLQ) with retry logic
//! - Circuit-breaker per corridor
//! - Kill-switch mechanism
//! - Health monitoring
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │         Adapter Manager (Orchestrator)              │
//! └────────────┬────────────────────────────────────────┘
//!              │
//!     ┌────────┼────────────────┬────────────┐
//!     │        │                │            │
//! ┌───▼────┐ ┌─▼──────┐ ┌──────▼──┐ ┌───────▼──────┐
//! │ SWIFT  │ │  ACH   │ │  RTGS   │ │ CBDC Bridge  │
//! │Adapter │ │Adapter │ │ Adapter │ │   Adapter    │
//! └───┬────┘ └─┬──────┘ └──────┬──┘ └───────┬──────┘
//!     │        │                │            │
//!     └────────┼────────────────┴────────────┘
//!              │
//! ┌────────────▼─────────────────────────────────────┐
//! │       Circuit-Breaker + DLQ + Kill-Switch        │
//! └──────────────────────────────────────────────────┘
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, unused_qualifications)]

pub mod circuit_breaker;
pub mod connector;
pub mod dlq;
pub mod error;
pub mod iso20022;
pub mod kill_switch;
pub mod manager;
pub mod metrics;
pub mod swift;
pub mod types;

pub use error::{Error, Result};
pub use manager::AdapterManager;
pub use types::*;

/// Default DLQ retry attempts
pub const DEFAULT_RETRY_ATTEMPTS: u32 = 3;

/// Default circuit breaker threshold (failures before opening)
pub const DEFAULT_CB_FAILURE_THRESHOLD: u32 = 5;

/// Default circuit breaker timeout (seconds before half-open)
pub const DEFAULT_CB_TIMEOUT_SECONDS: u64 = 60;

/// Default request timeout (seconds)
pub const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 30;