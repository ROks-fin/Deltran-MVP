# FX Historical Data for Risk Engine

## Overview

DelTran Risk Engine использует реалистичные исторические данные валютных курсов за последние 10 лет (2015-2025) для:
- Risk limit monitoring
- Volatility-based position sizing
- Value at Risk (VaR) calculations
- Stress testing scenarios
- FX exposure management

## Data Structure

### 1. **fx_rate_ticks** - Intraday Tick Data

Гранулярные данные с интервалом минут для внутридневного мониторинга.

```sql
SELECT
    currency_pair,
    tick_timestamp,
    bid_price,
    ask_price,
    mid_price,      -- Автоматически вычисляется
    spread,         -- Bid-ask spread
    volume,
    liquidity_score,
    market_session  -- ASIAN, EUROPEAN, AMERICAN, OVERLAP
FROM fx_rate_ticks
WHERE currency_pair = 'USD/INR'
  AND tick_timestamp >= NOW() - INTERVAL '1 day'
ORDER BY tick_timestamp DESC
LIMIT 100;
```

**Key Fields:**
- `bid_price/ask_price`: 8-знаковая точность для FX
- `spread`: Индикатор ликвидности (меньше = лучше)
- `market_session`: Влияет на ликвидность и волатильность
- `liquidity_score`: 0-100, выше при перекрытии сессий

### 2. **fx_rate_daily** - Daily OHLC Data

Дневные данные с OHLC (Open, High, Low, Close) и moving averages.

```sql
SELECT
    trade_date,
    open_price,
    high_price,
    low_price,
    close_price,
    daily_volume,
    daily_return,      -- % change from previous day
    daily_volatility,  -- 30-day rolling volatility
    sma_7,            -- 7-day moving average
    sma_30,           -- 30-day moving average
    sma_90            -- 90-day moving average
FROM fx_rate_daily
WHERE currency_pair = 'EUR/USD'
  AND trade_date >= '2024-01-01'
ORDER BY trade_date DESC;
```

**Moving Averages Usage:**
- **SMA-7**: Short-term trend (1 week)
- **SMA-30**: Medium-term trend (1 month)
- **SMA-90**: Long-term trend (1 quarter)

**Example: Detect trend changes:**
```sql
-- Golden Cross: SMA-7 crosses above SMA-30 (bullish signal)
SELECT
    trade_date,
    currency_pair,
    close_price,
    sma_7,
    sma_30,
    sma_7 - sma_30 AS spread
FROM fx_rate_daily
WHERE currency_pair = 'USD/INR'
  AND sma_7 > sma_30
  AND trade_date >= '2024-01-01'
ORDER BY trade_date DESC;
```

### 3. **fx_rate_volatility** - Volatility Metrics

Historical volatility measures для risk calculations.

```sql
SELECT
    calculation_date,
    currency_pair,
    volatility_1d,    -- 1-day realized vol
    volatility_7d,    -- 7-day realized vol
    volatility_30d,   -- 30-day realized vol (most common)
    volatility_90d,   -- 90-day realized vol
    volatility_365d,  -- 1-year realized vol
    var_95_1d,        -- Value at Risk (95% confidence)
    var_99_1d,        -- Value at Risk (99% confidence)
    max_drawdown_30d, -- Max drop in 30 days
    max_surge_30d     -- Max rise in 30 days
FROM fx_rate_volatility
WHERE currency_pair = 'GBP/USD'
  AND calculation_date >= '2023-01-01'
ORDER BY calculation_date DESC;
```

**VaR Interpretation:**
- `var_95_1d = -1.5`: 5% вероятность потерь > 1.5% за день
- `var_99_1d = -2.3`: 1% вероятность потерь > 2.3% за день

### 4. **fx_currency_pairs** - Configuration

Configuration and risk parameters для каждой валютной пары.

```sql
SELECT
    currency_pair,
    is_active,
    max_exposure_usd,           -- Max position in USD equivalent
    alert_threshold,            -- % move that triggers alert
    circuit_breaker_threshold,  -- % move that halts trading
    typical_spread_bps,         -- Normal spread in basis points
    market_depth_score          -- Liquidity indicator
FROM fx_currency_pairs
WHERE is_active = true
ORDER BY market_depth_score DESC;
```

## Currency Pairs

### Major Pairs (Highest Liquidity)

| Pair | Approx Rate (2015-2025 avg) | Volatility | Use Case |
|------|----------------------------|------------|----------|
| **EUR/USD** | 1.1200 | 8% | Most liquid global pair |
| **GBP/USD** | 1.3000 | 10% | Brexit volatility included |
| **USD/AED** | 3.6725 | 0.1% | Pegged (very stable) |
| **USD/INR** | 75.50 | 6% | India corridor |

### Cross Pairs

| Pair | Approx Rate | Volatility | Use Case |
|------|-------------|------------|----------|
| **EUR/AED** | 4.1100 | 8% | Europe-UAE flows |
| **GBP/AED** | 4.7700 | 10% | UK-UAE flows |
| **EUR/INR** | 84.60 | 9% | Europe-India flows |
| **GBP/INR** | 98.15 | 11% | UK-India flows |

### Exotic Pairs (Lower Liquidity)

| Pair | Approx Rate | Volatility | Use Case |
|------|-------------|------------|----------|
| **AED/INR** | 20.56 | 6% | Direct UAE-India corridor (KEY for DelTran!) |
| **SAR/INR** | 20.13 | 6% | Saudi-India corridor |

## Crisis Periods Included

Historical data включает повышенную волатильность в периоды:

1. **Brexit (Jun-Dec 2016)**: Volatility multiplier 2.5x
2. **COVID-19 (Mar-Jun 2020)**: Volatility multiplier 3.0x
3. **Ukraine Conflict (Feb-May 2022)**: Volatility multiplier 2.0x

## Setup Instructions

### 1. Apply Database Migration

```bash
psql -h localhost -U postgres -d deltran -f infrastructure/database/migrations/003-fx-rates-historical.sql
```

### 2. Generate Historical Data

```bash
# Install dependencies
pip install numpy pandas psycopg2-binary

# Run generator (takes ~5-10 minutes)
python scripts/generate_fx_historical_data.py
```

**Output:**
```
Generating data for 10 currency pairs
Date range: 2015-01-01 to 2025-01-01 (10 years)
Crisis periods included: 3

Processing USD/AED...
  Initial rate: 3.6725
  Volatility: 0.1%
  Spread: 0.5 bps
✓ Completed USD/AED

...

✓ All historical data generated successfully!

Data summary:
  - Daily OHLC data: ~2,500 records per pair
  - Volatility metrics: ~2,400 records per pair
  - Total pairs: 10
  - Total records: ~49,000
```

### 3. Verify Data

```sql
-- Check daily data
SELECT
    currency_pair,
    COUNT(*) as num_days,
    MIN(trade_date) as first_date,
    MAX(trade_date) as last_date,
    AVG(daily_volatility) as avg_volatility
FROM fx_rate_daily
GROUP BY currency_pair
ORDER BY currency_pair;

-- Check volatility spikes (crisis periods)
SELECT
    trade_date,
    currency_pair,
    close_price,
    daily_return,
    daily_volatility
FROM fx_rate_daily
WHERE ABS(daily_return) > 3  -- Returns > 3%
ORDER BY trade_date DESC, ABS(daily_return) DESC
LIMIT 20;

-- Expected: Spikes during Brexit, COVID, Ukraine periods
```

## Risk Engine Integration

### Example: Position Limit Based on Volatility

```sql
-- Calculate dynamic position limit based on 30-day volatility
WITH latest_vol AS (
    SELECT
        currency_pair,
        volatility_30d,
        var_99_1d
    FROM fx_rate_volatility
    WHERE calculation_date = (SELECT MAX(calculation_date) FROM fx_rate_volatility)
)
SELECT
    cp.currency_pair,
    cp.max_exposure_usd AS base_limit,
    lv.volatility_30d,
    -- Reduce limit if volatility is high
    CASE
        WHEN lv.volatility_30d < 5 THEN cp.max_exposure_usd
        WHEN lv.volatility_30d < 10 THEN cp.max_exposure_usd * 0.7
        WHEN lv.volatility_30d < 15 THEN cp.max_exposure_usd * 0.5
        ELSE cp.max_exposure_usd * 0.3
    END AS adjusted_limit,
    lv.var_99_1d AS expected_max_loss_pct
FROM fx_currency_pairs cp
JOIN latest_vol lv ON cp.currency_pair = lv.currency_pair
WHERE cp.is_active = true
ORDER BY lv.volatility_30d DESC;
```

### Example: Circuit Breaker Check

```sql
-- Check if any pair hit circuit breaker threshold today
WITH today_moves AS (
    SELECT
        currency_pair,
        open_price,
        close_price,
        high_price,
        low_price,
        ABS((close_price - open_price) / open_price * 100) AS daily_move_pct,
        ((high_price - low_price) / open_price * 100) AS intraday_range_pct
    FROM fx_rate_daily
    WHERE trade_date = CURRENT_DATE
)
SELECT
    tm.currency_pair,
    tm.daily_move_pct,
    tm.intraday_range_pct,
    cp.circuit_breaker_threshold,
    CASE
        WHEN tm.daily_move_pct > cp.circuit_breaker_threshold THEN 'HALT TRADING'
        WHEN tm.daily_move_pct > cp.alert_threshold THEN 'ALERT'
        ELSE 'NORMAL'
    END AS status
FROM today_moves tm
JOIN fx_currency_pairs cp ON tm.currency_pair = cp.currency_pair
WHERE tm.daily_move_pct > cp.alert_threshold
ORDER BY tm.daily_move_pct DESC;
```

### Example: Real-time Spread Monitoring

```sql
-- Monitor liquidity via bid-ask spread
SELECT
    currency_pair,
    tick_timestamp,
    bid_price,
    ask_price,
    spread,
    (spread / mid_price * 10000) AS spread_bps,
    liquidity_score,
    market_session
FROM fx_rate_ticks
WHERE tick_timestamp >= NOW() - INTERVAL '1 hour'
  AND currency_pair IN ('USD/INR', 'AED/INR')  -- Key DelTran pairs
ORDER BY spread_bps DESC
LIMIT 10;

-- Wide spreads indicate low liquidity - may need to adjust pricing
```

## Typical Values Reference

### Spread (Basis Points)
- **EUR/USD**: 0.1-0.5 bps (most liquid)
- **USD/INR**: 2-5 bps
- **AED/INR**: 8-15 bps (exotic, wider spread)

### Volatility (Annualized %)
- **USD/AED**: <0.5% (pegged)
- **EUR/USD**: 6-10%
- **GBP/USD**: 8-12% (higher due to Brexit)
- **USD/INR**: 4-8%
- **Emerging pairs**: 8-15%

### Market Sessions
- **Asian (00:00-08:00 UTC)**: Lower liquidity, AUD/JPY most active
- **European (08:00-16:00 UTC)**: High liquidity, EUR/USD peak
- **American (16:00-00:00 UTC)**: High liquidity, USD pairs active
- **Overlap (08:00-12:00, 16:00-17:00)**: Highest liquidity, tightest spreads

## Best Practices

1. **Always check volatility before large trades**
   - Use `volatility_30d` as baseline
   - Scale position size inversely to volatility

2. **Monitor during crisis periods**
   - Check `max_drawdown_30d` and `max_surge_30d`
   - Reduce exposure when >5% drawdown

3. **Respect market sessions**
   - Execute major trades during OVERLAP sessions
   - Avoid exotic pairs during ASIAN session (low liquidity)

4. **Use moving averages for trends**
   - SMA-7 > SMA-30 = uptrend (safe to buy base currency)
   - SMA-7 < SMA-30 = downtrend (caution)

5. **Set dynamic limits**
   - Base limits on `max_exposure_usd`
   - Adjust down during high volatility periods

## Next Steps

- [ ] Integrate with Risk Engine service
- [ ] Add real-time rate updates via FX data providers (Reuters, Bloomberg)
- [ ] Implement correlation matrix between pairs
- [ ] Add stress test scenarios (e.g., 2008 financial crisis simulation)
- [ ] Create Grafana dashboards for volatility monitoring

## Resources

- Migration file: `infrastructure/database/migrations/003-fx-rates-historical.sql`
- Generator script: `scripts/generate_fx_historical_data.py`
- Sample queries: Above examples
- Risk Engine integration: TBD (Priority 1 next steps)
