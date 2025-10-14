//! Risk Engine for DelTran
//!
//! Real-time risk assessment for cross-border payments

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod types;
pub mod limits;
pub mod scoring;

pub use error::{Error, Result};
pub use types::*;
pub use limits::LimitChecker;
pub use scoring::RiskScorer;
