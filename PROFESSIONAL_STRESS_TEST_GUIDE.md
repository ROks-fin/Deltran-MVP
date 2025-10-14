# DelTran Professional Multi-Region Stress Test

## Overview

Professional bank-grade stress testing system targeting **3000 TPS sustained load** across a 4-bank multi-region scenario:

- **UAE Bank** (Emirates National Bank) - Dubai, GMT+4
- **Israel Bank** (Bank Leumi) - Tel Aviv, GMT+2
- **Pakistan Bank** (Habib Bank) - Karachi, GMT+5
- **India Bank** (State Bank of India) - Mumbai, GMT+5:30

## Test Focus Areas

### ✅ Clearing Windows (Timezone-Aware)
- Each bank has region-specific clearing windows
- Automatic timezone conversion
- Weekend handling (UAE/Israel: Sunday-Thursday, Others: Monday-Friday)
- Real-time window status monitoring

### ✅ FX Rate Volatility & Spikes
- Real-time exchange rate simulation
- Brownian motion volatility modeling
- Random spike injection (0.1% probability, ±5% magnitude)
- 8 currency pairs: USD, AED, ILS, PKR, INR, EUR, GBP, SAR

### ✅ Payment Netting & Multilateral Settlement
- Bilateral and multilateral netting cycles
- Gross vs. net settlement comparison
- Netting efficiency calculation
- Per-bank position tracking (creditor/debtor)

### ✅ Liquidity Provider Pool
- Multi-currency liquidity pools
- Automatic rebalancing (triggers at 20% deviation)
- Low/high threshold monitoring
- Real-time liquidity status

### ✅ Real-Time Web Dashboard Integration
- Live TPS monitoring
- Payment flow visualization
- Compliance review tracking
- Risk heatmaps
- Currency distribution charts
- WebSocket real-time updates

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      STRESS TEST CLIENT                          │
│  - 3000 TPS Load Generator                                       │
│  - 4-Bank Payment Patterns (Domestic/Regional/Cross-Regional)    │
│  - FX Rate Engine with Volatility                                │
│  - Liquidity Pool Management                                     │
└───────────────────────┬─────────────────────────────────────────┘
                        │ HTTP/REST
                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                      GATEWAY SERVICE (Go)                        │
│  - Payment Processing                                            │
│  - SWIFT MT103 Generation                                        │
│  - Idempotency Protection                                        │
│  - Circuit Breakers                                              │
│  - Real-time Metrics API                                         │
│  - WebSocket Broadcasting                                        │
└─────────┬───────────────────────────┬───────────────────────────┘
          │                           │
          ▼                           ▼
┌─────────────────────┐   ┌──────────────────────────┐
│   PostgreSQL 15     │   │      Redis Cluster       │
│  - Primary + 2x     │   │  - Master + 2x Replica   │
│    Replicas         │   │  - 3x Sentinel           │
│  - Connection Pool  │   │  - Session Storage       │
│    (PgBouncer)      │   │  - Rate Limiting         │
└─────────────────────┘   └──────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────┐
│                      DATABASE SCHEMA                             │
│  - Banks & Accounts                                              │
│  - Payments & Transactions                                       │
│  - Clearing Windows (timezone-aware)                             │
│  - Liquidity Pools & Balances                                    │
│  - Netting Cycles & Positions                                    │
│  - FX Rates & History                                            │
│  - Compliance Checks                                             │
│  - Audit Log (partitioned)                                       │
└─────────────────────────────────────────────────────────────────┘
```

## Prerequisites

### Required Software
- **Docker Desktop** (for Windows)
- **Python 3.9+** with pip
- **Go 1.21+** (for gateway service)
- **Node.js 18+** and npm (for web dashboard)

### Python Packages
```bash
pip install aiohttp pytz
```

### Port Requirements
- `5432` - PostgreSQL Primary
- `5433-5434` - PostgreSQL Replicas
- `6379` - Redis Master
- `6380-6381` - Redis Replicas
- `8080` - Gateway API
- `3000` - Web Dashboard

## Quick Start

### Option 1: Automated Script (Recommended)

```batch
run_professional_stress_test.bat
```

This will:
1. ✅ Check Docker availability
2. ✅ Start PostgreSQL cluster
3. ✅ Run database migrations
4. ✅ Start Redis cluster
5. ✅ Build and start Gateway service
6. ✅ Install Python dependencies
7. ✅ Execute 5-minute stress test (3000 TPS)
8. ✅ Generate detailed JSON report

### Option 2: Manual Setup

#### Step 1: Start Infrastructure

```batch
cd infra

REM Start PostgreSQL
docker-compose -f docker-compose.database.yml up -d postgres-primary

REM Wait for database to be ready
timeout /t 10

REM Start Redis
docker-compose -f docker-compose.cache.yml up -d redis-master

REM Wait for Redis to be ready
timeout /t 5
```

#### Step 2: Run Migrations

```batch
REM Core schema
docker exec -i deltran-postgres-primary psql -U deltran_app -d deltran < sql/001_core_schema.sql

REM Advanced settlement
docker exec -i deltran-postgres-primary psql -U deltran_app -d deltran < sql/002_advanced_settlement.sql

REM Test banks
docker exec -i deltran-postgres-primary psql -U deltran_app -d deltran < sql/003_init_test_banks.sql
```

#### Step 3: Start Gateway

```batch
cd gateway-go
set CGO_ENABLED=1
go build -o gateway.exe ./cmd/gateway
gateway.exe
```

#### Step 4: Start Web Dashboard (Optional)

```batch
cd deltran-web
npm install
npm run dev
```

Visit: http://localhost:3000

#### Step 5: Run Stress Test

```batch
python tests/bank_grade_multi_region_stress.py
```

## Test Configuration

### Stress Test Parameters

Located in `tests/bank_grade_multi_region_stress.py`:

```python
TARGET_TPS = 3000                 # Target transactions per second
TEST_DURATION_SECONDS = 300       # 5 minutes
RAMP_UP_SECONDS = 30              # Gradual ramp-up period

# Payment Distribution
PATTERN_WEIGHTS = {
    "domestic": 0.60,             # 60% domestic payments
    "regional": 0.25,             # 25% regional (neighboring countries)
    "cross_regional": 0.15        # 15% cross-regional
}

TYPE_WEIGHTS = {
    "retail": 0.70,               # 70% retail payments ($10-$5K)
    "corporate": 0.25,            # 25% corporate ($5K-$500K)
    "wholesale": 0.05             # 5% wholesale ($500K-$50M)
}
```

### Bank Configurations

#### UAE Bank (ENBXAEADXXX)
- Primary Currency: AED
- Secondary: USD, EUR, SAR
- Clearing Window: 08:00-16:00 Dubai Time (GMT+4)
- Active Days: Sunday-Thursday
- Daily Liquidity Limit: $500M

#### Israel Bank (LUMIILITXXX)
- Primary Currency: ILS
- Secondary: USD, EUR, GBP
- Clearing Window: 09:00-17:00 Jerusalem Time (GMT+2)
- Active Days: Sunday-Thursday
- Daily Liquidity Limit: $300M

#### Pakistan Bank (HABBPKKKXXX)
- Primary Currency: PKR
- Secondary: USD, AED, SAR
- Clearing Window: 09:00-17:00 Karachi Time (GMT+5)
- Active Days: Monday-Friday
- Daily Liquidity Limit: $200M

#### India Bank (SBININBBXXX)
- Primary Currency: INR
- Secondary: USD, EUR, AED
- Clearing Window: 10:00-18:00 Kolkata Time (GMT+5:30)
- Active Days: Monday-Saturday
- Daily Liquidity Limit: $800M

## Monitoring & Metrics

### Real-Time API Endpoints

#### System Metrics
```bash
curl http://localhost:8080/api/v1/metrics/realtime
```

Response:
```json
{
  "tps": 2987.3,
  "volume_24h": {
    "USD": 45230000.00,
    "AED": 165000000.00,
    "INR": 3750000000.00
  },
  "success_rate": 99.2,
  "pending_count": 234,
  "processing_count": 89,
  "queue_depth": 323,
  "failed_last_1h": 12,
  "settled_today": 12456,
  "active_banks": 4,
  "compliance_review": 3
}
```

#### Payment List
```bash
curl http://localhost:8080/api/v1/payments?page=1&page_size=20
```

#### Daily Metrics
```bash
curl http://localhost:8080/api/v1/metrics/daily?days=7
```

#### Bank Metrics
```bash
curl http://localhost:8080/api/v1/metrics/banks
```

#### Export Payments (CSV)
```bash
curl "http://localhost:8080/api/v1/export/payments?date_from=2025-01-01" > payments.csv
```

### WebSocket Real-Time Updates

Connect to: `ws://localhost:8080/api/v1/ws`

Events:
- `payment_update` - New payment created/updated
- `metrics_update` - System metrics changed
- `compliance_alert` - Compliance issue detected
- `liquidity_rebalance` - Liquidity pool rebalanced

### Database Queries

#### Liquidity Pool Status
```sql
SELECT * FROM deltran.v_liquidity_pool_status;
```

#### Active Netting Cycles
```sql
SELECT * FROM deltran.v_active_netting_cycles;
```

#### Clearing Window Status
```sql
SELECT
    bic_code,
    name,
    deltran.is_bank_in_clearing_window(id) as in_window
FROM deltran.banks
WHERE is_active = true;
```

#### FX Rates
```sql
SELECT * FROM deltran.v_latest_fx_rates;
```

#### Recent Liquidity Rebalances
```sql
SELECT
    pool_id,
    currency,
    amount,
    balance_before,
    balance_after,
    rebalance_reason,
    timestamp
FROM deltran.liquidity_transactions
WHERE is_rebalance = true
ORDER BY timestamp DESC
LIMIT 20;
```

## Test Results

### Output Files

After test completion, find these files:

1. **stress_test_report_YYYYMMDD_HHMMSS.json**
   - Complete test results
   - TPS statistics
   - Volume by currency
   - Pattern distribution
   - Success rate
   - Latency percentiles

Example:
```json
{
  "test_config": {
    "target_tps": 3000,
    "duration_seconds": 300,
    "banks": [...]
  },
  "results": {
    "total_payments": 900000,
    "successful": 892380,
    "failed": 7620,
    "success_rate_pct": 99.15,
    "avg_tps": 3001.2,
    "max_tps": 3245.6,
    "min_tps": 2789.1,
    "latency_ms": {
      "average": 45.3,
      "p95": 89.7,
      "p99": 156.2
    },
    "volume_by_currency": {
      "USD": "1234567890.00",
      "AED": "4567890123.00",
      ...
    },
    "liquidity_rebalances": 47
  }
}
```

### Console Output

During test execution, you'll see:

```
================================================================================
[14:32:15] STRESS TEST METRICS (T+120s)
================================================================================
  TPS: 2987.3 (target: 3000)
  Total Payments: 358,476 | Success: 356,234 | Failed: 2,242
  Success Rate: 99.37%
  Latency: avg=43.2ms | p95=87.1ms | p99=152.3ms

  Volume by Currency:
    USD: 45,678,901.23
    AED: 167,234,567.89
    ILS: 34,567,890.12
    PKR: 8,901,234,567.00
    INR: 12,345,678,901.00

  Payment Patterns:
    domestic: 215,086 (60.0%)
    regional: 89,619 (25.0%)
    cross_regional: 53,771 (15.0%)

  Transaction Types:
    retail: 250,933 (70.0%)
    corporate: 89,619 (25.0%)
    wholesale: 17,924 (5.0%)

  Clearing Window Violations: 234
  Liquidity Rebalances: 12

  Liquidity Pool Status:
    USD: 105.3% of initial (2,106,000,000 / 2,000,000,000)
    AED: 87.2% of initial (872,000,000 / 1,000,000,000)
    ILS: 112.8% of initial (564,000,000 / 500,000,000)
    PKR: 94.5% of initial (47,250,000,000 / 50,000,000,000)
    INR: 102.1% of initial (61,260,000,000 / 60,000,000,000)
================================================================================
```

## Verification Checklist

After running the stress test, verify:

### ✅ Infrastructure Health
- [ ] PostgreSQL primary is running and responsive
- [ ] Redis master is running
- [ ] Gateway service is responding to /health
- [ ] Web dashboard loads at http://localhost:3000

### ✅ Data Integrity
- [ ] All 4 banks exist in database
- [ ] Bank accounts have positive balances
- [ ] Clearing windows are configured correctly
- [ ] Liquidity pools are initialized
- [ ] FX rates are present

### ✅ Test Execution
- [ ] Test achieved target TPS (within 10%)
- [ ] Success rate > 95%
- [ ] P99 latency < 500ms
- [ ] No system crashes or errors
- [ ] JSON report generated

### ✅ Feature Validation
- [ ] Clearing window violations logged
- [ ] FX rate spikes occurred
- [ ] Liquidity rebalancing triggered
- [ ] Payments distributed across banks
- [ ] All currency pairs used

### ✅ Web Dashboard
- [ ] Real-time metrics updating
- [ ] Payment list shows recent transactions
- [ ] Charts display historical data
- [ ] No mock/cartoon data visible
- [ ] WebSocket connection established

## Troubleshooting

### Docker Not Starting

```batch
REM Check Docker Desktop is running
docker --version

REM Check running containers
docker ps

REM View logs
docker logs deltran-postgres-primary
docker logs deltran-redis-master
```

### Database Connection Failed

```batch
REM Test connection
docker exec deltran-postgres-primary pg_isready -U deltran_app -d deltran

REM Check credentials in .env
cd infra
type .env.example
```

### Gateway Build Failed

```batch
cd gateway-go

REM Ensure CGO is enabled
set CGO_ENABLED=1

REM Check Go version
go version

REM Clean and rebuild
go clean
go build -v ./cmd/gateway
```

### Low TPS

Possible causes:
- Insufficient CPU/RAM
- Docker resource limits too low
- Network latency
- Database connection pool exhausted

Solutions:
- Increase Docker Desktop resources (Settings → Resources)
- Reduce TARGET_TPS in test script
- Increase PgBouncer max connections
- Use SSD for Docker volumes

### Mock Data Still Showing

```batch
REM Verify mock_api.go was deleted
dir gateway-go\internal\server\mock_api.go

REM Rebuild gateway
cd gateway-go
go build -o gateway.exe ./cmd/gateway

REM Restart gateway service
```

## Performance Tuning

### PostgreSQL

Edit `infra/docker-compose.database.yml`:

```yaml
command:
  - postgres
  - -c
  - max_connections=500          # Increase for higher TPS
  - -c
  - shared_buffers=512MB         # More cache
  - -c
  - effective_cache_size=2GB     # Larger cache
```

### Redis

Edit `infra/docker-compose.cache.yml`:

```yaml
command: >
  redis-server
  --maxmemory 1gb                # More memory
  --maxmemory-policy allkeys-lru
```

### Gateway

Edit `gateway-go/cmd/gateway/main.go`:

```go
// Configure connection pool
db.SetMaxOpenConns(50)   // Increase for higher TPS
db.SetMaxIdleConns(10)
```

## Support & Documentation

### Files & Directories

```
MVP DelTran/
├── gateway-go/              # Go gateway service
│   ├── cmd/gateway/         # Main entry point
│   ├── internal/            # Internal packages
│   │   ├── server/          # HTTP server
│   │   ├── auth/            # Authentication
│   │   ├── compliance/      # Compliance checks
│   │   ├── iso20022/        # ISO 20022 support
│   │   └── swift/           # SWIFT integration
│   └── go.mod
│
├── deltran-web/             # Next.js web dashboard
│   ├── app/                 # App routes
│   ├── components/          # React components
│   └── hooks/               # Custom hooks
│
├── infra/                   # Infrastructure
│   ├── docker-compose.database.yml
│   ├── docker-compose.cache.yml
│   └── sql/                 # Database migrations
│       ├── 001_core_schema.sql
│       ├── 002_advanced_settlement.sql
│       └── 003_init_test_banks.sql
│
├── tests/                   # Test scripts
│   └── bank_grade_multi_region_stress.py
│
└── run_professional_stress_test.bat  # Automated runner
```

### Additional Resources

- [PostgreSQL Performance Tuning](https://wiki.postgresql.org/wiki/Performance_Optimization)
- [Redis Best Practices](https://redis.io/docs/management/optimization/)
- [Go Performance Tips](https://go.dev/doc/effective_go)
- [aiohttp Documentation](https://docs.aiohttp.org/)

## License

Proprietary - DelTran Payment System

---

**Ready for Bank-Grade Testing** 🚀

For questions or issues, check logs:
- Gateway: Console where gateway.exe is running
- PostgreSQL: `docker logs deltran-postgres-primary`
- Redis: `docker logs deltran-redis-master`
