//! Event-Driven Regulatory Reporting Module
//!
//! Provides:
//! - Event-driven report generation from bronze/silver/gold layers
//! - IFRS compliance mapping from subledger
//! - PII tokenization and masking
//! - Automated report scheduling and submission

#![forbid(unsafe_code)]

pub mod event_reporter;
pub mod ifrs_mapper;
pub mod pii_protection;
pub mod report_generator;
pub mod scheduler;

pub use event_reporter::{EventReporter, ReportingEvent};
pub use ifrs_mapper::{IfrsMapper, IfrsReport, IfrsLineItem};
pub use pii_protection::{PiiProtection, TokenizationService, MaskingService};
pub use report_generator::{ReportGenerator, ReportFormat, GeneratedReport};
pub use scheduler::{ReportScheduler, ScheduledJob};
