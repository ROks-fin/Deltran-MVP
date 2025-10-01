//! # DelTran Protocol Core
//!
//! Implements the formal Protocol Layer semantics:
//! - INSTRUCT_PAYMENT: Payment initiation with eligibility tokens
//! - NETTING: Multilateral netting proposals and confirmations
//! - FINALIZE: 2-phase commit settlement with partial settlement support
//! - SETTLEMENT_PROOF: Cryptographic proofs with BFT + HSM signatures
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  State Machine  │ ← Protocol state transitions
//! └────────┬────────┘
//!          │
//! ┌────────▼────────┐
//! │   Validation    │ ← Eligibility, sanctions, schema
//! └────────┬────────┘
//!          │
//! ┌────────▼────────┐
//! │  Cryptography   │ ← Ed25519, SHA3, Merkle, HSM
//! └────────┬────────┘
//!          │
//! ┌────────▼────────┐
//! │ Settlement Proof│ ← Multi-sig + HSM seal
//! └─────────────────┘
//! ```
//!
//! ## Safety
//!
//! - `#![forbid(unsafe_code)]`: No unsafe operations
//! - Money invariants enforced at type level
//! - Deterministic canonical serialization
//! - Replay protection with nonce + TTL

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    unused_qualifications,
    clippy::all
)]

pub mod canonical;
pub mod checkpoint;
pub mod compliance;
pub mod error;
pub mod hsm;
pub mod merkle;
pub mod partial;
pub mod proof;
pub mod reporting;
pub mod state;
pub mod types;
pub mod validation;

pub use error::{Error, Result};
pub use state::{ProtocolState, StateMachine};
pub use types::*;

/// Protocol version (semantic versioning)
pub const PROTOCOL_VERSION: u16 = 1;

/// Network ID for mainnet
pub const NETWORK_MAINNET: &str = "deltran-mainnet";

/// Network ID for testnet
pub const NETWORK_TESTNET: &str = "deltran-testnet";

/// Default TTL for payment instructions (5 minutes)
pub const DEFAULT_TTL_SECONDS: u32 = 300;

/// Default checkpoint interval (blocks)
pub const DEFAULT_CHECKPOINT_INTERVAL: u64 = 100;

/// BFT quorum requirement (5 of 7 validators = 71.4%)
pub const BFT_QUORUM_NUMERATOR: u32 = 5;
pub const BFT_QUORUM_DENOMINATOR: u32 = 7;

/// Minimum netting volume (USD equivalent)
pub const MIN_NETTING_VOLUME: &str = "100000.00";

/// Minimum netting efficiency (15%)
pub const MIN_NETTING_EFFICIENCY: f64 = 0.15;

/// Minimum participants for netting
pub const MIN_NETTING_PARTICIPANTS: usize = 2;

/// 2PC timeout (seconds)
pub const DEFAULT_2PC_TIMEOUT_SECONDS: u32 = 900; // 15 minutes