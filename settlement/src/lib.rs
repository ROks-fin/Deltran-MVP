//! Settlement Engine
//!
//! Implements multilateral netting and batch settlement for cross-border payments.
//!
//! # Architecture

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, unused_qualifications)]
//!
//! The settlement engine operates in windows (e.g., 4 times per day):
//!
//! 1. **Collection**: Gather pending payments from ledger
//! 2. **Netting**: Compute optimal netting using min-cost flow
//! 3. **Settlement**: Generate ISO 20022 payment instructions
//! 4. **Finalization**: Record settlement batch in ledger
//!
//! # Netting Algorithm
//!
//! Uses **max-flow min-cost** algorithm to find optimal netting:
//! - Minimize number of interbank transfers
//! - Maximize netted amounts
//! - Respect liquidity constraints
//!
//! # Example
//!
//! ```no_run
//! use settlement::{Config, SettlementEngine};
//!
//! #[tokio::main]
//! async fn main() -> settlement::Result<()> {
//!     let config = Config::default();
//!     let engine = SettlementEngine::new(config).await?;
//!
//!     // Run settlement window
//!     let batch = engine.run_settlement_window().await?;
//!     println!("Settled {} payments, {} net transfers",
//!              batch.payment_count, batch.net_transfers.len());
//!
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::all
)]

pub mod types;
pub mod netting;
pub mod window;
pub mod iso20022;
pub mod error;
pub mod config;
pub mod engine;

// Re-exports
pub use error::{Error, Result};
pub use types::*;
pub use config::Config;
pub use engine::SettlementEngine;