# DelTran Gateway Service

**Production-ready ISO 20022 Gateway** for the DelTran cross-border payment protocol.

## Overview

The Gateway Service is the entry/exit point for all ISO 20022 messages in the DelTran system. It:

- ✅ Receives ISO 20022 XML messages (pain.001, pacs.008, camt.054, etc.)
- ✅ Parses and validates messages
- ✅ Converts to canonical internal format
- ✅ Routes to appropriate microservices via NATS
- ✅ Persists all transactions to PostgreSQL
- ✅ Provides payment status queries

## Supported Messages

### P0 (Critical - Implemented)

| Message | Description | Status |
|---------|-------------|--------|
| **pain.001** | Customer Credit Transfer Initiation | ✅ Complete |
| **pacs.008** | FI to FI Customer Credit Transfer | ✅ Complete |
| **camt.054** | Bank to Customer Debit/Credit Notification (FUNDING) | ✅ Complete |

### P1 (High Priority - Pending)

| Message | Description | Status |
|---------|-------------|--------|
| **pacs.002** | FI to FI Payment Status Report | ⏳ Pending |
| **pain.002** | Customer Payment Status Report | ⏳ Pending |
| **camt.053** | Bank to Customer Statement | ⏳ Pending |

## Architecture

```
┌─────────────────┐
│   Banks (ISO)   │
└────────┬────────┘
         │ pain.001, pacs.008, camt.054
         ▼
┌─────────────────────────────────┐
│      Gateway Service (Rust)      │
│  - HTTP Server (Axum)            │
│  - ISO 20022 Parsers             │
│  - Canonical Model Converter     │
│  - PostgreSQL Persistence        │
│  - NATS Publisher                │
└─────────┬───────────────────────┘
          │ NATS Events
          ▼
┌────────────────────────────────────┐
│    DelTran Microservices           │
│  - Obligation Engine               │
│  - Risk Engine                     │
│  - Token Engine                    │
│  - Clearing Engine                 │
│  - Settlement Engine               │
│  - Notification Engine             │
└────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- Rust 1.75+
- PostgreSQL 15+
- NATS Server with JetStream
- Docker & Docker Compose (optional)

### Local Development

```bash
# 1. Start dependencies
docker-compose up -d gateway-db nats

# 2. Run migrations
cargo install sqlx-cli
sqlx migrate run

# 3. Run Gateway
cargo run

# 4. Test health endpoint
curl http://localhost:8080/health
```

### Docker Deployment

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f gateway

# Stop all services
docker-compose down
```

## API Endpoints

### ISO 20022 Message Submission

```bash
# Submit pain.001 (Payment Initiation)
curl -X POST http://localhost:8080/iso20022/pain.001 \
  -H "Content-Type: application/xml" \
  --data @sample_pain001.xml

# Submit pacs.008 (Settlement Instruction)
curl -X POST http://localhost:8080/iso20022/pacs.008 \
  -H "Content-Type: application/xml" \
  --data @sample_pacs008.xml

# Submit camt.054 (Funding Notification - CRITICAL!)
curl -X POST http://localhost:8080/iso20022/camt.054 \
  -H "Content-Type: application/xml" \
  --data @sample_camt054.xml
```

### Payment Status Query

```bash
# Get payment by DelTran TX ID
curl http://localhost:8080/payment/{deltran_tx_id}
```

### Health Check

```bash
curl http://localhost:8080/health
```

## Configuration

Environment variables:

```bash
# Database
DATABASE_URL=postgres://deltran:deltran@localhost:5432/deltran_gateway

# NATS
NATS_URL=nats://localhost:4222

# Server
BIND_ADDR=0.0.0.0:8080

# Logging
RUST_LOG=info,deltran_gateway=debug
```

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
# Requires running Gateway, DB, and NATS
cargo test --test integration_tests
```

### Load Testing

See [stress-tests/README.md](../../stress-tests/README.md) for K6 load tests.

```bash
cd ../../stress-tests

# Pain.001 load test (200 TPS)
..\..\k6-v0.49.0-windows-amd64\k6.exe run pain001_load_test.js

# End-to-end flow test
..\..\k6-v0.49.0-windows-amd64\k6.exe run end_to_end_flow_test.js
```

## Database Schema

### `payments` Table

Stores all payments in canonical format:

```sql
CREATE TABLE payments (
    deltran_tx_id UUID PRIMARY KEY,
    obligation_id UUID,
    uetr UUID,
    end_to_end_id VARCHAR(35) NOT NULL,
    instruction_id VARCHAR(35) NOT NULL,
    instructed_amount DECIMAL(18, 5) NOT NULL,
    settlement_amount DECIMAL(18, 5) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    debtor_name VARCHAR(140),
    creditor_name VARCHAR(140),
    debtor_agent_bic VARCHAR(11),
    creditor_agent_bic VARCHAR(11),
    status VARCHAR(50) NOT NULL,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    -- ... more fields
);
```

### `payment_events` Table

Audit trail of all payment state changes.

## NATS Topics

Messages published to NATS:

| Topic | Description | Consumers |
|-------|-------------|-----------|
| `deltran.obligation.create` | New payment obligation | Obligation Engine |
| `deltran.risk.check` | Risk/compliance check | Risk Engine |
| `deltran.token.mint` | Token minting request | Token Engine |
| `deltran.clearing.submit` | Submit for clearing | Clearing Engine |
| `deltran.settlement.execute` | Execute settlement | Settlement Engine |
| `deltran.notification.*` | Send notifications | Notification Engine |
| `deltran.reporting.payment` | Analytics/reporting | Reporting Engine |

## Performance Targets

For pilot deployment:

- **Throughput**: 200 TPS sustained, 500 TPS spike
- **Latency**: p95 < 500ms, p99 < 1000ms
- **Availability**: 99.9% uptime
- **Error Rate**: < 0.1%

## Security

- ✅ No secrets in code (environment variables only)
- ✅ Database connection pooling
- ✅ Input validation on all ISO messages
- ✅ SQL injection protection (sqlx compile-time checks)
- ✅ Non-root Docker container
- ⏳ TODO: API authentication/authorization
- ⏳ TODO: TLS/HTTPS
- ⏳ TODO: Rate limiting

## Monitoring

### Prometheus Metrics

Exposed at `/metrics` (TODO):
- `deltran_payments_total` - Total payments processed
- `deltran_payment_duration_seconds` - Payment processing duration
- `deltran_iso_parse_errors_total` - ISO parsing errors
- `deltran_db_operations_total` - Database operations

### Logging

Structured logging with `tracing`:
```rust
RUST_LOG=info,deltran_gateway=debug
```

## Troubleshooting

### Database Connection Errors

```bash
# Check PostgreSQL is running
docker-compose ps gateway-db

# Test connection
psql -h localhost -U deltran -d deltran_gateway
```

### NATS Connection Errors

```bash
# Check NATS is running
docker-compose ps nats

# Monitor NATS
curl http://localhost:8222/varz
```

### Parsing Errors

Check logs for detailed error messages:
```bash
docker-compose logs gateway | grep ERROR
```

## Development Roadmap

### Phase 1: Core Messages (Complete)
- [x] pain.001 parser
- [x] pacs.008 parser
- [x] camt.054 parser (CRITICAL)
- [x] Database persistence
- [x] NATS routing
- [x] Docker deployment

### Phase 2: Additional Messages (In Progress)
- [ ] pacs.002 parser
- [ ] pain.002 parser
- [ ] camt.053 parser
- [ ] Message validation
- [ ] Error handling improvements

### Phase 3: Production Readiness (Pending)
- [ ] Authentication/authorization
- [ ] TLS/HTTPS
- [ ] Rate limiting
- [ ] Prometheus metrics
- [ ] Distributed tracing
- [ ] Circuit breakers

### Phase 4: Optimization (Pending)
- [ ] Message batching
- [ ] Async processing
- [ ] Caching layer
- [ ] Load balancing
- [ ] Horizontal scaling

## Contributing

1. Run tests: `cargo test`
2. Format code: `cargo fmt`
3. Lint: `cargo clippy`
4. Update documentation

## License

Proprietary - DelTran Protocol

## Support

For issues, contact the DelTran development team.
