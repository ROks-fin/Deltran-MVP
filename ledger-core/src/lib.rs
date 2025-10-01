//! DelTran Ledger Core
//!
//! Append-only event ledger with cryptographic signatures and Merkle proofs.
//!
//! # Architecture
//!
//! - **Event Sourcing**: All state is derived from immutable events
//! - **Single Writer**: One logical writer thread eliminates race conditions
//! - **Merkle Tree**: Cryptographic proofs of inclusion
//! - **Batching**: Amortizes fsync cost for 10x throughput

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, unused_qualifications)]
//!
//! # Invariants
//!
//! - Money conservation: Σ(debits) == Σ(credits) for all time
//! - Deterministic replay: Same events → same state
//! - Append-only: Events never modified or deleted
//! - Linearizable: Total ordering of all events

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::all
)]

pub mod types;
pub mod storage;
pub mod ledger;
pub mod merkle;
pub mod crypto;
pub mod error;
pub mod actor;
pub mod config;
pub mod metrics;

// Re-exports
pub use error::{Error, Result};
pub use types::{
    AccountId, Currency, LedgerEvent, EventType, PaymentState, PaymentStatus,
    Block, Signature,
};
pub use ledger::Ledger;
pub use config::Config;