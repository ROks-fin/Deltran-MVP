# Ledger Core

High-performance append-only event ledger written in Rust.

## Features

- **Event Sourcing**: All state derived from immutable events
- **Single-Writer Pattern**: Eliminates race conditions
- **Batching**: 10x throughput with configurable batching
- **Cryptographic Signatures**: Ed25519 for event authentication
- **Merkle Proofs**: Cryptographic proof of inclusion
- **Memory Safe**: `#![forbid(unsafe_code)]`
- **Exact Arithmetic**: rust_decimal for money (no floats)
- **Performance**: 1,000+ TPS, p95 <10ms append latency

## Quick Start

```rust
use ledger_core::{Config, Ledger, types::*};

#[tokio::main]
async fn main() -> ledger_core::Result<()> {
    // Open ledger
    let config = Config::default();
    let ledger = Ledger::open(config).await?;

    // Create event
    let event = LedgerEvent {
        event_id: Uuid::now_v7(),
        payment_id: Uuid::now_v7(),
        event_type: EventType::PaymentInitiated,
        amount: Decimal::new(10000, 2), // $100.00
        currency: Currency::USD,
        debtor: AccountId::new("US1234567890"),
        creditor: AccountId::new("AE9876543210"),
        timestamp_nanos: chrono::Utc::now().timestamp_nanos_opt().unwrap(),
        block_id: None,
        signature: Signature::from_bytes([0u8; 64]),
        previous_event_id: None,
        metadata: Default::default(),
    };

    // Append event
    let event_id = ledger.append_event(event).await?;

    // Retrieve event
    let retrieved = ledger.get_event(event_id).await?;

    // Get payment state
    let state = ledger.get_payment_state(payment_id).await?;

    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│            Multiple Writers (gRPC Clients)          │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────┐
│         Bounded Channel (Backpressure)              │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────┐
│      Single Writer Actor (Batching: 100/10ms)       │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────┐
│        RocksDB (6 Column Families, Atomic)          │
│  events | blocks | state | indices | merkle | snaps │
└─────────────────────────────────────────────────────┘
```

## Storage Layout

### Column Families

1. **events**: Append-only event log
   - Key: `event_id` (16 bytes)
   - Value: `LedgerEvent` (bincode serialized)

2. **blocks**: Finalized blocks
   - Key: `block_height` (8 bytes)
   - Value: `Block` (bincode serialized)

3. **state**: Payment states
   - Key: `payment_id` (16 bytes)
   - Value: `PaymentState` (bincode serialized)

4. **indices**: Secondary indices
   - `payment_event`: `(payment_id, event_id)` → `[]`
   - `account_payment`: `(account_id, payment_id)` → `[]`
   - `status_payment`: `(status, payment_id)` → `[]`

5. **merkle**: Merkle tree nodes
   - Key: `node_id` (hash)
   - Value: `MerkleNode`

6. **snapshots**: Snapshot metadata
   - Key: `snapshot_id`
   - Value: `SnapshotMetadata`

## Configuration

### Config File (`config.toml`)

```toml
data_dir = "./data/ledger"
service_name = "ledger-core"
grpc_listen_addr = "0.0.0.0:50051"
metrics_listen_addr = "0.0.0.0:9090"

[rocksdb]
write_buffer_size_mb = 256
max_write_buffer_number = 4
target_file_size_mb = 256
max_background_jobs = 4
enable_statistics = true

[batching]
max_batch_size = 100
batch_timeout_ms = 10
enabled = true

[snapshot]
snapshot_interval_blocks = 10000
compress = true
compression_level = 3
```

### Environment Variables

```bash
export LEDGER_DATA_DIR=/data/ledger
export LEDGER_GRPC_ADDR=0.0.0.0:50051
export LEDGER_METRICS_ADDR=0.0.0.0:9090
```

## Testing

### Unit Tests

```bash
cargo test
```

### Property-Based Tests

```bash
cargo test --test property_tests
```

### Benchmarks

```bash
cargo bench --bench append_benchmark
```

## Performance

### Latency (Unbatched)
- Append: p50 ~5ms, p95 <10ms, p99 <20ms
- Read: p50 ~1ms, p95 <5ms

### Latency (Batched)
- Append: p50 ~0.5ms, p95 <1ms, p99 <5ms
- Batch flush: ~10ms (100 events = 0.1ms/event)

### Throughput
- Unbatched: ~200 TPS
- Batched: 1,000+ TPS
- Target: 5,000 TPS (optimized)

### Storage
- Event size: ~500 bytes
- 1M events: ~500 MB (raw), ~200 MB (compressed)

## Invariants

### 1. Money Conservation
```rust
let valid = ledger.check_money_conservation(payment_id).await?;
assert!(valid); // Σ(debits) == Σ(credits)
```

### 2. Deterministic Replay
```rust
let state = ledger.rebuild_payment_state(payment_id).await?;
// Same events → same state
```

### 3. Linearizability
- UUIDv7 embeds timestamp
- Single-writer enforces total order
- Atomic batch writes

### 4. Append-Only
- Events never modified
- Only finalized into blocks
- Full audit trail

## Cryptography

### Ed25519 Signatures
```rust
use ledger_core::crypto::KeyPair;

let keypair = KeyPair::generate();
let signature = keypair.sign(message);
assert!(keypair.verify(message, &signature).is_ok());
```

### Merkle Proofs
```rust
use ledger_core::merkle::MerkleTree;

let mut tree = MerkleTree::new();
tree.append(leaf_hash);
let root = tree.root();

let proof = tree.generate_proof(0).unwrap();
assert!(proof.verify());
```

## Metrics

### Prometheus Metrics

- `ledger_events_total` - Total events appended
- `ledger_events_batch_size` - Batch size histogram
- `ledger_append_duration_seconds` - Append latency
- `ledger_blocks_total` - Total blocks finalized
- `ledger_storage_size_bytes` - Storage size estimate

### Example Query

```promql
rate(ledger_events_total[5m])  # Events per second
histogram_quantile(0.95, ledger_append_duration_seconds)  # p95 latency
```

## Development

### Build

```bash
cargo build --release
```

### Run Tests

```bash
cargo test --all
```

### Run Benchmarks

```bash
cargo bench
```

### Lint

```bash
cargo clippy -- -D warnings
```

### Format

```bash
cargo fmt
```

## Production Deployment

### Enable Production Features

```bash
cargo build --release --features production
```

This enables:
- jemalloc allocator (better performance)
- Link-time optimization
- Optimized RocksDB

### Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --features production

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl3 ca-certificates
COPY --from=builder /app/target/release/ledger-server /usr/local/bin/
CMD ["ledger-server"]
```

### Systemd Service

```ini
[Unit]
Description=Ledger Core
After=network.target

[Service]
Type=simple
User=ledger
Environment=LEDGER_DATA_DIR=/var/lib/ledger
Environment=RUST_LOG=info
ExecStart=/usr/local/bin/ledger-server
Restart=always

[Install]
WantedBy=multi-user.target
```

## Monitoring

### Logging

```rust
tracing::info!("Event appended", event_id = %event.event_id);
tracing::error!("Failed to append event", error = ?err);
```

### Metrics Dashboard

Import `dashboards/ledger-grafana.json` for Grafana dashboard.

## Security

### Memory Safety
- `#![forbid(unsafe_code)]`
- No buffer overflows
- No data races

### Cryptographic Verification
- Ed25519 signatures (128-bit security)
- SHA-256 hashing
- Merkle proofs

### Audit Trail
- All events immutable
- Full history preserved
- Cryptographically verifiable

## License

Apache-2.0

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md)