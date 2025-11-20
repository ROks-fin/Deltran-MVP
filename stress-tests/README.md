# DelTran Stress Testing

This directory contains K6 load tests for the DelTran Gateway Service with **real ISO 20022 messages**.

## Prerequisites

- K6 installed (see `d:\MVP DelTran\k6-v0.49.0-windows-amd64\k6.exe`)
- Gateway Service running on port 8080
- PostgreSQL database running
- NATS server running

## Test Scenarios

### 1. pain.001 Load Test (`pain001_load_test.js`)

Tests Gateway's ability to handle payment initiation messages at scale.

**Load Profile:**
- Ramp up: 1 minute to 50 TPS
- Sustained: 3 minutes at 100 TPS
- Spike: 2 minutes at 200 TPS
- Recovery: 2 minutes at 100 TPS
- Ramp down: 1 minute to 0

**Metrics:**
- Payment processing duration (p95 < 500ms, p99 < 1000ms)
- Error rate (< 1%)
- Payments created count

**Run:**
```bash
..\..\k6-v0.49.0-windows-amd64\k6.exe run pain001_load_test.js
```

### 2. End-to-End Flow Test (`end_to_end_flow_test.js`)

Simulates complete payment lifecycle with real ISO messages:
1. Submit pain.001 (payment initiation)
2. Submit camt.054 (funding notification)
3. Verify payment status

**Load Profile:**
- 2 minutes: Ramp to 20 concurrent flows
- 5 minutes: Sustain at 50 concurrent flows
- 3 minutes: Spike to 100 concurrent flows
- 2 minutes: Ramp down to 0

**Metrics:**
- End-to-end latency (p95 < 2000ms, p99 < 5000ms)
- Completed payments (> 1000)
- Funding success rate

**Run:**
```bash
..\..\k6-v0.49.0-windows-amd64\k6.exe run end_to_end_flow_test.js
```

### 3. Mixed Message Load Test (`mixed_message_load_test.js`)

Tests Gateway with realistic mix of message types:
- 60% pain.001 (payment initiation)
- 20% camt.054 (funding)
- 15% pacs.008 (settlement)
- 5% pacs.002 (status reports)

**Run:**
```bash
..\..\k6-v0.49.0-windows-amd64\k6.exe run mixed_message_load_test.js
```

## Running All Tests

```bash
# Run all tests in sequence
bash run_all_tests.sh
```

## Configuration

Override default settings with environment variables:

```bash
# Custom Gateway URL
set GATEWAY_URL=http://gateway:8080

# Custom duration
..\..\k6-v0.49.0-windows-amd64\k6.exe run --duration 10m pain001_load_test.js

# Custom VUs
..\..\k6-v0.49.0-windows-amd64\k6.exe run --vus 100 --duration 5m pain001_load_test.js
```

## Results

Test results are saved to `stress-tests/results/` as JSON files:
- `pain001_load_test_result.json`
- `end_to_end_flow_result.json`
- `mixed_message_load_test_result.json`

## Success Criteria

For investor demo and pilot deployment:

### Performance
- ✅ p95 response time < 500ms for pain.001
- ✅ p99 response time < 1000ms for pain.001
- ✅ End-to-end payment flow < 2 seconds (p95)

### Reliability
- ✅ Error rate < 1% under load
- ✅ Zero data loss (all payments persisted)
- ✅ Funding events correctly matched to payments

### Scalability
- ✅ Handle 200 TPS sustained
- ✅ Handle 500 TPS spike for 1 minute
- ✅ 1000+ concurrent payment flows

### Business Metrics
- ✅ 100% payment-to-funding match rate
- ✅ Real-time event propagation to NATS
- ✅ Database persistence latency < 50ms

## Monitoring During Tests

Monitor Gateway metrics:
```bash
# Health check
curl http://localhost:8080/health

# Check PostgreSQL connections
psql -U deltran -c "SELECT count(*) FROM payments;"

# NATS monitoring
nats-top
```

## Troubleshooting

### High Error Rates
- Check Gateway logs for parsing errors
- Verify database connection pool size
- Check NATS connectivity

### Slow Response Times
- Increase PostgreSQL max_connections
- Tune database indexes
- Increase Gateway worker threads

### Memory Issues
- Monitor Gateway memory usage
- Check for connection leaks
- Adjust K6 batch size
