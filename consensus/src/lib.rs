//! CometBFT Consensus Integration
//!
//! Integrates the ledger core with CometBFT for Byzantine Fault Tolerant consensus.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                 CometBFT Network                     │
//! │         (Byzantine Fault Tolerant)                   │
//! │   Validator 1 | Validator 2 | Validator 3          │
//! └────────────────────┬────────────────────────────────┘
//!                      │ Consensus (2/3 majority)
//!                      ↓
//! ┌─────────────────────────────────────────────────────┐
//! │              ABCI Application                        │
//! │  CheckTx → DeliverTx → Commit                       │
//! └────────────────────┬────────────────────────────────┘
//!                      │
//!                      ↓
//! ┌─────────────────────────────────────────────────────┐
//! │              Ledger Core                             │
//! │  Event sourcing + Merkle proofs                     │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # ABCI Methods
//!
//! - **CheckTx**: Validate transaction before mempool
//! - **DeliverTx**: Execute transaction in block
//! - **Commit**: Finalize block with Merkle root
//! - **Query**: Read-only queries
//! - **InitChain**: Initialize blockchain state
//!
//! # Byzantine Fault Tolerance
//!
//! - Tolerates up to 1/3 malicious validators
//! - 2/3 majority required for consensus
//! - Block finality in ~6 seconds
//! - Deterministic state machine replication

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::all
)]

pub mod abci;
pub mod error;
pub mod state;
pub mod config;

// Re-exports
pub use error::{Error, Result};
pub use abci::LedgerApp;
pub use config::Config;