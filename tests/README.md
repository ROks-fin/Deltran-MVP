# DelTran System - Testing Suite

Comprehensive testing suite for the DelTran distributed settlement system.

## Test Files

### 1. Component Test Suite (`component_test_suite.py`)
Tests each system component individually:
- ✅ Gateway Service (health, banks, payments)
- ✅ Settlement Engine (batches, netting, settlement)
- ✅ Compliance Service (limits, checks, status)
- ✅ Risk Engine (assessment, exposure, VaR)
- ✅ Reconciliation Service (status, discrepancies)
- ✅ Message Bus (publish, stats)
- ✅ Ledger Core (entries, balances, history)

### 2. Multi-Bank Stress Test (`stress_test_multibank.py`)
Simulates realistic multi-bank integration:
- 🏦 5 Banks: US, Germany, UK, Japan, Switzerland
- 💱 5 Currencies: USD, EUR, GBP, JPY, CHF
- 📊 50 TPS target rate
- ⏱️ 5 minute duration (~15,000 transactions)
- 📈 Real-time metrics collection
- 🎯 Performance analysis (P95, P99 latency)

### 3. Test Runner Scripts
- **Linux/Mac**: `run_all_tests.sh`
- **Windows**: `run_all_tests.bat`

## Banks Configuration

| Bank ID | Name | Country | Currency | SWIFT | Initial Balance |
|---------|------|---------|----------|-------|----------------|
| BANK001 | Global Bank America | USA | USD | GLBAUS33 | $10M |
| BANK002 | European Finance Group | Germany | EUR | EURFDE33 | €8M |
| BANK003 | London Sterling Bank | UK | GBP | LSTRGB2L | £7M |
| BANK004 | Tokyo International Bank | Japan | JPY | TOINJPJT | ¥1.2B |
| BANK005 | Swiss Private Banking | Switzerland | CHF | SWPBCHZZ | CHF 9M |

## Running Tests

### Prerequisites
```bash
# Install Python dependencies
pip install aiohttp asyncio

# Start the Gateway service
cd gateway
cargo run --release
```

### Run All Tests (Automated)

**Linux/Mac:**
```bash
cd tests
chmod +x run_all_tests.sh
./run_all_tests.sh
```

**Windows:**
```cmd
cd tests
run_all_tests.bat
```

### Run Individual Tests

**Component Tests:**
```bash
python tests/component_test_suite.py
```

**Stress Test:**
```bash
python tests/stress_test_multibank.py
```

## Expected Results

### Component Tests
- Should see ✓ for each successfully tested component
- Some tests may warn if services are not fully implemented yet
- Typical run time: 10-30 seconds

### Stress Test
Sample output:
```
Configuration:
  Banks: 5
  Currencies: USD, EUR, GBP, JPY, CHF
  Target Rate: 50 TPS
  Duration: 300 seconds
  Total Expected Transactions: ~15000

STRESS TEST REPORT
==================
Test Duration: 300.00 seconds
Total Requests: 15000
Successful: 14850 (99.00%)
Failed: 150 (1.00%)

Response Times:
  Min: 12.50ms
  Max: 456.78ms
  Mean: 89.34ms
  Median: 76.12ms
  P95: 145.67ms
  P99: 234.89ms

Throughput: 49.50 TPS

Transactions by Currency:
  USD: 3045 transactions, 45,678,901.23 USD
  EUR: 2987 transactions, 37,890,234.56 EUR
  GBP: 3012 transactions, 32,123,456.78 GBP
  JPY: 2976 transactions, 4,567,890,123.45 JPY
  CHF: 2980 transactions, 39,876,543.21 CHF
```

## Test Scenarios

### Payment Flow Tests
1. **Cross-border payments** (different currencies)
2. **Same-currency transfers** (domestic)
3. **Large value payments** (>$100k)
4. **Small retail payments** (<$1k)
5. **Concurrent transactions** (multiple simultaneous)

### Compliance Tests
1. Limit enforcement (daily/transaction limits)
2. Sanctions screening
3. Risk assessment
4. Regulatory reporting

### Settlement Tests
1. Batch creation and processing
2. Netting calculations
3. Settlement finalization
4. Reconciliation

### Error Scenarios
1. Rate limiting (429 responses)
2. Timeout handling
3. Invalid data rejection
4. Duplicate transaction detection

## Performance Benchmarks

### Target Metrics
- ✅ **Latency P95**: < 200ms
- ✅ **Latency P99**: < 500ms
- ✅ **Throughput**: 50+ TPS
- ✅ **Success Rate**: > 99%
- ✅ **Settlement Rate**: > 98%

### System Limits
- **Max Concurrent Connections**: 200
- **Rate Limit**: 100 req/sec per client
- **Batch Size**: 1000 payments
- **Settlement Cycle**: 60 seconds

## Troubleshooting

### Gateway Not Running
```
Error: Gateway is not running at http://localhost:8080
Solution: Start gateway with: cd gateway && cargo run --release
```

### High Failure Rate
```
Failed: 5000 (33%)
Possible causes:
- Rate limiting (reduce TPS)
- Database connection issues
- Memory constraints
Solution: Check gateway logs, reduce load, increase resources
```

### Timeout Errors
```
Error: Timeout
Possible causes:
- Gateway overloaded
- Network issues
- Database slow
Solution: Reduce concurrent requests, check system resources
```

## Web Interface Integration

The test suite generates real data that feeds into the web interface:

1. **Dashboard Metrics**
   - Total volume calculated from payments
   - Active payment count
   - Settlement rate from completed batches
   - Average processing time

2. **Transaction Table**
   - Real payment data from `/api/payments`
   - Live status updates
   - Filtering and sorting

3. **Analytics**
   - Risk heatmap from real risk scores
   - Currency distribution from payment volumes
   - Payment flow visualization

## Continuous Integration

These tests can be integrated into CI/CD pipelines:

```yaml
# Example GitHub Actions
- name: Run DelTran Tests
  run: |
    ./tests/run_all_tests.sh
```

## Test Results Location

All test results are saved to `tests/results/`:
- `component_tests_YYYYMMDD_HHMMSS.log`
- `stress_test_YYYYMMDD_HHMMSS.log`

## Next Steps

After successful testing:
1. ✅ Verify web interface shows real data
2. ✅ Check all components are operational
3. ✅ Review performance metrics
4. ✅ Validate compliance checks
5. ✅ Test reconciliation accuracy
6. 🚀 Deploy to production

## Support

For issues or questions:
- Check gateway logs: `gateway/logs/`
- Review test output in `tests/results/`
- Verify service configuration
- Check database connectivity
