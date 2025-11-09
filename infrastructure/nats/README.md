# NATS JetStream Configuration

## Overview
NATS JetStream provides persistent, reliable message delivery for DelTran's event-driven architecture.

## Streams Configuration

### Event Streams

| Stream | Retention | Purpose | Subjects |
|--------|-----------|---------|----------|
| TRANSACTIONS | 7 days | Transaction lifecycle events | events.transactions.* |
| COMPLIANCE | 30 days | Compliance checks and alerts | events.compliance.* |
| SETTLEMENT | 90 days | Settlement operations | events.settlement.* |
| CLEARING | 30 days | Clearing window operations | events.clearing.* |
| NOTIFICATIONS | 7 days | Real-time notifications | events.notifications.* |
| REPORTING | 90 days | Reporting and analytics | events.reporting.* |
| RISK | 30 days | Risk events and circuit breakers | events.risk.* |
| AUDIT | 180 days | Complete audit trail | events.audit.* |

### Retention Policies

- **Short-term (7 days)**: Operational events - TRANSACTIONS, NOTIFICATIONS
- **Medium-term (30 days)**: Financial compliance - COMPLIANCE, CLEARING, RISK
- **Long-term (90 days)**: Settlement and reporting - SETTLEMENT, REPORTING
- **Extended (180 days)**: Audit trail - AUDIT

## Setup Instructions

### 1. Start NATS with JetStream

```bash
# Using Docker Compose (recommended)
docker-compose up -d nats

# Or standalone
docker run -d \
  --name deltran-nats \
  -p 4222:4222 \
  -p 8222:8222 \
  -v $(pwd)/nats-jetstream.conf:/etc/nats/nats-server.conf \
  -v nats-data:/data/jetstream \
  nats:2.10-alpine \
  -c /etc/nats/nats-server.conf
```

### 2. Initialize Streams

Using NATS CLI:
```bash
# Install NATS CLI
go install github.com/nats-io/natscli/nats@latest

# Create streams from configuration
nats stream add TRANSACTIONS --config streams-setup.json
nats stream add COMPLIANCE --config streams-setup.json
nats stream add SETTLEMENT --config streams-setup.json
nats stream add CLEARING --config streams-setup.json
nats stream add NOTIFICATIONS --config streams-setup.json
nats stream add REPORTING --config streams-setup.json
nats stream add RISK --config streams-setup.json
nats stream add AUDIT --config streams-setup.json
```

Or use the automated setup script:
```bash
chmod +x setup-streams.sh
./setup-streams.sh
```

### 3. Verify Streams

```bash
# List all streams
nats stream ls

# Get stream info
nats stream info TRANSACTIONS

# View stream configuration
nats stream get TRANSACTIONS
```

### 4. Create Consumers

```bash
# Create durable consumer for notification engine
nats consumer add NOTIFICATIONS notification-engine-consumer \
  --filter "events.notifications.*" \
  --ack explicit \
  --pull \
  --deliver all \
  --max-deliver 3 \
  --wait 30s

# Create consumer for reporting engine
nats consumer add REPORTING reporting-engine-consumer \
  --filter "events.reporting.*" \
  --ack explicit \
  --pull \
  --deliver all \
  --max-deliver 3 \
  --wait 5m
```

## Monitoring

### Health Check
```bash
curl http://localhost:8222/healthz
```

### Stream Stats
```bash
curl http://localhost:8222/jsz
```

### Connection Stats
```bash
curl http://localhost:8222/connz
```

## Subject Naming Convention

Format: `events.<domain>.<entity>.<action>`

Examples:
- `events.transactions.created`
- `events.compliance.alert`
- `events.settlement.completed`
- `events.clearing.window.opened`

## Event Structure

All events should follow this structure:

```json
{
  "event_id": "uuid",
  "event_type": "transaction.created",
  "timestamp": "2025-11-06T12:00:00Z",
  "source": "token-engine",
  "version": "1.0",
  "data": {
    "transaction_id": "uuid",
    "amount": 1000.00,
    "currency": "USD"
  },
  "metadata": {
    "correlation_id": "uuid",
    "user_id": "uuid"
  }
}
```

## Troubleshooting

### Stream not receiving messages
1. Check stream configuration: `nats stream info STREAM_NAME`
2. Verify subject matching: Ensure published subject matches stream subjects
3. Check permissions: Verify service has publish rights

### Consumer not processing
1. Check consumer status: `nats consumer info STREAM CONSUMER`
2. Verify acknowledgments: Ensure messages are being acked
3. Check max_deliver: Consumer may have exceeded retry limit

### High memory usage
1. Check stream retention: Adjust max_age or max_msgs
2. Enable limits: Set max_bytes to limit storage
3. Monitor discards: Check if old messages are being discarded

## Performance Tuning

### For high throughput:
- Increase `max_payload` to 8MB
- Set `write_deadline` appropriately
- Use file storage for persistence
- Enable compression for large payloads

### For low latency:
- Use memory storage for hot streams
- Reduce `ack_wait` timeout
- Increase `max_ack_pending`
- Use push-based consumers

## Security Considerations

### Production Deployment:
1. Enable TLS for client connections
2. Configure user authentication
3. Set up account isolation
4. Implement subject-level permissions
5. Enable audit logging
6. Rotate credentials regularly

## Backup & Recovery

### Backup streams:
```bash
# Backup stream data
nats stream backup TRANSACTIONS backup/transactions.tar.gz

# Restore from backup
nats stream restore backup/transactions.tar.gz
```

### Disaster Recovery:
- Configure stream replication (replicas: 3)
- Set up clustered NATS for high availability
- Regular backup schedule
- Test restore procedures

## Integration Examples

### Rust Publisher
```rust
use async_nats::jetstream;

let client = async_nats::connect("nats://localhost:4222").await?;
let jetstream = jetstream::new(client);

let event = serde_json::json!({
    "event_id": uuid::Uuid::new_v4(),
    "event_type": "transaction.created",
    "timestamp": chrono::Utc::now(),
    "data": {...}
});

jetstream.publish(
    "events.transactions.created",
    event.to_string().into()
).await?;
```

### Go Consumer
```go
nc, _ := nats.Connect("nats://localhost:4222")
js, _ := nc.JetStream()

sub, _ := js.PullSubscribe(
    "events.notifications.*",
    "notification-engine-consumer",
    nats.Durable("notification-engine-consumer"),
)

for {
    msgs, _ := sub.Fetch(10, nats.MaxWait(5*time.Second))
    for _, msg := range msgs {
        // Process message
        msg.Ack()
    }
}
```

## References

- [NATS Documentation](https://docs.nats.io/)
- [JetStream Guide](https://docs.nats.io/nats-concepts/jetstream)
- [NATS CLI](https://github.com/nats-io/natscli)
