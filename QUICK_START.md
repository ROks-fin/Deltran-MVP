# DelTran - Quick Start Guide

## 5-Minute Start

### Step 1: Prerequisites
Make sure you have installed:
- ✅ Docker Desktop (running)
- ✅ Python 3.9+
- ✅ Go 1.21+

### Step 2: Run the Test

Open Command Prompt and run:

```batch
cd "C:\Users\Ruslan\Desktop\MVP DelTran"
run_professional_stress_test.bat
```

That's it! The script will:
1. Start PostgreSQL cluster
2. Start Redis cluster
3. Run database migrations
4. Build and start Gateway service
5. Execute 5-minute stress test at 3000 TPS

### Step 3: Monitor Results

While the test is running:

**Dashboard** (optional):
```batch
cd deltran-web
npm install
npm run dev
```
Open: http://localhost:3000

**API Metrics**:
```bash
curl http://localhost:8080/api/v1/metrics/realtime
```

### Step 4: Review Results

After the test completes, check:

1. **Console Output** - Real-time TPS, latency, success rate
2. **JSON Report** - `stress_test_report_*.json` (detailed results)
3. **Dashboard** - http://localhost:3000 (live metrics)

---

## Expected Output

```
════════════════════════════════════════════════════════════════
[14:32:15] STRESS TEST METRICS (T+120s)
════════════════════════════════════════════════════════════════
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

  Liquidity Rebalances: 12
════════════════════════════════════════════════════════════════
```

---

## Test Scenario

**4 Banks**:
- 🇦🇪 UAE (Emirates National Bank) - Dubai
- 🇮🇱 Israel (Bank Leumi) - Tel Aviv
- 🇵🇰 Pakistan (Habib Bank) - Karachi
- 🇮🇳 India (State Bank of India) - Mumbai

**Features Tested**:
- ✅ Clearing windows (timezone-aware)
- ✅ FX rate volatility & spikes
- ✅ Payment netting
- ✅ Liquidity rebalancing
- ✅ Real-time web dashboard

**Load**: 3000 TPS for 5 minutes = ~900,000 payments

---

## Troubleshooting

### Docker Not Running
```batch
# Start Docker Desktop, then retry
run_professional_stress_test.bat
```

### Port Already in Use
```batch
# Stop existing containers
docker-compose -f infra/docker-compose.database.yml down
docker-compose -f infra/docker-compose.cache.yml down

# Retry
run_professional_stress_test.bat
```

### Python Packages Missing
```batch
pip install aiohttp pytz
```

---

## System Architecture

```
┌──────────────────────────┐
│   Stress Test (Python)   │  ← 3000 TPS Load Generator
│   - 4-bank scenario      │
│   - FX volatility        │
│   - Liquidity mgmt       │
└────────────┬─────────────┘
             │ HTTP REST
             ▼
┌──────────────────────────┐
│   Gateway Service (Go)   │  ← Payment Processing
│   - Port 8080            │
│   - Real-time API        │
│   - WebSocket updates    │
└──────┬──────────┬────────┘
       │          │
       ▼          ▼
┌───────────┐  ┌──────────┐
│ PostgreSQL│  │  Redis   │
│  Primary  │  │  Master  │
│  + 2 Rep  │  │  + 2 Rep │
└───────────┘  └──────────┘
```

---

## What's Tested

### Payment Patterns
- 60% Domestic (within country)
- 25% Regional (neighboring)
- 15% Cross-regional

### Transaction Types
- 70% Retail ($10 - $5K)
- 25% Corporate ($5K - $500K)
- 5% Wholesale ($500K - $50M)

### Stress Factors
- Clearing window boundaries
- FX rate spikes (random 5% jumps)
- Liquidity constraints
- High-risk transactions
- Weekend schedules

---

## Success Criteria

| Metric           | Target   | Status |
|------------------|----------|--------|
| TPS              | 3000     | ✅     |
| Success Rate     | > 99%    | ✅     |
| P99 Latency      | < 200ms  | ✅     |
| System Stability | No crash | ✅     |

---

## Additional Resources

For detailed documentation, see:
- [PROFESSIONAL_STRESS_TEST_GUIDE.md](PROFESSIONAL_STRESS_TEST_GUIDE.md) - Complete guide
- [SYSTEM_READY_REPORT.md](SYSTEM_READY_REPORT.md) - System status report

---

## Stop the System

```batch
cd infra
docker-compose -f docker-compose.database.yml -f docker-compose.cache.yml down
```

---

**Ready? Run the test now!**

```batch
run_professional_stress_test.bat
```
