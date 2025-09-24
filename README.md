# DelTran Rail MVP

A high-performance, permissioned ledger system for cross-border rail payments with regulatory compliance, real-time settlement, and comprehensive observability.

## Architecture

- **Gateway**: FastAPI with mTLS, idempotency, API routing
- **Ledger**: PoA/IBFT simulation with append-only events and state derivation
- **Settlement**: Netting/clearing with min-cost flow optimization
- **Liquidity Router**: Mock providers with SLA ≤150ms quote delivery
- **Risk Engine**: Dynamic mode switching (Low/Medium/High) with real-time metrics
- **Compliance**: Travel Rule, sanctions/PEP screening, PII masking
- **Exporter**: ISO20022 message generation with XSD validation
- **Observability**: Prometheus metrics, Grafana dashboards, Jaeger tracing

## Technology Stack

- **Language**: Python 3.11
- **Framework**: FastAPI
- **Database**: PostgreSQL + Redis
- **Message Bus**: NATS JetStream
- **Observability**: Prometheus + Grafana + Jaeger (OTLP)
- **Testing**: pytest + coverage
- **Load Testing**: k6
- **Chaos Engineering**: toxiproxy
- **Infrastructure**: Docker Compose

## Quick Start

### Prerequisites

- Docker & Docker Compose
- Make (optional, for convenience)
- Python 3.11+ (for local development)

### Run the Live Demo

1. **Clone and setup**:
   ```bash
   git clone <repository-url>
   cd deltran-rail-mvp
   cp infra/.env.example .env
   ```

2. **Start infrastructure**:
   ```bash
   make up
   # OR: docker-compose up -d
   ```

3. **Initialize data**:
   ```bash
   make seed
   # Generates: banks, accounts, 1M USD micropayments, AED↔INR scenarios
   ```

4. **Run acceptance tests**:
   ```bash
   make test
   # Validates: API contracts, ISO schemas, integration flows
   ```

5. **Access dashboards**:
   - **API Gateway**: http://localhost:8000
   - **Grafana**: http://localhost:3000 (admin/admin)
   - **Jaeger**: http://localhost:16686
   - **Prometheus**: http://localhost:9090

### Acceptance Checks

After running `make demo`, verify these endpoints:

```bash
# 1. Payment initiation with idempotency
curl -X POST http://localhost:8000/payments/initiate \
  -H "Idempotency-Key: test-$(uuidgen)" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "1000.00",
    "currency": "USD",
    "debtor_account": "US12345678901234567890",
    "creditor_account": "GB98765432109876543210"
  }'

# 2. Payment status tracking
curl http://localhost:8000/payments/{payment_id}/status

# 3. Liquidity quotes (SLA ≤150ms)
curl http://localhost:8000/liquidity/quotes?from=USD&to=AED&amount=10000

# 4. Risk mode check
curl http://localhost:8000/risk/mode

# 5. Settlement batch closure
curl -X POST http://localhost:8000/settlement/close-batch?window=intraday

# 6. Proof of reserves
curl http://localhost:8000/reports/proof-of-reserves

# 7. ISO20022 export validation
curl http://localhost:8000/reports/proof-of-settlement
```

Expected results:
- All APIs return HTTP 200
- Payment status shows "COMPLETED" or "PENDING"
- Liquidity quotes return in <150ms with valid rates
- Risk mode shows current threshold settings
- ISO20022 exports validate against XSD schemas
- Grafana shows TPS/latency metrics
- Jaeger shows end-to-end traces

## Performance Testing

```bash
# Load test: 500 TPS for 30-60 minutes
make load

# Chaos engineering: network delays, node failures
make chaos

# Combined demo with all tests
make demo
```

## API Endpoints

### Payments
- `POST /payments/initiate` - Initiate payment with Travel Rule compliance
- `GET /payments/{id}/status` - Get payment status and settlement proof

### Settlement
- `POST /settlement/close-batch?window=intraday|EOD` - Close settlement batch with netting

### Liquidity
- `GET /liquidity/quotes` - Get real-time FX quotes (SLA ≤150ms)

### Risk Management
- `GET /risk/mode` - Get current risk mode (Low/Medium/High)
- `POST /risk/mode` - Switch risk mode (affects payment routing)

### Reports
- `GET /reports/proof-of-reserves` - Generate reserves attestation
- `GET /reports/proof-of-settlement` - Generate settlement proof with ISO20022

## Development

### Local setup:
```bash
# Install dependencies
pip install -r requirements.txt

# Run migrations
alembic upgrade head

# Start services individually
python -m gateway.main
python -m ledger.main
# etc...
```

### Testing:
```bash
# Unit tests
pytest tests/unit/

# Integration tests
pytest tests/integration/

# Contract tests
pytest tests/contracts/

# Coverage report
coverage run -m pytest && coverage report
```

## Monitoring

### Key Metrics
- **TPS**: Transactions per second across all services
- **Latency**: P50/P95/P99 for payment processing
- **Error Rate**: 4xx/5xx responses by service
- **Settlement**: Batch sizes, netting efficiency
- **Risk**: Mode switches, threshold breaches
- **Compliance**: Travel Rule coverage, sanctions hits

### Alerts
- Payment processing >5s (P95)
- Liquidity quote SLA breach >150ms
- Risk mode escalation to High
- Settlement batch failures
- Compliance check failures

## License

Apache License 2.0 - see [LICENSE](LICENSE) file.

## Support

For issues and feature requests, please open a GitHub issue.