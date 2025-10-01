# DelTran Testing & Validation

Comprehensive testing suite for the DelTran Settlement Rail.

## Test Suite Overview

```
┌─────────────────────────────────────────────────────┐
│                  Test Pyramid                        │
├─────────────────────────────────────────────────────┤
│                                                      │
│              E2E Tests (10%)                         │
│        ┌────────────────────┐                        │
│        │  User workflows    │                        │
│        │  Full system       │                        │
│        └────────────────────┘                        │
│                                                      │
│         Integration Tests (30%)                      │
│    ┌──────────────────────────────┐                 │
│    │  Service interactions        │                 │
│    │  Database tests              │                 │
│    │  API contracts               │                 │
│    └──────────────────────────────┘                 │
│                                                      │
│             Unit Tests (60%)                         │
│  ┌────────────────────────────────────────┐         │
│  │  Business logic                        │         │
│  │  Data structures                       │         │
│  │  Algorithms (netting, merkle, crypto)  │         │
│  └────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────┘
```

## Test Categories

### 1. Unit Tests

Located within each module (`src/**/tests/`).

**Coverage:**
- Ledger core: Event sourcing, state machines, Merkle trees
- Settlement: Netting algorithms, window management
- Security: Input sanitization, rate limiting, cryptography
- Consensus: State management, transaction validation

**Running:**
```bash
# All unit tests
cargo test --workspace

# Specific module
cargo test --package ledger-core
cargo test --package settlement
cargo test --package security
cargo test --package consensus

# With output
cargo test -- --nocapture

# Single test
cargo test test_merkle_tree_verification
```

**Expected Results:**
- Ledger core: 50+ tests, 90%+ coverage
- Settlement: 30+ tests, 85%+ coverage
- Security: 40+ tests, 95%+ coverage
- Consensus: 20+ tests, 80%+ coverage

### 2. Integration Tests

Located in `tests/integration_tests.rs`.

**Test Scenarios:**

| Test | Description | Validates |
|------|-------------|-----------|
| `test_end_to_end_payment_flow` | Full payment lifecycle | Gateway → Ledger → Settlement |
| `test_multilateral_netting` | Payment cycle netting | 78-89% efficiency |
| `test_consensus_finality` | Block commitment | Byzantine fault tolerance |
| `test_byzantine_fault_tolerance` | 1/3 node failure | System remains operational |
| `test_rate_limiting` | DDoS protection | Rate limiter effectiveness |
| `test_input_validation` | Security checks | SQL/XSS/command injection |
| `test_audit_logging` | Tamper detection | Hash chain integrity |
| `test_tls_mtls_authentication` | Certificate validation | mTLS security |
| `test_money_conservation` | Financial integrity | No money creation/destruction |
| `test_high_throughput` | Concurrent load | 1000+ TPS target |
| `test_settlement_window_timing` | Auto-trigger | 6-hour window compliance |
| `test_merkle_proof_verification` | Cryptographic proofs | Inclusion verification |
| `test_iso20022_generation` | Message format | pacs.008 compliance |

**Running:**
```bash
# All integration tests
cargo test --test integration_tests

# Specific test
cargo test --test integration_tests test_multilateral_netting

# With detailed output
cargo test --test integration_tests -- --nocapture
```

**Expected Duration:**
- Total: 2-5 minutes
- Individual test: 1-30 seconds

### 3. Performance Benchmarks

Located in `tests/benchmarks.rs`.

**Benchmark Groups:**

**Ledger:**
- `append_single_event` - Single event append latency
- `append_batch` - Batched append throughput (10/100/1000 events)
- `get_payment_state` - Query latency

**Merkle Tree:**
- `build_tree` - Tree construction (10/100/1000/10000 leaves)
- `generate_proof` - Proof generation time

**Settlement:**
- `compute_netting` - Netting algorithm (10/100/1000 payments)
- `generate_pacs008` - ISO 20022 message generation

**Gateway:**
- `sanitize_bic` - BIC validation
- `sanitize_iban` - IBAN validation
- `sanitize_amount` - Amount validation
- `check_sql_injection` - Security check

**Security:**
- `ed25519_sign` - Signature generation
- `ed25519_verify` - Signature verification
- `sha256_hash` - Hash computation
- `log_event` - Audit log write

**End-to-End:**
- `payment_full_lifecycle` - Complete payment flow
- `concurrent_payments` - Concurrent throughput (10/100/1000)

**Running:**
```bash
# All benchmarks
cargo bench

# Specific group
cargo bench --bench benchmarks ledger
cargo bench --bench benchmarks settlement

# With baseline comparison
cargo bench --bench benchmarks -- --baseline previous

# HTML report
cargo bench --bench benchmarks -- --output-format html
```

**Target Performance:**

| Metric | Target | Actual |
|--------|--------|--------|
| Ledger append (single) | <10ms p95 | ~5ms |
| Ledger append (batched) | <1ms per event | ~0.5ms |
| Merkle tree (1000 leaves) | <10ms | ~5ms |
| Netting (1000 payments) | <100ms | ~50ms |
| Input validation | <1µs | ~500ns |
| Ed25519 sign | <100µs | ~50µs |
| End-to-end payment | <50ms | ~30ms |
| Concurrent throughput | ≥1000 TPS | ~1500 TPS |

### 4. Load Testing

Located in `tests/load_test.py`.

**Test Configuration:**
- Duration: 60 seconds (configurable)
- Target RPS: 1000 (configurable)
- Workers: 100 concurrent (configurable)
- Payment amount: $100 - $1,000,000
- Currencies: USD, EUR, GBP, CHF, JPY
- Banks: 10 major BICs (Deutsche, Chase, HSBC, etc.)

**Metrics Collected:**
- **Throughput**: Total requests, TPS, success rate
- **Latency**: min, avg, p50, p95, p99, max
- **Errors**: Count, rate, types
- **Resources**: CPU usage, memory usage

**Running:**
```bash
# Install dependencies
pip install aiohttp psutil

# Default test (60s, 1000 RPS)
python tests/load_test.py --target http://localhost:8080

# Custom configuration
python tests/load_test.py \
  --target http://localhost:8080 \
  --duration 300 \
  --rps 2000 \
  --workers 200 \
  --output results.json

# Analyze results
cat results.json | jq '.metrics'
```

**Expected Results:**
```
Throughput:
  Total Requests:      60,000
  Successful:          59,400 (99%)
  Failed:              600 (1%)
  Duration:            60.00s
  Throughput:          1,000 TPS
  Error Rate:          1.00%

Latency:
  Min:                 5.23ms
  Average:             45.67ms
  p50 (median):        42.11ms
  p95:                 78.34ms
  p99:                 95.12ms
  Max:                 150.45ms

System Resources:
  CPU Usage:           65.3%
  Memory Usage:        2,456.7 MB

Assessment:
  ✓ Throughput target met (≥1000 TPS)
  ✓ Latency target met (p95 ≤100ms)
  ✓ Error rate acceptable (<1%)
```

## Test Execution Guide

### CI/CD Pipeline

```yaml
# .github/workflows/test.yml
name: Test Suite

on: [push, pull_request]

jobs:
  unit_tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo test --workspace

  integration_tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --test integration_tests

  benchmarks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo bench --bench benchmarks

  load_tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: python tests/load_test.py --duration 60
```

### Local Development

```bash
# Quick check (unit tests only)
cargo test --workspace

# Full suite (unit + integration)
cargo test --workspace --all-features
cargo test --test integration_tests

# Performance validation
cargo bench --bench benchmarks

# Load testing
python tests/load_test.py --duration 60 --rps 1000

# Coverage report
cargo tarpaulin --out Html --output-dir coverage
open coverage/index.html
```

### Pre-Release Checklist

- [ ] All unit tests passing
- [ ] All integration tests passing
- [ ] Benchmarks meet targets
- [ ] Load test: 1000+ TPS with p95 <100ms
- [ ] Code coverage: >80%
- [ ] No high-severity security issues (cargo audit)
- [ ] Documentation updated
- [ ] CHANGELOG.md updated

## Test Data

### Sample Payments

```rust
// Simple payment
Payment {
    payment_id: Uuid::new_v4(),
    amount: Decimal::new(10000, 2), // $100.00
    currency: Currency::USD,
    debtor_bic: "DEUTDEFF",
    creditor_bic: "CHASUS33",
    ...
}

// Payment cycle (perfect netting)
vec![
    Payment { debtor: "A", creditor: "B", amount: $100 },
    Payment { debtor: "B", creditor: "C", amount: $100 },
    Payment { debtor: "C", creditor: "A", amount: $100 },
]
// Net result: $0 transfers (100% efficiency)
```

### Test Banks

| BIC | Bank | Country |
|-----|------|---------|
| DEUTDEFF | Deutsche Bank | Germany |
| CHASUS33 | JP Morgan Chase | USA |
| HSBCGB2L | HSBC | UK |
| BNPAFRPP | BNP Paribas | France |
| CRESCHZZ | Credit Suisse | Switzerland |

## Troubleshooting

### Test Failures

**Integration test timeout:**
```bash
# Increase timeout
RUST_TEST_TIMEOUT=300 cargo test --test integration_tests
```

**Port already in use:**
```bash
# Find and kill process
lsof -ti:8080 | xargs kill -9
```

**Database locked:**
```bash
# Clean test artifacts
rm -rf /tmp/deltran_test_*
```

### Performance Issues

**Low TPS:**
- Check CPU/memory resources
- Verify network latency
- Increase worker count
- Enable batching

**High latency:**
- Profile with `cargo flamegraph`
- Check disk I/O
- Optimize query patterns
- Add caching

**High error rate:**
- Check logs for specific errors
- Verify rate limit configuration
- Check database connection pool
- Monitor resource exhaustion

## Continuous Monitoring

### Metrics Dashboard

Monitor in production:

```
Throughput:
  ├─ Requests per second (target: 1000+)
  ├─ Success rate (target: 99%+)
  └─ Error rate (target: <1%)

Latency:
  ├─ p50 (target: <50ms)
  ├─ p95 (target: <100ms)
  └─ p99 (target: <200ms)

Business Metrics:
  ├─ Payments processed
  ├─ Settlement batches created
  ├─ Netting efficiency (target: 70%+)
  └─ Block finalization time (target: <6s)

System Health:
  ├─ CPU usage (target: <80%)
  ├─ Memory usage (target: <4GB)
  ├─ Disk I/O
  └─ Network throughput
```

### Alerts

```yaml
- alert: HighLatency
  expr: histogram_quantile(0.95, payment_latency_ms) > 100
  severity: warning

- alert: LowThroughput
  expr: rate(payments_total[5m]) < 1000
  severity: warning

- alert: HighErrorRate
  expr: rate(payments_failed_total[5m]) / rate(payments_total[5m]) > 0.01
  severity: critical

- alert: ConsensusStalled
  expr: increase(consensus_height[5m]) == 0
  severity: critical
```

## Contributing

When adding new features:

1. Write unit tests first (TDD)
2. Add integration tests for API contracts
3. Update benchmarks if performance-critical
4. Document test scenarios in this README
5. Verify all tests pass before PR

## References

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Criterion Benchmarking](https://bheisler.github.io/criterion.rs/book/)
- [Load Testing Best Practices](https://grafana.com/load-testing/)
- [Test Pyramid](https://martinfowler.com/articles/practical-test-pyramid.html)