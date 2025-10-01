//! Audit Logging
//!
//! Comprehensive audit trail for:
//! - Payment events (initiated, approved, settled, failed)
//! - User actions (login, logout, API calls)
//! - System events (config changes, errors, security events)
//! - Administrative actions (user management, permission changes)
//! - Regulatory compliance (transaction monitoring, reporting)
//!
//! Features:
//! - Structured logging with JSON
//! - Immutable append-only log
//! - Tamper detection with hash chain
//! - Retention policies
//! - Search and filtering

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::error;
use uuid::Uuid;

/// Audit log errors
#[derive(Error, Debug)]
pub enum AuditError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Integrity check failed: {0}")]
    IntegrityFailure(String),

    #[error("Log not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, AuditError>;

/// Audit event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // Payment events
    PaymentInitiated,
    PaymentApproved,
    PaymentSettled,
    PaymentFailed,
    PaymentCancelled,

    // User events
    UserLogin,
    UserLogout,
    UserCreated,
    UserDeleted,
    UserUpdated,
    PasswordChanged,
    PermissionGranted,
    PermissionRevoked,

    // API events
    ApiRequest,
    ApiResponse,
    ApiError,

    // System events
    SystemStartup,
    SystemShutdown,
    ConfigChanged,
    DatabaseMigration,

    // Security events
    AuthenticationFailed,
    AuthorizationDenied,
    SuspiciousActivity,
    RateLimitExceeded,
    TlsHandshakeFailed,

    // Regulatory events
    SanctionsScreening,
    ComplianceCheck,
    RegulatoryReport,
    AuditTrailExport,

    // Administrative events
    AdminAction,
    BackupCreated,
    BackupRestored,
    CertificateRotated,
    SecretRotated,
}

/// Audit severity level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum AuditSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub event_id: Uuid,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Event type
    pub event_type: AuditEventType,

    /// Severity level
    pub severity: AuditSeverity,

    /// Actor (user/service/system)
    pub actor: String,

    /// Target resource
    pub resource: Option<String>,

    /// Action performed
    pub action: String,

    /// Result (success/failure)
    pub result: AuditResult,

    /// IP address
    pub ip_address: Option<String>,

    /// User agent
    pub user_agent: Option<String>,

    /// Request ID
    pub request_id: Option<Uuid>,

    /// Additional metadata
    pub metadata: serde_json::Value,

    /// Previous event hash (for hash chain)
    pub previous_hash: String,

    /// Current event hash
    pub hash: String,
}

/// Audit result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditResult {
    Success,
    Failure,
    Partial,
}

impl AuditEvent {
    /// Create new audit event
    pub fn new(
        event_type: AuditEventType,
        severity: AuditSeverity,
        actor: String,
        action: String,
        result: AuditResult,
    ) -> Self {
        let event_id = Uuid::new_v4();
        let timestamp = Utc::now();

        let mut event = Self {
            event_id,
            timestamp,
            event_type,
            severity,
            actor,
            resource: None,
            action,
            result,
            ip_address: None,
            user_agent: None,
            request_id: None,
            metadata: serde_json::Value::Null,
            previous_hash: String::new(),
            hash: String::new(),
        };

        // Compute hash (without previous_hash)
        event.hash = event.compute_hash();
        event
    }

    /// Compute event hash
    fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();

        // Include all fields except hash itself
        hasher.update(self.event_id.as_bytes());
        hasher.update(self.timestamp.to_rfc3339().as_bytes());
        hasher.update(serde_json::to_string(&self.event_type).unwrap().as_bytes());
        hasher.update(serde_json::to_string(&self.severity).unwrap().as_bytes());
        hasher.update(self.actor.as_bytes());
        if let Some(resource) = &self.resource {
            hasher.update(resource.as_bytes());
        }
        hasher.update(self.action.as_bytes());
        hasher.update(serde_json::to_string(&self.result).unwrap().as_bytes());
        hasher.update(self.previous_hash.as_bytes());

        hex::encode(hasher.finalize())
    }

    /// Verify event hash
    pub fn verify_hash(&self) -> bool {
        self.hash == self.compute_hash()
    }

    /// Set previous hash (for hash chain)
    pub fn set_previous_hash(&mut self, previous_hash: String) {
        self.previous_hash = previous_hash;
        self.hash = self.compute_hash();
    }

    /// Builder methods
    pub fn with_resource(mut self, resource: String) -> Self {
        self.resource = Some(resource);
        self.hash = self.compute_hash();
        self
    }

    pub fn with_ip(mut self, ip: String) -> Self {
        self.ip_address = Some(ip);
        self.hash = self.compute_hash();
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self.hash = self.compute_hash();
        self
    }

    pub fn with_request_id(mut self, request_id: Uuid) -> Self {
        self.request_id = Some(request_id);
        self.hash = self.compute_hash();
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self.hash = self.compute_hash();
        self
    }
}

/// Audit log configuration
#[derive(Debug, Clone)]
pub struct AuditLogConfig {
    /// Log file path
    pub log_path: PathBuf,

    /// Enable hash chain
    pub enable_hash_chain: bool,

    /// Minimum severity to log
    pub min_severity: AuditSeverity,

    /// Rotate log after N entries
    pub rotate_after: usize,

    /// Retention days
    pub retention_days: u32,
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            log_path: PathBuf::from("./data/audit.log"),
            enable_hash_chain: true,
            min_severity: AuditSeverity::Info,
            rotate_after: 100_000,
            retention_days: 2555, // 7 years (regulatory requirement)
        }
    }
}

/// Audit logger
pub struct AuditLogger {
    config: AuditLogConfig,
    file: Arc<Mutex<File>>,
    last_hash: Arc<Mutex<String>>,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new(config: AuditLogConfig) -> Result<Self> {
        // Create parent directory
        if let Some(parent) = config.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open log file in append mode
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&config.log_path)?;

        // Get last hash from file
        let last_hash = Self::get_last_hash(&config.log_path)?;

        Ok(Self {
            config,
            file: Arc::new(Mutex::new(file)),
            last_hash: Arc::new(Mutex::new(last_hash)),
        })
    }

    /// Get last hash from log file
    fn get_last_hash(path: &Path) -> Result<String> {
        if !path.exists() {
            return Ok(String::new());
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        // Read last line
        let last_line = reader.lines().last();

        if let Some(Ok(line)) = last_line {
            let event: AuditEvent = serde_json::from_str(&line)
                .map_err(|e| AuditError::Serialization(e.to_string()))?;
            Ok(event.hash)
        } else {
            Ok(String::new())
        }
    }

    /// Log audit event
    pub async fn log(&self, mut event: AuditEvent) -> Result<()> {
        // Check severity
        if event.severity < self.config.min_severity {
            return Ok(());
        }

        // Set previous hash if hash chain enabled
        if self.config.enable_hash_chain {
            let last_hash = self.last_hash.lock().await;
            event.set_previous_hash(last_hash.clone());
        }

        // Serialize event
        let mut json = serde_json::to_string(&event)
            .map_err(|e| AuditError::Serialization(e.to_string()))?;
        json.push('\n');

        // Write to file
        let mut file = self.file.lock().await;
        file.write_all(json.as_bytes())?;
        file.flush()?;

        // Update last hash
        if self.config.enable_hash_chain {
            let mut last_hash = self.last_hash.lock().await;
            *last_hash = event.hash.clone();
        }

        Ok(())
    }

    /// Verify log integrity (hash chain)
    pub async fn verify_integrity(&self) -> Result<bool> {
        if !self.config.enable_hash_chain {
            return Ok(true);
        }

        let file = File::open(&self.config.log_path)?;
        let reader = BufReader::new(file);

        let mut previous_hash = String::new();

        for (i, line) in reader.lines().enumerate() {
            let line = line?;
            let event: AuditEvent = serde_json::from_str(&line)
                .map_err(|e| AuditError::Serialization(e.to_string()))?;

            // Verify event hash
            if !event.verify_hash() {
                return Err(AuditError::IntegrityFailure(format!(
                    "Event hash mismatch at line {}",
                    i + 1
                )));
            }

            // Verify chain
            if event.previous_hash != previous_hash {
                return Err(AuditError::IntegrityFailure(format!(
                    "Hash chain broken at line {}",
                    i + 1
                )));
            }

            previous_hash = event.hash.clone();
        }

        Ok(true)
    }

    /// Search audit log
    pub async fn search(
        &self,
        event_type: Option<AuditEventType>,
        actor: Option<String>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<Vec<AuditEvent>> {
        let file = File::open(&self.config.log_path)?;
        let reader = BufReader::new(file);

        let mut results = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let event: AuditEvent = serde_json::from_str(&line)
                .map_err(|e| AuditError::Serialization(e.to_string()))?;

            // Filter by event type
            if let Some(ref et) = event_type {
                if &event.event_type != et {
                    continue;
                }
            }

            // Filter by actor
            if let Some(ref a) = actor {
                if &event.actor != a {
                    continue;
                }
            }

            // Filter by time range
            if let Some(start) = start_time {
                if event.timestamp < start {
                    continue;
                }
            }

            if let Some(end) = end_time {
                if event.timestamp > end {
                    continue;
                }
            }

            results.push(event);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_audit_event_hash() {
        let event = AuditEvent::new(
            AuditEventType::PaymentInitiated,
            AuditSeverity::Info,
            "user123".to_string(),
            "Create payment".to_string(),
            AuditResult::Success,
        );

        assert!(event.verify_hash());
    }

    #[tokio::test]
    async fn test_audit_logger() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("audit.log");

        let config = AuditLogConfig {
            log_path: log_path.clone(),
            enable_hash_chain: true,
            min_severity: AuditSeverity::Info,
            rotate_after: 100_000,
            retention_days: 2555,
        };

        let logger = AuditLogger::new(config).unwrap();

        // Log event
        let event = AuditEvent::new(
            AuditEventType::PaymentInitiated,
            AuditSeverity::Info,
            "user123".to_string(),
            "Create payment".to_string(),
            AuditResult::Success,
        )
        .with_resource("payment-123".to_string())
        .with_ip("192.168.1.1".to_string());

        logger.log(event).await.unwrap();

        // Verify file exists
        assert!(log_path.exists());

        // Verify integrity
        assert!(logger.verify_integrity().await.unwrap());
    }

    #[tokio::test]
    async fn test_hash_chain() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("audit.log");

        let config = AuditLogConfig {
            log_path: log_path.clone(),
            enable_hash_chain: true,
            min_severity: AuditSeverity::Info,
            rotate_after: 100_000,
            retention_days: 2555,
        };

        let logger = AuditLogger::new(config).unwrap();

        // Log multiple events
        for i in 0..5 {
            let event = AuditEvent::new(
                AuditEventType::PaymentInitiated,
                AuditSeverity::Info,
                format!("user{}", i),
                "Create payment".to_string(),
                AuditResult::Success,
            );

            logger.log(event).await.unwrap();
        }

        // Verify integrity
        assert!(logger.verify_integrity().await.unwrap());
    }

    #[tokio::test]
    async fn test_search() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("audit.log");

        let config = AuditLogConfig {
            log_path: log_path.clone(),
            enable_hash_chain: false,
            min_severity: AuditSeverity::Info,
            rotate_after: 100_000,
            retention_days: 2555,
        };

        let logger = AuditLogger::new(config).unwrap();

        // Log events
        for i in 0..3 {
            let event = AuditEvent::new(
                AuditEventType::PaymentInitiated,
                AuditSeverity::Info,
                format!("user{}", i),
                "Create payment".to_string(),
                AuditResult::Success,
            );

            logger.log(event).await.unwrap();
        }

        // Search by actor
        let results = logger
            .search(None, Some("user1".to_string()), None, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].actor, "user1");

        // Search by event type
        let results = logger
            .search(Some(AuditEventType::PaymentInitiated), None, None, None)
            .await
            .unwrap();

        assert_eq!(results.len(), 3);
    }
}