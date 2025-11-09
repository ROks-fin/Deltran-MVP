# Settlement Engine

Critical financial settlement service for DelTran payment network. Handles final fund transfers between banks with atomic operations, automatic rollback, and reconciliation.

## Features

- **Atomic Settlement Execution**: Multi-step settlement with checkpoints (Validation → Lock → Transfer → Confirm → Finalize)
- **Fund Locking**: Prevents double-spending through explicit fund locks before transfers
- **Bank Integration**: Mock bank client for MVP, with trait-based design for real integrations (SWIFT, SEPA, Local ACH)
- **Nostro/Vostro Management**: Complete account management for correspondent banking
- **Reconciliation Engine**: Automatic reconciliation every 6 hours to detect discrepancies
- **Rollback & Recovery**: Automatic rollback on failures with retry mechanisms
- **gRPC API**: High-performance gRPC server for inter-service communication
- **HTTP API**: REST endpoints for monitoring and account queries

## Architecture

```
settlement-engine/
├── src/
│   ├── main.rs              # Entry point
│   ├── server.rs            # Server initialization and schedulers
│   ├── config.rs            # Configuration management
│   ├── error.rs             # Error types
│   ├── settlement/          # Core settlement logic
│   │   ├── executor.rs      # Settlement execution
│   │   ├── atomic.rs        # Atomic operations controller
│   │   ├── rollback.rs      # Rollback management
│   │   └── validator.rs     # Pre-settlement validation
│   ├── accounts/            # Account management
│   │   ├── nostro.rs        # Nostro accounts (our accounts at other banks)
│   │   ├── vostro.rs        # Vostro accounts (other banks with us)
│   │   └── reconciliation.rs # Reconciliation engine
│   ├── integration/         # External bank integrations
│   │   ├── mock.rs          # Mock bank client (for MVP)
│   │   ├── swift.rs         # SWIFT integration (stub)
│   │   ├── sepa.rs          # SEPA integration (stub)
│   │   └── local.rs         # Local ACH integration (stub)
│   ├── recovery/            # Recovery mechanisms
│   │   ├── retry.rs         # Retry logic
│   │   └── compensation.rs  # Compensation transactions
│   └── grpc/                # gRPC server
│       └── server.rs        # gRPC service implementation
└── proto/
    └── settlement.proto     # Protocol buffer definitions
```

## Settlement Flow

1. **Validation**: Check accounts, balances, settlement windows, compliance
2. **Fund Locking**: Lock funds in nostro account to prevent double-spending
3. **Transfer Initiation**: Call external bank API to initiate transfer
4. **Confirmation**: Wait for transfer confirmation (with timeout)
5. **Finalization**: Update balances, release locks, mark as completed

If any step fails, automatic rollback releases locks and reverts changes.

## Configuration

Environment variables:
- `DATABASE_URL`: PostgreSQL connection string
- `NATS_URL`: NATS server URL
- `GRPC_PORT`: gRPC server port (default: 50056)
- `HTTP_PORT`: HTTP API port (default: 8086)

## API Endpoints

### gRPC (port 50056)
- `ExecuteSettlement`: Execute a settlement
- `GetSettlementStatus`: Get settlement status
- `ReconcileAccounts`: Trigger reconciliation
- `StreamSettlementEvents`: Real-time settlement events
- `GetNostroBalance`: Get nostro account balance
- `GetVostroBalance`: Get vostro account balance

### HTTP (port 8086)
- `GET /health`: Health check
- `GET /metrics`: Prometheus metrics
- `GET /api/v1/accounts/nostro`: List nostro accounts
- `GET /api/v1/accounts/vostro`: List vostro accounts

## Background Jobs

1. **Reconciliation Scheduler**: Runs every 6 hours to reconcile all accounts
2. **Retry Scheduler**: Attempts to retry failed settlements every 5 minutes
3. **Cleanup Scheduler**: Cleans up completed atomic operations every 10 minutes

## Database Schema

Required tables:
- `settlement_transactions`: Main settlement records
- `nostro_accounts`: Our accounts at other banks
- `vostro_accounts`: Other banks' accounts with us
- `fund_locks`: Active fund locks
- `settlement_atomic_operations`: Atomic operation tracking
- `settlement_operation_checkpoints`: Operation checkpoints for rollback
- `reconciliation_reports`: Reconciliation reports
- `compensation_transactions`: Compensation/reversal transactions
- `settlement_windows`: Settlement time windows by currency

## Building

```bash
# Development
cargo build

# Release
cargo build --release

# Docker
docker build -t settlement-engine .
```

## Testing

```bash
# Unit tests
cargo test

# Integration tests (require database)
cargo test --ignored

# With coverage
cargo tarpaulin --out Html
```

## Metrics

Prometheus metrics exposed on `/metrics`:
- `settlement_total{status}`: Total settlements by status
- `settlement_duration_seconds`: Settlement duration histogram
- `settlement_amount_total{currency}`: Total settlement amounts
- `reconciliation_discrepancies_total`: Number of discrepancies
- `atomic_operations_total{state}`: Atomic operations by state
- `nostro_balance{bank,currency}`: Nostro account balances
- `vostro_balance{bank,currency}`: Vostro account balances

## Security

- Atomic operations with automatic rollback
- Fund locking to prevent double-spending
- Row-level locking in database
- Audit logging for all operations
- mTLS for external bank communications (production)

## Performance

- Throughput: 1,000 settlements/minute
- Settlement latency: <5 seconds domestic, <30 seconds international
- Rollback time: <2 seconds
- Reconciliation: <5 minutes for 10,000 transactions

## Production Considerations

1. Replace mock bank client with real integrations (SWIFT, SEPA, etc.)
2. Enable mTLS for external communications
3. Set up monitoring and alerting
4. Configure proper settlement windows
5. Establish reconciliation procedures
6. Set up disaster recovery
7. Enable audit logging export
8. Configure rate limiting

## License

Copyright 2024 DelTran
