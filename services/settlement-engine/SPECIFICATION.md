# Settlement Engine Service Specification

## Overview
Critical financial settlement service responsible for finalizing fund transfers between banks, managing settlement accounts, coordinating with external payment systems, and ensuring atomic settlement operations with automatic rollback capabilities.

## Core Responsibilities
1. Final settlement execution with external banking systems
2. Settlement account management and reconciliation
3. Multi-currency settlement coordination
4. Failed settlement handling and recovery
5. Settlement confirmation and notification
6. Integration with SWIFT, SEPA, and local payment rails
7. Atomic settlement operations with rollback

## Technology Stack
- **Language**: Rust (for performance and safety in financial operations)
- **Database**: PostgreSQL 16 with strong consistency
- **Message Queue**: NATS JetStream for event streaming
- **gRPC**: Internal service communication
- **External APIs**: Bank API integrations (mocked for MVP)
- **Async Runtime**: Tokio for concurrent operations

## Service Architecture

### Directory Structure
```
services/settlement-engine/
├── src/
│   ├── main.rs                 # Service entry point
│   ├── config.rs               # Configuration management
│   ├── server.rs               # gRPC server implementation
│   ├── settlement/
│   │   ├── mod.rs             # Settlement module
│   │   ├── executor.rs        # Settlement execution logic
│   │   ├── atomic.rs          # Atomic operation control
│   │   ├── rollback.rs        # Rollback mechanisms
│   │   └── validator.rs       # Pre-settlement validation
│   ├── accounts/
│   │   ├── mod.rs             # Account management
│   │   ├── nostro.rs          # Nostro account handling
│   │   ├── vostro.rs          # Vostro account handling
│   │   └── reconciliation.rs  # Account reconciliation
│   ├── integration/
│   │   ├── mod.rs             # External integrations
│   │   ├── swift.rs           # SWIFT integration
│   │   ├── sepa.rs            # SEPA integration
│   │   ├── local.rs           # Local payment systems
│   │   └── mock.rs            # Mock bank APIs
│   ├── recovery/
│   │   ├── mod.rs             # Recovery mechanisms
│   │   ├── retry.rs           # Retry logic
│   │   └── compensation.rs    # Compensation transactions
│   ├── events/
│   │   ├── mod.rs             # Event handling
│   │   ├── publisher.rs       # Event publishing
│   │   └── consumer.rs        # Event consumption
│   └── monitoring/
│       ├── mod.rs             # Monitoring
│       ├── metrics.rs         # Prometheus metrics
│       └── health.rs          # Health checks
├── proto/
│   └── settlement.proto       # gRPC definitions
├── Cargo.toml
├── Dockerfile
└── Makefile
```

## Implementation Details

### 1. Core Settlement Executor

```rust
// src/settlement/executor.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct SettlementExecutor {
    db_pool: Arc<PgPool>,
    bank_clients: Arc<BankClientManager>,
    event_publisher: Arc<EventPublisher>,
    atomic_controller: Arc<AtomicController>,
    metrics: Arc<SettlementMetrics>,
}

#[derive(Debug, Clone)]
pub struct SettlementRequest {
    pub id: Uuid,
    pub obligation_id: Uuid,
    pub from_bank: String,
    pub to_bank: String,
    pub amount: Decimal,
    pub currency: String,
    pub settlement_date: DateTime<Utc>,
    pub priority: SettlementPriority,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum SettlementStatus {
    Pending,
    Validating,
    Executing,
    Completed,
    Failed(String),
    RolledBack(String),
}

impl SettlementExecutor {
    pub async fn execute_settlement(
        &self,
        request: SettlementRequest,
    ) -> Result<SettlementResult, SettlementError> {
        // Start atomic operation
        let atomic_op = self.atomic_controller
            .begin_operation(request.id)
            .await?;

        // Create settlement transaction in database
        let settlement = self.create_settlement_record(&request).await?;

        // Execute with automatic rollback on failure
        match self.perform_atomic_settlement(&request, &atomic_op).await {
            Ok(result) => {
                atomic_op.commit().await?;
                self.publish_settlement_completed(&result).await?;
                Ok(result)
            }
            Err(e) => {
                // Automatic rollback
                atomic_op.rollback().await?;
                self.publish_settlement_failed(&request, &e).await?;
                Err(e)
            }
        }
    }

    async fn perform_atomic_settlement(
        &self,
        request: &SettlementRequest,
        atomic_op: &AtomicOperation,
    ) -> Result<SettlementResult, SettlementError> {
        // Step 1: Validate settlement prerequisites
        self.validate_settlement(request).await?;
        atomic_op.checkpoint("validation_complete").await?;

        // Step 2: Lock funds in source account
        let lock_id = self.lock_funds(
            &request.from_bank,
            &request.amount,
            &request.currency,
        ).await?;
        atomic_op.checkpoint("funds_locked").await?;

        // Step 3: Initiate external transfer
        let transfer_ref = self.initiate_external_transfer(request).await?;
        atomic_op.checkpoint("transfer_initiated").await?;

        // Step 4: Wait for confirmation with timeout
        let confirmation = self.await_confirmation(
            &transfer_ref,
            Duration::from_secs(300), // 5 minute timeout
        ).await?;
        atomic_op.checkpoint("transfer_confirmed").await?;

        // Step 5: Update settlement records
        let result = self.finalize_settlement(
            request,
            confirmation,
            lock_id,
        ).await?;
        atomic_op.checkpoint("settlement_finalized").await?;

        Ok(result)
    }

    async fn validate_settlement(
        &self,
        request: &SettlementRequest,
    ) -> Result<(), SettlementError> {
        // Check account existence and status
        let from_account = self.get_nostro_account(&request.from_bank).await?;
        let to_account = self.get_vostro_account(&request.to_bank).await?;

        if !from_account.is_active || !to_account.is_active {
            return Err(SettlementError::InactiveAccount);
        }

        // Verify sufficient balance
        let balance = self.get_account_balance(
            &from_account,
            &request.currency,
        ).await?;

        if balance < request.amount {
            return Err(SettlementError::InsufficientFunds {
                required: request.amount,
                available: balance,
            });
        }

        // Check settlement windows
        if !self.is_settlement_window_open(&request.currency).await? {
            return Err(SettlementError::SettlementWindowClosed);
        }

        // Verify compliance clearance
        if !self.has_compliance_clearance(&request.obligation_id).await? {
            return Err(SettlementError::ComplianceBlocked);
        }

        Ok(())
    }

    async fn lock_funds(
        &self,
        bank: &str,
        amount: &Decimal,
        currency: &str,
    ) -> Result<Uuid, SettlementError> {
        let lock_id = Uuid::new_v4();

        // Create fund lock in database
        sqlx::query!(
            r#"
            INSERT INTO fund_locks (
                id, bank, amount, currency, locked_at, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            lock_id,
            bank,
            amount,
            currency,
            Utc::now(),
            Utc::now() + Duration::from_secs(600), // 10 minute expiry
        )
        .execute(&*self.db_pool)
        .await?;

        // Update available balance
        sqlx::query!(
            r#"
            UPDATE nostro_accounts
            SET available_balance = available_balance - $1
            WHERE bank = $2 AND currency = $3
            "#,
            amount,
            bank,
            currency,
        )
        .execute(&*self.db_pool)
        .await?;

        Ok(lock_id)
    }

    async fn initiate_external_transfer(
        &self,
        request: &SettlementRequest,
    ) -> Result<ExternalTransferRef, SettlementError> {
        // Select appropriate payment rail
        let payment_rail = self.select_payment_rail(request).await?;

        match payment_rail {
            PaymentRail::SWIFT => {
                self.initiate_swift_transfer(request).await
            }
            PaymentRail::SEPA => {
                self.initiate_sepa_transfer(request).await
            }
            PaymentRail::LocalACH => {
                self.initiate_local_transfer(request).await
            }
            PaymentRail::Mock => {
                // For MVP - simulate external transfer
                self.simulate_external_transfer(request).await
            }
        }
    }
}
```

### 2. Atomic Operation Controller

```rust
// src/settlement/atomic.rs
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct AtomicController {
    operations: Arc<RwLock<HashMap<Uuid, AtomicOperation>>>,
    db_pool: Arc<PgPool>,
}

#[derive(Debug, Clone)]
pub struct AtomicOperation {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub checkpoints: Vec<Checkpoint>,
    pub state: Arc<RwLock<AtomicState>>,
    db_pool: Arc<PgPool>,
}

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum AtomicState {
    InProgress,
    Committed,
    RolledBack,
}

impl AtomicController {
    pub async fn begin_operation(&self, id: Uuid) -> Result<AtomicOperation, Error> {
        let operation = AtomicOperation {
            id,
            started_at: Utc::now(),
            checkpoints: Vec::new(),
            state: Arc::new(RwLock::new(AtomicState::InProgress)),
            db_pool: self.db_pool.clone(),
        };

        // Register operation
        self.operations.write().await.insert(id, operation.clone());

        // Create audit log entry
        sqlx::query!(
            r#"
            INSERT INTO atomic_operations (id, type, started_at, state)
            VALUES ($1, 'settlement', $2, 'in_progress')
            "#,
            id,
            Utc::now(),
        )
        .execute(&*self.db_pool)
        .await?;

        Ok(operation)
    }
}

impl AtomicOperation {
    pub async fn checkpoint(&mut self, name: &str) -> Result<(), Error> {
        let checkpoint = Checkpoint {
            name: name.to_string(),
            timestamp: Utc::now(),
            data: serde_json::json!({}),
        };

        self.checkpoints.push(checkpoint.clone());

        // Persist checkpoint
        sqlx::query!(
            r#"
            INSERT INTO operation_checkpoints (
                operation_id, name, timestamp, data
            ) VALUES ($1, $2, $3, $4)
            "#,
            self.id,
            checkpoint.name,
            checkpoint.timestamp,
            checkpoint.data,
        )
        .execute(&*self.db_pool)
        .await?;

        Ok(())
    }

    pub async fn commit(&self) -> Result<(), Error> {
        let mut state = self.state.write().await;

        if !matches!(*state, AtomicState::InProgress) {
            return Err(Error::InvalidState);
        }

        *state = AtomicState::Committed;

        // Update database
        sqlx::query!(
            r#"
            UPDATE atomic_operations
            SET state = 'committed', completed_at = $1
            WHERE id = $2
            "#,
            Utc::now(),
            self.id,
        )
        .execute(&*self.db_pool)
        .await?;

        Ok(())
    }

    pub async fn rollback(&self) -> Result<(), Error> {
        let mut state = self.state.write().await;

        if !matches!(*state, AtomicState::InProgress) {
            return Err(Error::InvalidState);
        }

        *state = AtomicState::RolledBack;

        // Execute rollback for each checkpoint in reverse order
        for checkpoint in self.checkpoints.iter().rev() {
            self.rollback_checkpoint(checkpoint).await?;
        }

        // Update database
        sqlx::query!(
            r#"
            UPDATE atomic_operations
            SET state = 'rolled_back', completed_at = $1
            WHERE id = $2
            "#,
            Utc::now(),
            self.id,
        )
        .execute(&*self.db_pool)
        .await?;

        Ok(())
    }

    async fn rollback_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), Error> {
        match checkpoint.name.as_str() {
            "funds_locked" => {
                // Release fund lock
                sqlx::query!(
                    r#"
                    UPDATE fund_locks
                    SET released_at = $1
                    WHERE operation_id = $2
                    "#,
                    Utc::now(),
                    self.id,
                )
                .execute(&*self.db_pool)
                .await?;

                // Restore available balance
                sqlx::query!(
                    r#"
                    UPDATE nostro_accounts
                    SET available_balance = available_balance + (
                        SELECT amount FROM fund_locks
                        WHERE operation_id = $1
                    )
                    WHERE bank = (
                        SELECT bank FROM fund_locks
                        WHERE operation_id = $1
                    )
                    "#,
                    self.id,
                )
                .execute(&*self.db_pool)
                .await?;
            }
            "transfer_initiated" => {
                // Cancel external transfer if possible
                // This would involve calling the external API to cancel
            }
            _ => {
                // Log unhandled checkpoint rollback
            }
        }

        Ok(())
    }
}
```

### 3. Bank Integration Layer

```rust
// src/integration/mock.rs
use async_trait::async_trait;

#[async_trait]
pub trait BankClient: Send + Sync {
    async fn initiate_transfer(
        &self,
        request: &TransferRequest,
    ) -> Result<TransferReference, BankError>;

    async fn get_transfer_status(
        &self,
        reference: &TransferReference,
    ) -> Result<TransferStatus, BankError>;

    async fn cancel_transfer(
        &self,
        reference: &TransferReference,
    ) -> Result<(), BankError>;

    async fn get_account_balance(
        &self,
        account: &str,
        currency: &str,
    ) -> Result<Decimal, BankError>;
}

// Mock implementation for MVP
pub struct MockBankClient {
    latency: Duration,
    success_rate: f64,
}

#[async_trait]
impl BankClient for MockBankClient {
    async fn initiate_transfer(
        &self,
        request: &TransferRequest,
    ) -> Result<TransferReference, BankError> {
        // Simulate network latency
        tokio::time::sleep(self.latency).await;

        // Simulate success/failure based on configured rate
        if rand::random::<f64>() > self.success_rate {
            return Err(BankError::TransferFailed {
                reason: "Simulated failure".to_string(),
            });
        }

        // Generate mock reference
        let reference = TransferReference {
            id: Uuid::new_v4().to_string(),
            bank_reference: format!("MOCK-{}", Uuid::new_v4()),
            created_at: Utc::now(),
        };

        Ok(reference)
    }

    async fn get_transfer_status(
        &self,
        reference: &TransferReference,
    ) -> Result<TransferStatus, BankError> {
        // Simulate processing time
        tokio::time::sleep(self.latency / 2).await;

        // Mock status progression
        let elapsed = Utc::now() - reference.created_at;

        if elapsed < Duration::from_secs(10) {
            Ok(TransferStatus::Processing)
        } else if elapsed < Duration::from_secs(30) {
            Ok(TransferStatus::Completed {
                completed_at: Utc::now(),
                bank_confirmation: format!("CONF-{}", Uuid::new_v4()),
            })
        } else {
            Ok(TransferStatus::Failed {
                reason: "Timeout in mock processing".to_string(),
            })
        }
    }

    async fn cancel_transfer(
        &self,
        _reference: &TransferReference,
    ) -> Result<(), BankError> {
        tokio::time::sleep(self.latency / 3).await;
        Ok(())
    }

    async fn get_account_balance(
        &self,
        _account: &str,
        _currency: &str,
    ) -> Result<Decimal, BankError> {
        // Return mock balance
        Ok(Decimal::from(1_000_000))
    }
}
```

### 4. Reconciliation Engine

```rust
// src/accounts/reconciliation.rs
use chrono::{DateTime, Utc};

pub struct ReconciliationEngine {
    db_pool: Arc<PgPool>,
    bank_clients: Arc<BankClientManager>,
    event_publisher: Arc<EventPublisher>,
}

impl ReconciliationEngine {
    pub async fn reconcile_accounts(&self) -> Result<ReconciliationReport, Error> {
        let mut report = ReconciliationReport::new();

        // Get all active nostro/vostro accounts
        let accounts = self.get_active_accounts().await?;

        for account in accounts {
            let result = self.reconcile_account(&account).await?;
            report.add_account_result(result);
        }

        // Store reconciliation report
        self.store_report(&report).await?;

        // Publish reconciliation events
        if report.has_discrepancies() {
            self.event_publisher.publish(
                "settlement.reconciliation.discrepancy",
                &report,
            ).await?;
        }

        Ok(report)
    }

    async fn reconcile_account(
        &self,
        account: &Account,
    ) -> Result<AccountReconciliation, Error> {
        // Get internal balance
        let internal_balance = self.get_internal_balance(account).await?;

        // Get external balance from bank
        let external_balance = self.get_external_balance(account).await?;

        // Calculate discrepancy
        let discrepancy = internal_balance - external_balance;

        let mut reconciliation = AccountReconciliation {
            account_id: account.id,
            account_name: account.name.clone(),
            internal_balance,
            external_balance,
            discrepancy,
            status: ReconciliationStatus::Pending,
            transactions: Vec::new(),
        };

        if discrepancy.abs() > Decimal::from(0.01) {
            // Investigate discrepancy
            reconciliation.transactions = self.find_unmatched_transactions(
                account,
                internal_balance,
                external_balance,
            ).await?;

            if reconciliation.transactions.is_empty() {
                reconciliation.status = ReconciliationStatus::Unresolved;
            } else {
                reconciliation.status = ReconciliationStatus::Identified;
            }
        } else {
            reconciliation.status = ReconciliationStatus::Balanced;
        }

        Ok(reconciliation)
    }

    async fn find_unmatched_transactions(
        &self,
        account: &Account,
        internal_balance: Decimal,
        external_balance: Decimal,
    ) -> Result<Vec<UnmatchedTransaction>, Error> {
        // Query recent transactions
        let internal_txns = sqlx::query_as!(
            Transaction,
            r#"
            SELECT * FROM settlement_transactions
            WHERE account_id = $1
                AND created_at > NOW() - INTERVAL '7 days'
            ORDER BY created_at DESC
            "#,
            account.id,
        )
        .fetch_all(&*self.db_pool)
        .await?;

        // Get external transactions from bank
        let external_txns = self.bank_clients
            .get_client(&account.bank)
            .get_transactions(account, 7)
            .await?;

        // Match transactions
        let unmatched = self.match_transactions(internal_txns, external_txns)?;

        Ok(unmatched)
    }
}
```

### 5. gRPC Service Definition

```protobuf
// proto/settlement.proto
syntax = "proto3";

package settlement;

service SettlementService {
    // Execute settlement for an obligation
    rpc ExecuteSettlement(SettlementRequest) returns (SettlementResponse);

    // Get settlement status
    rpc GetSettlementStatus(SettlementStatusRequest) returns (SettlementStatusResponse);

    // Reconcile accounts
    rpc ReconcileAccounts(ReconcileRequest) returns (ReconcileResponse);

    // Stream settlement events
    rpc StreamSettlementEvents(StreamRequest) returns (stream SettlementEvent);
}

message SettlementRequest {
    string obligation_id = 1;
    string from_bank = 2;
    string to_bank = 3;
    string amount = 4;
    string currency = 5;
    int64 settlement_date = 6;
    Priority priority = 7;
    map<string, string> metadata = 8;
}

message SettlementResponse {
    string settlement_id = 1;
    SettlementStatus status = 2;
    string reference = 3;
    int64 completed_at = 4;
    string confirmation_code = 5;
}

enum SettlementStatus {
    PENDING = 0;
    VALIDATING = 1;
    EXECUTING = 2;
    COMPLETED = 3;
    FAILED = 4;
    ROLLED_BACK = 5;
}

enum Priority {
    NORMAL = 0;
    HIGH = 1;
    URGENT = 2;
}

message SettlementEvent {
    string event_id = 1;
    string settlement_id = 2;
    string event_type = 3;
    int64 timestamp = 4;
    google.protobuf.Any payload = 5;
}
```

### 6. Database Schema

```sql
-- Settlement transactions
CREATE TABLE settlement_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    obligation_id UUID NOT NULL,
    from_bank VARCHAR(20) NOT NULL,
    to_bank VARCHAR(20) NOT NULL,
    amount DECIMAL(20, 2) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    priority VARCHAR(10) DEFAULT 'normal',

    -- External references
    external_reference VARCHAR(100),
    bank_confirmation VARCHAR(100),

    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    validated_at TIMESTAMP,
    executed_at TIMESTAMP,
    completed_at TIMESTAMP,
    failed_at TIMESTAMP,

    -- Error tracking
    error_message TEXT,
    retry_count INT DEFAULT 0,

    -- Metadata
    metadata JSONB DEFAULT '{}'
);

CREATE INDEX idx_settlements_obligation ON settlement_transactions(obligation_id);
CREATE INDEX idx_settlements_status ON settlement_transactions(status);
CREATE INDEX idx_settlements_created ON settlement_transactions(created_at DESC);

-- Nostro accounts (our accounts at other banks)
CREATE TABLE nostro_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank VARCHAR(20) NOT NULL,
    account_number VARCHAR(50) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    balance DECIMAL(20, 2) NOT NULL DEFAULT 0,
    available_balance DECIMAL(20, 2) NOT NULL DEFAULT 0,
    last_reconciled TIMESTAMP,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(bank, currency)
);

-- Vostro accounts (other banks' accounts with us)
CREATE TABLE vostro_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank VARCHAR(20) NOT NULL,
    account_number VARCHAR(50) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    balance DECIMAL(20, 2) NOT NULL DEFAULT 0,
    credit_limit DECIMAL(20, 2),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(bank, currency)
);

-- Fund locks for atomic operations
CREATE TABLE fund_locks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    operation_id UUID NOT NULL,
    bank VARCHAR(20) NOT NULL,
    amount DECIMAL(20, 2) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    locked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    released_at TIMESTAMP,
    INDEX idx_locks_operation (operation_id),
    INDEX idx_locks_expires (expires_at)
);

-- Atomic operations tracking
CREATE TABLE atomic_operations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type VARCHAR(50) NOT NULL,
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    state VARCHAR(20) NOT NULL DEFAULT 'in_progress',
    rollback_reason TEXT
);

-- Operation checkpoints for rollback
CREATE TABLE operation_checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    operation_id UUID REFERENCES atomic_operations(id),
    name VARCHAR(50) NOT NULL,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    data JSONB,
    INDEX idx_checkpoints_operation (operation_id)
);

-- Reconciliation reports
CREATE TABLE reconciliation_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_date DATE NOT NULL,
    total_accounts INT NOT NULL,
    balanced_accounts INT NOT NULL,
    discrepancy_accounts INT NOT NULL,
    total_discrepancy DECIMAL(20, 2),
    details JSONB,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(report_date)
);

-- Settlement windows
CREATE TABLE settlement_windows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    currency VARCHAR(3) NOT NULL,
    window_start TIME NOT NULL,
    window_end TIME NOT NULL,
    days_of_week INT[] DEFAULT ARRAY[1,2,3,4,5], -- Mon-Fri
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(currency, window_start)
);

-- Initialize settlement windows
INSERT INTO settlement_windows (currency, window_start, window_end) VALUES
    ('USD', '14:00', '22:00'),  -- US business hours
    ('EUR', '08:00', '16:00'),  -- European business hours
    ('GBP', '08:00', '16:00'),  -- UK business hours
    ('AED', '08:00', '16:00'),  -- UAE business hours
    ('INR', '09:00', '17:00');  -- India business hours
```

### 7. Configuration

```toml
# Config.toml
[server]
grpc_port = 50056
http_port = 8086
max_concurrent_settlements = 100

[database]
url = "postgresql://deltran:password@postgres:5432/deltran"
max_connections = 20
min_connections = 5

[nats]
url = "nats://nats:4222"
subject_prefix = "settlement"
stream = "settlement-events"

[settlement]
default_timeout_seconds = 300
max_retry_attempts = 3
retry_delay_seconds = 60

[reconciliation]
schedule = "0 */6 * * *"  # Every 6 hours
tolerance_amount = 0.01
alert_threshold = 1000.00

[banks]
mock_enabled = true
mock_latency_ms = 500
mock_success_rate = 0.95

[[banks.swift]]
enabled = true
endpoint = "https://swift-api.example.com"
timeout_seconds = 60

[[banks.sepa]]
enabled = true
endpoint = "https://sepa-api.example.com"
timeout_seconds = 45

[monitoring]
prometheus_enabled = true
metrics_port = 9096
health_check_interval = 30
```

### 8. Docker Configuration

```dockerfile
# Dockerfile
FROM rust:1.74 as builder

WORKDIR /usr/src/settlement-engine
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY . .
RUN touch src/main.rs
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/settlement-engine/target/release/settlement-engine /usr/local/bin/
COPY config.toml /etc/settlement-engine/

EXPOSE 50056 8086

CMD ["settlement-engine"]
```

## Integration Points

### 1. Upstream Dependencies
- Clearing Engine: Receives settlement instructions from clearing
- Compliance Engine: Validates compliance before settlement
- Risk Engine: Checks risk limits and exposure

### 2. Downstream Services
- Notification Engine: Sends settlement confirmations
- Reporting Engine: Provides settlement data for reports
- Gateway: Returns settlement status to clients

### 3. External Systems
- Bank APIs: SWIFT, SEPA, local payment systems
- Central banks: For regulatory reporting
- Correspondent banks: For nostro/vostro management

## Performance Requirements

- Settlement latency: < 5 seconds for domestic, < 30 seconds for international
- Throughput: 1,000 settlements/minute
- Reconciliation: < 5 minutes for 10,000 transactions
- Rollback time: < 2 seconds
- Recovery time: < 10 seconds
- Database consistency: ACID compliance

## Security Considerations

1. **Financial Security**
   - Atomic operations with automatic rollback
   - Fund locking to prevent double spending
   - Reconciliation to detect discrepancies

2. **Access Control**
   - mTLS for bank API communication
   - Service-to-service authentication
   - Audit logging for all operations

3. **Data Protection**
   - Encryption of sensitive financial data
   - Secure storage of bank credentials
   - PCI DSS compliance for card settlements

## Monitoring & Metrics

### Key Metrics
```prometheus
# Settlement metrics
settlement_total{status="completed|failed|rolled_back"}
settlement_duration_seconds
settlement_amount_total{currency}

# Reconciliation metrics
reconciliation_discrepancies_total
reconciliation_discrepancy_amount{currency}

# Atomic operation metrics
atomic_operations_total{state="committed|rolled_back"}
atomic_rollback_duration_seconds

# Account metrics
nostro_balance{bank,currency}
vostro_balance{bank,currency}
```

## Testing Strategy

1. **Unit Tests**
   - Atomic operation control
   - Rollback mechanisms
   - Settlement validation

2. **Integration Tests**
   - Bank API integration (mocked)
   - Database transactions
   - Event publishing

3. **Failure Scenario Tests**
   - Network failures during settlement
   - Partial settlement failures
   - Rollback verification
   - Recovery from crashes

## Deployment Checklist

- [ ] Configure database with strong consistency
- [ ] Set up nostro/vostro accounts
- [ ] Configure settlement windows
- [ ] Set up bank API credentials (or mock)
- [ ] Configure atomic operation timeouts
- [ ] Set up monitoring and alerts
- [ ] Test rollback mechanisms
- [ ] Verify reconciliation process
- [ ] Load test with concurrent settlements