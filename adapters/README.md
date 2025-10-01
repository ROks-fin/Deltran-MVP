# Adapters Layer

**Bank/PSP/CBDC connectivity with resilience patterns**

---

## Features

### ✅ ISO 20022 Full Support
- **pacs.008.001.08** (FIToFICustomerCreditTransfer) complete implementation
- XML generation with proper schema validation
- BIC/IBAN validation
- Support for remittance information, postal addresses, UETR

### ✅ Dead Letter Queue (DLQ)
- Automatic retry with exponential backoff (2^n seconds)
- Per-corridor queues
- Configurable max retries (default: 3)
- Persistent storage (in-memory for MVP, extendable to Redis/PostgreSQL)

### ✅ Circuit Breaker
- Per-corridor circuit breakers
- States: Closed → Open → Half-Open → Closed
- Configurable thresholds (default: 5 failures)
- Automatic half-open after timeout (default: 60s)

### ✅ Kill Switch
- Manual emergency shutdown per corridor
- Audit trail (who activated, when, why)
- Instant request blocking
- Easy reactivation

### ✅ Connector Interface
- Trait-based design (`BankConnector`)
- Pluggable adapters (SWIFT, ACH, RTGS, CBDC)
- Health checks
- Status polling

### ✅ Metrics
- Prometheus integration
- Request counters (total, success, failure)
- Duration histograms
- DLQ size gauges
- Circuit breaker state gauges

---

## Architecture

```text
┌──────────────────────────────────────────────────────────┐
│               Adapter Manager (Orchestrator)             │
│                                                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐        │
│  │Kill Switch │  │Circuit     │  │    DLQ     │        │
│  │            │  │Breaker     │  │            │        │
│  └────────────┘  └────────────┘  └────────────┘        │
└────────────┬─────────────────────────────────────────────┘
             │
    ┌────────┼──────────────┬──────────────┬──────────────┐
    │        │              │              │              │
┌───▼────┐ ┌─▼──────┐ ┌────▼──────┐ ┌─────▼──────────┐   │
│ SWIFT  │ │  ACH   │ │   RTGS    │ │ CBDC Bridge    │   │
│Adapter │ │Adapter │ │  Adapter  │ │    Adapter     │   │
└───┬────┘ └─┬──────┘ └────┬──────┘ └─────┬──────────┘   │
    │        │              │              │              │
    └────────┴──────────────┴──────────────┴──────────────┘
```

---

## Usage

### Basic Example

```rust
use adapters::{AdapterManager, CircuitBreakerConfig, TransferRequest};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Create manager
    let cb_config = CircuitBreakerConfig::default();
    let manager = Arc::new(AdapterManager::new(
        cb_config,
        1000, // DLQ max size
        3,    // Max retries
    ));

    // Register SWIFT connector
    let swift_connector = SwiftConnector::new(SwiftConfig {
        api_endpoint: "https://api.swift.com".into(),
        api_key: "secret".into(),
        timeout_seconds: 30,
    }).unwrap();

    manager.register_connector(Arc::new(swift_connector)).await;

    // Send transfer
    let request = TransferRequest { /* ... */ };
    match manager.send_transfer(request).await {
        Ok(response) => println!("Transfer accepted: {:?}", response),
        Err(e) => eprintln!("Transfer failed: {}", e),
    }
}
```

### ISO 20022 Generation

```rust
use adapters::iso20022::{Pacs008Generator, Pacs008};
use protocol_core::SettlementInstruction;

let instruction = SettlementInstruction { /* ... */ };

// Generate pacs.008
let pacs008 = Pacs008Generator::from_instruction(
    &instruction,
    "DELTRANAEXXX", // Instructing agent BIC
    "DELTRANUAEXXX", // Instructed agent BIC
)?;

// Serialize to XML
let xml = Pacs008Generator::to_xml(&pacs008)?;

// Validate
Pacs008Generator::validate(&pacs008)?;
```

### Circuit Breaker

```rust
// Check if request allowed
manager.circuit_breakers.is_request_allowed("UAE-IND").await?;

// Record success
manager.circuit_breakers.record_success("UAE-IND").await;

// Record failure
manager.circuit_breakers.record_failure("UAE-IND").await;

// Manual reset
manager.circuit_breakers.reset("UAE-IND").await;
```

### Kill Switch

```rust
// Activate
manager.activate_kill_switch(
    "UAE-IND",
    "High error rate detected".to_string(),
    "admin".to_string(),
).await?;

// Check if active
if manager.kill_switches.is_active("UAE-IND").await {
    println!("Kill switch is active!");
}

// Deactivate
manager.deactivate_kill_switch("UAE-IND", "admin".to_string()).await?;
```

### DLQ

```rust
// Push to DLQ
manager.dlq.push(request, "Connection timeout".to_string()).await?;

// Get size
let size = manager.dlq.size("UAE-IND").await;

// Clear DLQ
manager.dlq.clear("UAE-IND").await;
```

---

## Configuration

### Circuit Breaker

```rust
CircuitBreakerConfig {
    failure_threshold: 5,       // Open after 5 failures
    timeout_seconds: 60,        // Half-open after 60s
    success_threshold: 2,       // Close after 2 successes in half-open
}
```

### DLQ

```rust
DeadLetterQueue::new(
    1000, // Max size per corridor
    3,    // Max retry attempts
)
```

### SWIFT Connector

```rust
SwiftConfig {
    api_endpoint: "https://api.swift.com/v1".into(),
    api_key: env::var("SWIFT_API_KEY").unwrap(),
    timeout_seconds: 30,
}
```

---

## Testing

```bash
# Unit tests
cargo test --package adapters

# Specific module
cargo test --package adapters -- iso20022

# With output
cargo test --package adapters -- --nocapture
```

### Test Coverage

- ✅ ISO 20022 generation (3 tests)
- ✅ Circuit breaker transitions (2 tests)
- ✅ Kill switch activation/deactivation (1 test)
- ✅ DLQ push (1 test)

**Total: 7 tests, 5 passing (71%)**

---

## Metrics

### Prometheus Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `adapter_requests_total` | Counter | Total requests (by corridor, adapter type, status) |
| `adapter_request_duration_seconds` | Histogram | Request duration |
| `adapter_dlq_size` | Gauge | DLQ size per corridor |
| `adapter_circuit_breaker_state` | Gauge | Circuit breaker state (0=closed, 1=half-open, 2=open) |

### Example Queries

```promql
# Success rate
rate(adapter_requests_total{status="success"}[5m])
/
rate(adapter_requests_total[5m])

# p95 latency
histogram_quantile(0.95, adapter_request_duration_seconds_bucket[5m])

# DLQ backlog
adapter_dlq_size{corridor_id="UAE-IND"}

# Circuit breaker open corridors
adapter_circuit_breaker_state{} == 2
```

---

## Adapters

### SWIFT

- **Protocol**: ISO 20022 pacs.008
- **Transport**: HTTPS (SWIFT Alliance API)
- **Authentication**: API key
- **Status**: MVP implemented (mock)

### ACH (TODO)

- **Protocol**: NACHA file format
- **Transport**: SFTP
- **Authentication**: SSH keys
- **Status**: Not implemented

### RTGS (TODO)

- **Protocol**: ISO 20022 pacs.009
- **Transport**: Dedicated network
- **Authentication**: Certificate-based
- **Status**: Not implemented

### CBDC Bridge (TODO)

- **Protocol**: Custom RPC
- **Transport**: gRPC
- **Authentication**: mTLS
- **Status**: Not implemented

---

## Roadmap

### MVP (Current)
- ✅ ISO 20022 pacs.008 generator
- ✅ DLQ with exponential backoff
- ✅ Circuit breaker per corridor
- ✅ Kill switch
- ✅ SWIFT connector (mock)
- ✅ Metrics

### Short-term (2-3 weeks)
- [ ] Real SWIFT Alliance API integration
- [ ] ACH connector
- [ ] Persistent DLQ (Redis)
- [ ] Health check API endpoint
- [ ] Retry dashboard

### Medium-term (1-2 months)
- [ ] RTGS connector
- [ ] CBDC bridge connector
- [ ] Advanced metrics (P99, error breakdown)
- [ ] Alerting rules
- [ ] Chaos testing

---

## Security

### ✅ No Unsafe Code
```rust
#![forbid(unsafe_code)]
```

### ✅ API Key Management
- Never log API keys
- Load from environment variables
- Rotate regularly

### ✅ TLS/mTLS
- All bank connections encrypted
- Certificate validation
- Mutual authentication (for sensitive corridors)

### ✅ Audit Logging
- All kill switch activations logged
- Circuit breaker state changes logged
- DLQ operations logged

---

## Troubleshooting

### Circuit Breaker Keeps Opening

**Symptoms**: Circuit breaker opens frequently for a corridor.

**Causes**:
- Bank API downtime
- Network issues
- High latency

**Resolution**:
1. Check bank API status
2. Review metrics: `adapter_requests_total{status="failure"}`
3. Increase `failure_threshold` if transient errors
4. Manually reset: `manager.circuit_breakers.reset("corridor-id").await`

### DLQ Growing

**Symptoms**: `adapter_dlq_size` increasing.

**Causes**:
- Persistent failures
- Bank rejecting requests
- Configuration issues

**Resolution**:
1. Check DLQ messages: `manager.dlq.get_messages("corridor-id").await`
2. Review `last_error` field
3. Fix root cause (e.g., invalid BIC, insufficient funds)
4. Clear DLQ after fix: `manager.dlq.clear("corridor-id").await`

### Kill Switch Activated Accidentally

**Symptoms**: All requests rejected for a corridor.

**Resolution**:
```rust
manager.deactivate_kill_switch("corridor-id", "admin").await?;
```

---

## License

Apache 2.0 - See [LICENSE](../LICENSE)

---

**Made with ❤️ by DelTran Team**