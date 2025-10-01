# DelTran Message Bus

Message bus implementation with NATS and JetStream for reliable pub/sub messaging.

## Features

✅ **NATS Integration**
- Core NATS for high-performance messaging
- JetStream for persistence and guaranteed delivery
- Automatic reconnection with exponential backoff

✅ **Partitioning**
- Hash-based partitioning by `corridor_id` and `bank_id`
- 32 partitions by default (configurable)
- Consistent hashing using BLAKE3

✅ **Publisher**
- Retry logic with exponential backoff (3 attempts by default)
- JetStream acknowledgments for guaranteed delivery
- Request-reply pattern support

✅ **Subscriber**
- Consumer groups for load balancing across workers
- Durable consumers (survive restarts)
- Explicit acknowledgments (ACK/NAK/TERM)
- Max delivery attempts (3 by default)

✅ **Observability**
- Prometheus metrics for publish/receive/processing
- Connection status monitoring
- Duration histograms

## Architecture

```
┌─────────────┐         ┌──────────────┐         ┌──────────────┐
│  Publisher  │─────────│     NATS     │─────────│  Subscriber  │
│             │         │  JetStream   │         │ (Consumer)   │
└─────────────┘         └──────────────┘         └──────────────┘
      │                        │                         │
      │                        │                         │
      ├── partition by         │                         │
      │   corridor/bank        ├── 32 partitions         │
      │                        │                         │
      ├── retry 3x             ├── persist to disk       │
      │                        │                         │
      └── JetStream ACK        └── consumer groups ──────┘
```

## Message Types

- `PaymentInstruction` - Payment initiation
- `NettingProposal` - Netting cycle proposals
- `BankConfirmation` - ACK/NACK from banks
- `SettlementFinalize` - Finalization commands
- `SettlementProof` - Cryptographic proofs
- `Checkpoint` - Consensus checkpoints
- `AdapterTransfer` - Bank adapter requests
- `AdapterResponse` - Bank adapter responses
- `RiskAssessment` - Risk engine decisions
- `ComplianceCheck` - Compliance screening
- `SystemEvent` - System-wide events

## Usage

### Basic Publisher

```rust
use message_bus::{NatsClient, NatsConfig, Publisher, PublisherConfig, Message, MessageType, PartitionKey};
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to NATS
    let config = NatsConfig {
        urls: vec!["nats://localhost:4222".to_string()],
        enable_jetstream: true,
        ..Default::default()
    };

    let client = Arc::new(NatsClient::new(config));
    client.connect().await?;

    // Create publisher
    let publisher = Publisher::new(client.clone(), PublisherConfig::default());

    // Publish message
    let message = Message::new(
        MessageType::PaymentInstruction,
        PartitionKey::Corridor("USD-EUR".to_string()),
        json!({
            "payment_id": "pay_123",
            "amount": 1000.00,
            "currency": "USD"
        }),
    );

    publisher.publish(&message).await?;

    Ok(())
}
```

### Basic Subscriber

```rust
use message_bus::{
    NatsClient, NatsConfig, Subscriber, SubscriberConfig,
    MessageHandler, Message, MessageType
};
use async_trait::async_trait;
use std::sync::Arc;

struct MyHandler;

#[async_trait]
impl MessageHandler for MyHandler {
    async fn handle(&self, message: Message) -> message_bus::Result<()> {
        println!("Received: {:?}", message.payload);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = NatsConfig::default();
    let client = Arc::new(NatsClient::new(config));
    client.connect().await?;

    let subscriber = Subscriber::new(
        client.clone(),
        SubscriberConfig::default(),
        MessageType::PaymentInstruction,
    );

    let handler = Arc::new(MyHandler);
    subscriber.subscribe(handler).await?;

    Ok(())
}
```

### Partitioning Example

```rust
use message_bus::PartitionKey;

// Partition by corridor
let key = PartitionKey::Corridor("USD-EUR".to_string());
let partition = key.partition_number(32); // 0-31

// Partition by bank
let key = PartitionKey::Bank("BANKGB2L".to_string());

// Partition by both
let key = PartitionKey::CorridorBank {
    corridor_id: "USD-EUR".to_string(),
    bank_id: "BANKGB2L".to_string(),
};

// Broadcast to all
let key = PartitionKey::Broadcast;
```

## JetStream Streams

Each message type has a dedicated JetStream stream:

| Message Type          | Stream Name            | Retention | Max Age |
|-----------------------|------------------------|-----------|---------|
| PaymentInstruction    | PAYMENT_INSTRUCTIONS   | Limits    | 7 days  |
| NettingProposal       | NETTING_PROPOSALS      | Limits    | 7 days  |
| BankConfirmation      | BANK_CONFIRMATIONS     | Limits    | 7 days  |
| SettlementFinalize    | SETTLEMENT_FINALIZATIONS| Limits   | 7 days  |
| SettlementProof       | SETTLEMENT_PROOFS      | Limits    | 30 days |
| Checkpoint            | CHECKPOINTS            | Limits    | 30 days |
| AdapterTransfer       | ADAPTER_TRANSFERS      | Limits    | 3 days  |
| AdapterResponse       | ADAPTER_RESPONSES      | Limits    | 3 days  |

## Subject Hierarchy

```
deltran.{message_type}.{partition_key}

Examples:
  deltran.payment.instruction.corridor.USD-EUR
  deltran.payment.instruction.bank.BANKGB2L
  deltran.payment.instruction.corridor.USD-EUR.bank.BANKGB2L
  deltran.netting.proposal.corridor.GBP-USD
  deltran.settlement.proof.broadcast
```

## Configuration

### Publisher Config

```rust
PublisherConfig {
    use_jetstream: true,                             // Enable persistence
    publish_timeout: Duration::from_secs(5),         // Overall timeout
    max_retry_attempts: 3,                           // Retry count
    initial_retry_delay: Duration::from_millis(100), // Initial backoff
    max_retry_delay: Duration::from_secs(2),         // Max backoff
}
```

### Subscriber Config

```rust
SubscriberConfig {
    consumer_group: "deltran-workers".to_string(),   // Consumer group name
    durable_name: "deltran-consumer".to_string(),    // Durable name
    max_concurrent: 10,                              // Concurrent messages
    ack_wait: Duration::from_secs(30),               // ACK timeout
    max_deliver: 3,                                  // Max redelivery
    use_jetstream: true,                             // Enable JetStream
}
```

### NATS Config

```rust
NatsConfig {
    urls: vec!["nats://localhost:4222".to_string()], // Server URLs
    name: "deltran".to_string(),                     // Connection name
    max_reconnect_attempts: None,                    // Infinite reconnects
    reconnect_delay: Duration::from_secs(2),         // Reconnect delay
    connection_timeout: Duration::from_secs(5),      // Connect timeout
    enable_jetstream: true,                          // Enable JetStream
}
```

## Docker Compose

Start NATS cluster:

```bash
docker-compose -f infra/docker-compose.nats.yml up -d
```

This starts:
- 3 NATS servers (HA cluster)
- JetStream enabled on all nodes
- Prometheus exporter for metrics
- NATS Box for CLI tools

Access:
- NATS: `localhost:4222` (primary)
- HTTP monitoring: `http://localhost:8222`
- Metrics: `http://localhost:7777/metrics`

## Monitoring

Prometheus metrics exposed:

```
# Publish metrics
message_bus_publish_total{message_type, status}
message_bus_publish_duration_seconds{message_type}

# Receive metrics
message_bus_receive_total{message_type, status}
message_bus_process_duration_seconds{message_type}

# Connection metrics
nats_connection_status{status}
```

## CLI Tools

Using NATS Box:

```bash
# List streams
docker exec deltran-nats-box nats --server nats://nats-1:4222 stream ls

# List consumers for a stream
docker exec deltran-nats-box nats --server nats://nats-1:4222 consumer ls PAYMENT_INSTRUCTIONS

# Publish test message
docker exec deltran-nats-box nats --server nats://nats-1:4222 pub deltran.payment.instruction.corridor.USD-EUR '{"test": "data"}'

# Subscribe to messages
docker exec deltran-nats-box nats --server nats://nats-1:4222 sub "deltran.payment.instruction.*"

# Check cluster status
docker exec deltran-nats-box nats --server nats://nats-1:4222 server report jetstream
```

## Performance

- **Throughput**: >1M msg/sec (core NATS)
- **Latency**: <1ms (p50), <10ms (p99)
- **Persistence**: JetStream with file storage
- **Partitions**: 32 (can scale to 256+)
- **Consumer Groups**: Horizontal scaling

## Production Checklist

- [ ] Deploy 3+ NATS servers for HA
- [ ] Enable TLS for encrypted connections
- [ ] Configure authentication (NKEY/JWT)
- [ ] Set up monitoring (Prometheus + Grafana)
- [ ] Tune JetStream limits (file store, memory)
- [ ] Configure backups for JetStream data
- [ ] Set up log aggregation
- [ ] Test failover scenarios
- [ ] Implement circuit breakers for publishers
- [ ] Configure dead letter queues

## Testing

Run tests:

```bash
cargo test --package message-bus
```

Integration tests require NATS server:

```bash
# Start NATS
docker run -d --name nats -p 4222:4222 nats:2.10-alpine --jetstream

# Run tests
cargo test --package message-bus -- --ignored

# Cleanup
docker rm -f nats
```

## License

Apache-2.0
