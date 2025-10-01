# DelTran Security Module

Comprehensive security infrastructure for the DelTran Settlement Rail.

## Overview

The security module provides defense-in-depth protection across multiple layers:

```
┌─────────────────────────────────────────────────────┐
│                  Security Layer                      │
├─────────────────────────────────────────────────────┤
│  TLS/mTLS  │  Rate Limiter  │  Secrets Manager     │
│  Audit Log │  Input Sanitizer │  Validators        │
└─────────────────────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────┐
│              Application Services                    │
│  Gateway │ Ledger │ Settlement │ Consensus          │
└─────────────────────────────────────────────────────┘
```

## Components

### 1. TLS/mTLS (`tls_config.rs`)

Mutual TLS authentication for inter-service communication.

**Features:**
- Certificate generation (development)
- Certificate validation
- Cipher suite configuration
- TLS 1.2/1.3 support
- ALPN for gRPC (h2)

**Usage:**

```rust
use security::tls_config::{TlsConfig, CertificateGenerator};

// Generate certificates
CertificateGenerator::generate_ca("./certs", "DelTran CA")?;
CertificateGenerator::generate_service_cert(
    "./certs",
    "./certs/ca.crt",
    "./certs/ca.key",
    "gateway",
    vec!["gateway.deltran.local".to_string()],
)?;

// Load configuration
let config = TlsConfig::from_env()?;
let server_config = config.build_server_config()?;
let client_config = config.build_client_config()?;
```

**Environment Variables:**
```bash
TLS_CERT_PATH=./certs/server.crt
TLS_KEY_PATH=./certs/server.key
TLS_CA_CERT_PATH=./certs/ca.crt
TLS_REQUIRE_CLIENT_CERT=true
```

### 2. Rate Limiting (`rate_limiter.rs`)

Multi-tier rate limiting with DDoS protection.

**Features:**
- Token bucket algorithm
- Sliding window rate limiting
- Per-IP limits
- Per-account limits
- Global limits
- Adaptive rate limiting (system load-based)

**Algorithms:**

**Token Bucket:**
- Burst capacity: 100 requests
- Refill rate: 16.67 tokens/second (~1k req/min)
- Smooth rate limiting

**Sliding Window:**
- Window size: 60 seconds
- Max requests: 1,000
- Prevents burst attacks

**Usage:**

```rust
use security::rate_limiter::{RateLimiter, RateLimiterConfig};

let config = RateLimiterConfig {
    max_requests: 1000,
    window_duration: Duration::from_secs(60),
    burst_size: 100,
    refill_rate: 16.67,
    adaptive: true,
    adaptive_threshold: 0.8,
};

let limiter = RateLimiter::new(config);

// Check IP
match limiter.check_ip("192.168.1.1".parse()?).await {
    RateLimitResult::Allowed => { /* OK */ },
    RateLimitResult::Denied { retry_after } => {
        // Return 429 with Retry-After header
    },
    RateLimitResult::SystemOverload => {
        // Return 503
    },
}

// Update system load for adaptive limiting
limiter.update_system_load(0.85).await;
```

**Performance:**
- Per-IP: 1,000 requests/minute
- Per-account: 2,000 requests/minute (2x for authenticated)
- Global: 10,000 requests/minute
- Adaptive throttling at 80% system load

### 3. Secrets Management (`secrets_manager.rs`)

Secure storage and retrieval of sensitive data.

**Backends:**
- **Environment variables** (development only)
- **Encrypted file** (AES-256-GCM)
- **HashiCorp Vault** (planned)
- **AWS Secrets Manager** (planned)

**Features:**
- AES-256-GCM encryption
- Secret rotation
- Version tracking
- Audit trail

**Usage:**

```rust
use security::secrets_manager::{SecretsManager, BackendConfig};

// Encrypted file backend
let config = BackendConfig::EncryptedFile {
    path: "./secrets.enc".into(),
    master_key_env: "MASTER_KEY".to_string(),
};

let manager = SecretsManager::from_config(config)?;

// Set secret
manager.set_secret("db_password", "super_secret_123")?;

// Get secret
let password = manager.get_secret("db_password")?;

// Rotate secret
manager.rotate_secret("db_password", "new_password_456")?;

// Helpers
let db_url = manager.get_db_url()?;
let ledger_key = manager.get_ledger_key()?;
let validator_key = manager.get_validator_key()?;
```

**Master Key Generation:**
```bash
# Generate 256-bit key
openssl rand -hex 32 > master_key.txt
export MASTER_KEY=$(cat master_key.txt)
```

**Security:**
- Master key stored in environment/KMS
- Secrets encrypted at rest
- No secrets in logs
- Rotation audit trail

### 4. Audit Logging (`audit_log.rs`)

Tamper-proof audit trail for compliance.

**Features:**
- Append-only log
- Hash chain for tamper detection
- Structured JSON logging
- Search and filtering
- 7-year retention (regulatory requirement)

**Event Types:**
- Payment events (initiated, approved, settled, failed)
- User events (login, logout, created, deleted)
- API events (request, response, error)
- System events (startup, shutdown, config change)
- Security events (auth failed, suspicious activity)
- Regulatory events (sanctions screening, compliance check)

**Hash Chain:**
```
Event 1 → Hash 1
          ↓
Event 2 → Hash 2 (includes Hash 1)
          ↓
Event 3 → Hash 3 (includes Hash 2)
```

Each event includes the hash of the previous event, creating an immutable chain.

**Usage:**

```rust
use security::audit_log::{
    AuditLogger, AuditEvent, AuditEventType,
    AuditSeverity, AuditResult
};

let config = AuditLogConfig::default();
let logger = AuditLogger::new(config)?;

// Log event
let event = AuditEvent::new(
    AuditEventType::PaymentInitiated,
    AuditSeverity::Info,
    "user@example.com".to_string(),
    "Initiate payment".to_string(),
    AuditResult::Success,
)
.with_resource("payment-12345".to_string())
.with_ip("192.168.1.1".to_string())
.with_request_id(Uuid::new_v4());

logger.log(event).await?;

// Verify integrity
let is_valid = logger.verify_integrity().await?;

// Search
let events = logger.search(
    Some(AuditEventType::PaymentInitiated),
    Some("user@example.com".to_string()),
    Some(start_time),
    Some(end_time),
).await?;
```

**Log Format (JSON):**
```json
{
  "event_id": "550e8400-e29b-41d4-a716-446655440000",
  "timestamp": "2024-01-15T10:30:00Z",
  "event_type": "payment_initiated",
  "severity": "info",
  "actor": "user@example.com",
  "resource": "payment-12345",
  "action": "Initiate payment",
  "result": "success",
  "ip_address": "192.168.1.1",
  "user_agent": "Mozilla/5.0...",
  "request_id": "...",
  "metadata": {},
  "previous_hash": "abc123...",
  "hash": "def456..."
}
```

### 5. Input Sanitization (`input_sanitizer.rs`)

Comprehensive input validation and sanitization.

**Protection Against:**
- SQL injection
- XSS (Cross-Site Scripting)
- Command injection
- Buffer overflow
- Unicode attacks

**Validators:**
- BIC codes (SWIFT)
- IBAN
- Email addresses (RFC 5322)
- Phone numbers (E.164)
- Payment amounts (exact decimal)
- Currency codes (ISO 4217)
- Names and addresses

**Usage:**

```rust
use security::input_sanitizer::InputSanitizer;
use rust_decimal::Decimal;

let sanitizer = InputSanitizer::new();

// BIC validation
let bic = sanitizer.sanitize_bic("DEUTDEFF")?;

// IBAN validation
let iban = sanitizer.sanitize_iban("DE89 3704 0044 0532 0130 00")?;

// Amount validation
let amount = Decimal::new(10000, 2); // $100.00
let min = Decimal::new(1, 2);
let max = Decimal::new(100000000, 2);
let sanitized = sanitizer.sanitize_amount(amount, min, max)?;

// Name sanitization
let name = sanitizer.sanitize_name("John O'Brien")?;

// Payment reference
let reference = sanitizer.sanitize_payment_reference("INV-2024-001")?;

// Currency validation
let currency = sanitizer.sanitize_currency("USD")?;

// Security checks
sanitizer.check_sql_injection("user input")?;
sanitizer.check_xss("<script>alert(1)</script>")?; // Error
sanitizer.check_command_injection("file.txt; rm -rf /")?; // Error
```

**Validation Rules:**

| Field | Format | Max Length | Validation |
|-------|--------|-----------|-----------|
| BIC | AAAABBCCXXX | 8 or 11 | ISO 9362 |
| IBAN | CC12AAAA... | 15-34 | ISO 13616 |
| Amount | 0.00 | - | Positive, 2 decimals |
| Currency | USD | 3 | ISO 4217 |
| Name | Text | 140 | Letters, spaces, '-.,áéíóú |
| Address | Text | 70 | Alphanumeric + common chars |
| Reference | Alphanumeric | 35 | ISO 20022 |

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
security = { path = "../security" }
```

## Configuration

### Environment Variables

```bash
# TLS
export TLS_CERT_PATH=./certs/server.crt
export TLS_KEY_PATH=./certs/server.key
export TLS_CA_CERT_PATH=./certs/ca.crt
export TLS_REQUIRE_CLIENT_CERT=true

# Secrets
export MASTER_KEY=$(openssl rand -hex 32)

# Rate Limiting
export RATE_LIMIT_MAX_REQUESTS=1000
export RATE_LIMIT_WINDOW_SECONDS=60
export RATE_LIMIT_BURST_SIZE=100

# Audit Log
export AUDIT_LOG_PATH=./data/audit.log
export AUDIT_LOG_ENABLE_HASH_CHAIN=true
export AUDIT_LOG_RETENTION_DAYS=2555
```

## Testing

### Unit Tests

```bash
cargo test --package security
```

### Integration Tests

```bash
cargo test --package security --test integration_tests
```

### Security Audit

```bash
# Dependency audit
cargo audit

# Security linting
cargo clippy -- -D warnings
```

## Performance

### Benchmarks

| Operation | Throughput | Latency (p95) |
|-----------|-----------|--------------|
| TLS handshake | 1,000/sec | 5ms |
| Rate limit check | 100,000/sec | <1µs |
| Secret retrieval | 50,000/sec | <1ms |
| Audit log write | 10,000/sec | 1ms |
| Input sanitization | 200,000/sec | <1µs |

### Memory Usage

| Component | Memory |
|-----------|--------|
| TLS | ~10 MB per connection |
| Rate limiter | ~100 KB per 1,000 IPs |
| Secrets manager | ~1 MB base |
| Audit logger | ~5 MB buffer |

## Security Best Practices

### Production Deployment

1. **TLS Configuration**
   - Use CA-signed certificates
   - Enable mTLS for all inter-service communication
   - Rotate certificates every 90 days
   - Use TLS 1.3 (fallback to 1.2)
   - Disable weak cipher suites

2. **Secrets Management**
   - Use Vault or AWS Secrets Manager
   - Never commit secrets to version control
   - Rotate secrets every 90 days
   - Different secrets per environment
   - Audit all secret access

3. **Rate Limiting**
   - Configure per-service limits
   - Enable adaptive limiting
   - Monitor rate limit metrics
   - Use DDoS mitigation (Cloudflare, AWS Shield)
   - Whitelist trusted IPs

4. **Audit Logging**
   - Enable for all critical operations
   - Store logs in immutable storage (S3 with versioning)
   - Retain for 7 years (SOX, GDPR)
   - Monitor for anomalies
   - Regular integrity checks

5. **Input Validation**
   - Validate all user input
   - Sanitize before database operations
   - Use parameterized queries
   - Implement CSRF protection
   - Content Security Policy (CSP)

### Compliance

This module helps meet requirements for:

- **PCI DSS** - Payment card data security
  - Requirement 2: Default passwords changed
  - Requirement 4: Encrypt transmission of cardholder data
  - Requirement 8: Identify and authenticate access
  - Requirement 10: Track and monitor access

- **GDPR** - Data protection and privacy
  - Article 32: Security of processing
  - Article 33: Breach notification
  - Article 30: Records of processing

- **SOX** - Financial reporting controls
  - Section 302: Corporate responsibility
  - Section 404: Internal controls

- **SWIFT CSP** - Customer Security Programme
  - Control 1.1: Restrict internet access
  - Control 2.9: Encryption
  - Control 6.4: Security monitoring

- **ISO 27001** - Information security management
  - A.9: Access control
  - A.10: Cryptography
  - A.12: Operations security
  - A.18: Compliance

## Incident Response

### Security Event Detection

Monitor for:
- Multiple failed authentication attempts
- Rate limit exceeded (sustained)
- SQL injection attempts
- XSS attempts
- Unusual API patterns
- Certificate errors
- Hash chain breaks

### Response Procedures

1. **Authentication Failures**
   - Lock account after 5 attempts
   - Notify security team
   - Log IP address
   - Investigate source

2. **Rate Limit Exceeded**
   - Temporary IP ban (15 minutes)
   - Notify monitoring
   - Investigate for DDoS

3. **Injection Attempts**
   - Block IP immediately
   - Log full request
   - Alert security team
   - File incident report

4. **Hash Chain Break**
   - Stop processing immediately
   - Preserve logs
   - Investigate tampering
   - Restore from backup

## Monitoring

### Metrics

```prometheus
# Rate limiting
rate_limit_requests_total{ip="192.168.1.1", result="allowed"}
rate_limit_requests_total{ip="192.168.1.1", result="denied"}
rate_limit_system_load

# Secrets
secrets_retrieval_total{secret="db_password"}
secrets_rotation_total{secret="db_password"}

# Audit log
audit_log_events_total{event_type="payment_initiated"}
audit_log_integrity_checks_total{result="success"}

# TLS
tls_handshake_duration_seconds
tls_certificate_expiry_days

# Input validation
input_validation_total{field="bic", result="success"}
input_validation_total{field="amount", result="error"}
```

### Alerts

```yaml
- alert: HighRateLimitDenials
  expr: rate(rate_limit_requests_total{result="denied"}[5m]) > 100
  severity: warning

- alert: AuditLogIntegrityFailure
  expr: audit_log_integrity_checks_total{result="failure"} > 0
  severity: critical

- alert: TLSCertificateExpiring
  expr: tls_certificate_expiry_days < 30
  severity: warning

- alert: SuspiciousActivity
  expr: rate(audit_log_events_total{event_type="suspicious_activity"}[5m]) > 10
  severity: critical
```

## License

Copyright 2024 DelTran. All rights reserved.