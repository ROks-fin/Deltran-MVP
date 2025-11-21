pub mod circuit;
pub mod config;
pub mod database;
pub mod errors;
pub mod handlers;
pub mod limits;
pub mod models;
pub mod scoring;
pub mod path_selector;
pub mod nats_consumer;

// Re-exports for convenience
pub use path_selector::PathSelector;
pub use scoring::RiskScorer;
