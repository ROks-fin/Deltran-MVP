//! Security Module for DelTran
//!
//! Provides comprehensive security features:
//! - TLS/mTLS for inter-service communication
//! - Rate limiting and DDoS protection
//! - Secrets management (Vault, AWS Secrets Manager, encrypted files)
//! - Audit logging with tamper detection
//! - Input sanitization and validation
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────┐
//! │                  Security Layer                      │
//! ├─────────────────────────────────────────────────────┤
//! │  TLS/mTLS  │  Rate Limiter  │  Secrets Manager     │
//! │  Audit Log │  Input Sanitizer │  Validators        │
//! └─────────────────────────────────────────────────────┘
//!                          ↓
//! ┌─────────────────────────────────────────────────────┐
//! │              Application Services                    │
//! │  Gateway │ Ledger │ Settlement │ Consensus          │
//! └─────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! ## TLS/mTLS (`tls_config`)
//! - Mutual TLS authentication
//! - Certificate generation and rotation
//! - Cipher suite configuration
//! - TLS 1.2+ support
//!
//! ## Rate Limiting (`rate_limiter`)
//! - Token bucket algorithm
//! - Sliding window rate limiting
//! - Per-IP and per-account limits
//! - Adaptive rate limiting based on system load
//! - DDoS protection
//!
//! ## Secrets Management (`secrets_manager`)
//! - Multiple backends (Vault, AWS, encrypted files)
//! - AES-256-GCM encryption
//! - Secret rotation
//! - Audit trail
//!
//! ## Audit Logging (`audit_log`)
//! - Append-only log with hash chain
//! - Tamper detection
//! - Structured JSON logging
//! - Search and filtering
//! - 7-year retention (regulatory compliance)
//!
//! ## Input Sanitization (`input_sanitizer`)
//! - SQL injection prevention
//! - XSS prevention
//! - Command injection prevention
//! - BIC/IBAN/SWIFT validation
//! - Amount validation with exact decimals
//!
//! # Usage Examples
//!
//! ## TLS Configuration
//!
//! ```rust,no_run
//! use security::tls_config::{TlsConfig, CertificateGenerator};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Generate CA and service certificates
//! CertificateGenerator::generate_ca("./certs", "DelTran CA")?;
//! CertificateGenerator::generate_service_cert(
//!     "./certs",
//!     "./certs/ca.crt",
//!     "./certs/ca.key",
//!     "gateway",
//!     vec!["gateway.deltran.local".to_string()],
//! )?;
//!
//! // Load TLS config
//! let tls_config = TlsConfig::from_env()?;
//! let server_config = tls_config.build_server_config()?;
//! let client_config = tls_config.build_client_config()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Rate Limiting
//!
//! ```rust,no_run
//! use security::rate_limiter::{RateLimiter, RateLimiterConfig, RateLimitResult};
//! use std::net::IpAddr;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = RateLimiterConfig::default();
//! let limiter = RateLimiter::new(config);
//!
//! let ip: IpAddr = "192.168.1.1".parse()?;
//! match limiter.check_ip(ip).await {
//!     RateLimitResult::Allowed => {
//!         // Process request
//!     }
//!     RateLimitResult::Denied { retry_after } => {
//!         // Return 429 Too Many Requests
//!     }
//!     RateLimitResult::SystemOverload => {
//!         // Return 503 Service Unavailable
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Secrets Management
//!
//! ```rust,no_run
//! use security::secrets_manager::{SecretsManager, BackendConfig};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = BackendConfig::EncryptedFile {
//!     path: "./secrets.enc".into(),
//!     master_key_env: "MASTER_KEY".to_string(),
//! };
//!
//! let manager = SecretsManager::from_config(config)?;
//!
//! // Set secret
//! manager.set_secret("db_password", "super_secret_123")?;
//!
//! // Get secret
//! let password = manager.get_secret("db_password")?;
//!
//! // Rotate secret
//! manager.rotate_secret("db_password", "new_password_456")?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Audit Logging
//!
//! ```rust,no_run
//! use security::audit_log::{
//!     AuditLogger, AuditLogConfig, AuditEvent, AuditEventType,
//!     AuditSeverity, AuditResult
//! };
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = AuditLogConfig::default();
//! let logger = AuditLogger::new(config)?;
//!
//! // Log payment event
//! let event = AuditEvent::new(
//!     AuditEventType::PaymentInitiated,
//!     AuditSeverity::Info,
//!     "user@example.com".to_string(),
//!     "Initiate payment".to_string(),
//!     AuditResult::Success,
//! )
//! .with_resource("payment-12345".to_string())
//! .with_ip("192.168.1.1".to_string());
//!
//! logger.log(event).await?;
//!
//! // Verify integrity
//! let is_valid = logger.verify_integrity().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Input Sanitization
//!
//! ```rust,no_run
//! use security::input_sanitizer::InputSanitizer;
//! use rust_decimal::Decimal;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let sanitizer = InputSanitizer::new();
//!
//! // Sanitize BIC
//! let bic = sanitizer.sanitize_bic("DEUTDEFF")?;
//!
//! // Sanitize amount
//! let amount = Decimal::new(10000, 2); // $100.00
//! let min = Decimal::new(1, 2); // $0.01
//! let max = Decimal::new(100000000, 2); // $1,000,000.00
//! let sanitized = sanitizer.sanitize_amount(amount, min, max)?;
//!
//! // Check for SQL injection
//! sanitizer.check_sql_injection("user input")?;
//! # Ok(())
//! # }
//! ```
//!
//! # Security Best Practices
//!
//! ## Production Deployment
//!
//! 1. **TLS Configuration**
//!    - Use CA-signed certificates (not self-signed)
//!    - Enable mTLS for all inter-service communication
//!    - Rotate certificates every 90 days
//!    - Use TLS 1.3 when possible
//!
//! 2. **Secrets Management**
//!    - Use Vault or AWS Secrets Manager in production
//!    - Never commit secrets to version control
//!    - Rotate secrets every 90 days
//!    - Use different secrets per environment
//!
//! 3. **Rate Limiting**
//!    - Configure per-service limits
//!    - Enable adaptive rate limiting
//!    - Monitor rate limit metrics
//!    - Use DDoS mitigation service (Cloudflare, AWS Shield)
//!
//! 4. **Audit Logging**
//!    - Enable for all critical operations
//!    - Store logs in tamper-proof storage
//!    - Retain logs for 7 years (regulatory requirement)
//!    - Monitor for suspicious patterns
//!
//! 5. **Input Validation**
//!    - Validate all user input
//!    - Sanitize before database operations
//!    - Use parameterized queries
//!    - Implement CSRF protection
//!
//! ## Compliance
//!
//! This module helps meet requirements for:
//! - **PCI DSS** - Payment card data security
//! - **GDPR** - Data protection and privacy
//! - **SOX** - Financial reporting controls
//! - **SWIFT CSP** - Customer Security Programme
//! - **ISO 27001** - Information security management

#![forbid(unsafe_code)]
#![warn(
    missing_docs,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::all
)]

pub mod audit_log;
pub mod input_sanitizer;
pub mod rate_limiter;
pub mod secrets_manager;
pub mod tls_config;

// Re-exports for convenience
pub use audit_log::{AuditEvent, AuditEventType, AuditLogger, AuditResult, AuditSeverity};
pub use input_sanitizer::InputSanitizer;
pub use rate_limiter::{RateLimitResult, RateLimiter, RateLimiterConfig};
pub use secrets_manager::{BackendConfig, SecretsManager};
pub use tls_config::{CertificateGenerator, TlsConfig, TlsVersion};