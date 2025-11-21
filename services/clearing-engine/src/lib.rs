// Clearing Engine Library
// Production-ready clearing and netting implementation

pub mod cache;
pub mod config;
pub mod consensus;
pub mod database;
pub mod errors;
pub mod models;
pub mod netting;
pub mod window;
pub mod orchestrator;
pub mod iso20022;
pub mod metrics;
pub mod nats_consumer;

// Re-exports
pub use errors::{ClearingError, Result};
pub use models::*;
pub use netting::NettingEngine;
pub use window::{WindowManager, WindowConfig};
pub use orchestrator::{ClearingOrchestrator, ClearingResult};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SERVICE_NAME: &str = "clearing-engine";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_service_name() {
        assert_eq!(SERVICE_NAME, "clearing-engine");
    }
}
