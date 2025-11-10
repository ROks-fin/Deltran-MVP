# K6 Performance Tests for DelTran MVP

Complete performance testing suite for all 11 DelTran MVP services using K6.

## 📋 Test Scenarios

### 1. Integration Test - Health Checks
**File**: `scenarios/integration-test.js`

Tests health check endpoints for all 11 services:
- Gateway (8080)
- Token Engine (8081)
- Obligation Engine (8082)
- Liquidity Router (8083)
- Risk Engine (8084)
- Clearing Engine (8085)
- Compliance Engine (8086)
- Reporting Engine (8087)
- Settlement Engine (8088)
- Notification Engine (8089)
- Analytics Collector (8093)

**Thresholds**:
- Success rate > 95%
- P95 latency < 1000ms
- Failure rate < 5%

### 2. E2E Transaction Flow Test
**File**: `scenarios/e2e-transaction.js`

Tests complete transaction flow:
1. Create transaction via Gateway
2. Check transaction status
3. Verify in Analytics Collector

**Load Pattern**:
- Ramp up: 30s to 10 VUs
- Sustained: 1m at 50 VUs
- Ramp down: 30s to 0 VUs

**Thresholds**:
- Transaction success rate > 95%
- P95 latency < 1000ms
- P99 latency < 2000ms

### 3. Load Test - Realistic Scenarios
**File**: `scenarios/load-test-realistic.js`

Tests realistic transaction scenarios:
- Small INR-AED (5,000)
- Medium INR-AED (50,000)
- Large INR-AED (500,000)
- Small AED-INR (1,000)
- Medium AED-INR (10,000)
- Large AED-INR (100,000)
- XL Transaction (1,000,000)

**Load Pattern**:
- Executor: `constant-arrival-rate`
- Rate: 100 transactions per second
- Duration: 5 minutes
- Pre-allocated VUs: 50
- Max VUs: 200

**Thresholds**:
- P95 latency < 500ms
- P99 latency < 1000ms
- Failure rate < 5%

### 4. WebSocket Test - Notification Engine
**File**: `scenarios/websocket-test.js`

Tests WebSocket connections and real-time notifications:
- Connection establishment
- Channel subscriptions (transactions, settlements, notifications)
- Message reception
- Ping/pong latency

**Load Pattern**:
- Ramp up: 30s to 20 connections
- Sustained: 1m at 20 connections
- Ramp down: 30s to 0 connections

**Thresholds**:
- Connections established > 0
- Messages received > 0
- P95 message latency < 500ms

## 🚀 Installation

### Install K6

**macOS** (Homebrew):
```bash
brew install k6
```

**Windows** (Chocolatey):
```powershell
choco install k6
```

**Windows** (Manual):
1. Download from https://k6.io/docs/getting-started/installation/
2. Extract and add to PATH

**Linux** (Debian/Ubuntu):
```bash
sudo gpg -k
sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6
```

## 📦 Project Structure

```
tests/k6/
├── config/
│   └── services.js              # Service endpoints and configuration
├── scenarios/
│   ├── integration-test.js      # Health check integration tests
│   ├── e2e-transaction.js       # End-to-end transaction flow
│   ├── load-test-realistic.js   # Realistic load testing
│   └── websocket-test.js        # WebSocket testing
├── results/                     # Test results (auto-generated)
├── run_tests.sh                 # Test runner (Linux/macOS)
├── run_tests.bat                # Test runner (Windows)
└── README.md                    # This file
```

## 🏃 Running Tests

### Run All Tests

**Linux/macOS**:
```bash
cd tests/k6
chmod +x run_tests.sh
./run_tests.sh
```

**Windows**:
```powershell
cd tests\k6
run_tests.bat
```

### Run Individual Tests

**Integration Test**:
```bash
k6 run scenarios/integration-test.js
```

**E2E Transaction Test**:
```bash
k6 run scenarios/e2e-transaction.js
```

**Load Test**:
```bash
k6 run scenarios/load-test-realistic.js
```

**WebSocket Test**:
```bash
k6 run scenarios/websocket-test.js
```

### Run with Custom Options

**Increase duration**:
```bash
k6 run --duration 10m scenarios/load-test-realistic.js
```

**Increase VUs**:
```bash
k6 run --vus 100 scenarios/e2e-transaction.js
```

**Save results to JSON**:
```bash
k6 run --out json=results/test.json scenarios/integration-test.js
```

**Run in cloud** (requires K6 Cloud account):
```bash
k6 cloud scenarios/load-test-realistic.js
```

## 📊 Results

Test results are automatically saved to `results/run_<timestamp>/` directory:
- `integration.json` - Integration test results
- `e2e.json` - E2E transaction test results
- `load.json` - Load test results
- `websocket.json` - WebSocket test results

### Reading Results

Each JSON file contains:
- **Metrics**: http_req_duration, http_req_failed, custom metrics
- **Thresholds**: Pass/fail status for each threshold
- **Summary**: Aggregated statistics (min, max, avg, p95, p99)

### Viewing Results in Grafana

1. Import K6 results to InfluxDB:
```bash
k6 run --out influxdb=http://localhost:8086/k6 scenarios/load-test-realistic.js
```

2. Create Grafana dashboard with K6 datasource

## 🔧 Configuration

Edit `config/services.js` to modify:
- Service URLs
- Service endpoints
- Test data generators
- HTTP headers

Example:
```javascript
export const SERVICES = {
    gateway: {
        url: 'http://localhost:8080',
        endpoints: {
            transfer: '/api/v1/transfer',
            health: '/health',
        }
    },
    // ... other services
};
```

## 📈 Performance Targets

Based on DelTran MVP requirements:

| Metric | Target | Critical |
|--------|--------|----------|
| Throughput | 100 TPS | 200 TPS |
| P95 Latency | < 500ms | < 1000ms |
| P99 Latency | < 1000ms | < 2000ms |
| Error Rate | < 1% | < 5% |
| Success Rate | > 99% | > 95% |

## 🐛 Troubleshooting

### K6 not found
```bash
# Check if K6 is installed
k6 version

# If not installed, follow installation instructions above
```

### Connection refused
```bash
# Check if all services are running
curl http://localhost:8080/health
curl http://localhost:8081/health
# ... check all services

# Start services if needed
docker-compose up -d
```

### WebSocket connection failed
```bash
# Check Notification Engine
curl http://localhost:8089/health

# Check WebSocket endpoint manually
wscat -c ws://localhost:8089/ws
```

### High error rate
- Check service logs for errors
- Verify database connections
- Check NATS connectivity
- Monitor resource usage (CPU, memory)

## 📚 Resources

- [K6 Documentation](https://k6.io/docs/)
- [K6 Test Types](https://k6.io/docs/test-types/introduction/)
- [K6 Metrics](https://k6.io/docs/using-k6/metrics/)
- [K6 Thresholds](https://k6.io/docs/using-k6/thresholds/)
- [K6 Checks](https://k6.io/docs/using-k6/checks/)

## 🔗 Related Documentation

- [Agent-Performance Report](../../AGENT_PERFORMANCE_REPORT.md)
- [Agent-Analytics Report](../../AGENT_ANALYTICS_REPORT.md)
- [Monitoring Setup](../../monitoring/README.md)
- [HOW_TO_USE_AGENTS.md](../../HOW_TO_USE_AGENTS.md)
