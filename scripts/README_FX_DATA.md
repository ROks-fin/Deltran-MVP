# Quick Start: FX Historical Data

## Что это?

Реалистичные исторические данные валютных курсов (2015-2025) для демонстрации работы **Risk Engine**:
- 10 валютных пар (USD/AED, USD/INR, EUR/USD, GBP/USD, и др.)
- ~49,000 записей (daily OHLC + volatility metrics)
- Включены кризисные периоды (Brexit, COVID-19, Ukraine)
- Realistic parameters: volatility, spreads, correlations

## Установка (3 шага)

### 1. Применить миграцию базы данных

```bash
cd "c:\Users\User\Desktop\Deltran MVP"

psql -h localhost -U postgres -d deltran -f infrastructure/database/migrations/003-fx-rates-historical.sql
```

**Что создается:**
- `fx_rate_ticks` - Intraday тики (минутные данные)
- `fx_rate_daily` - Daily OHLC + moving averages
- `fx_rate_volatility` - Volatility metrics (VaR, drawdowns)
- `fx_currency_pairs` - Configuration (10 pairs pre-configured)

### 2. Установить Python зависимости

```bash
pip install numpy pandas psycopg2-binary
```

### 3. Сгенерировать данные

```bash
python scripts/generate_fx_historical_data.py
```

**Время выполнения:** ~5-10 минут

**Процесс:**
```
DelTran FX Historical Data Generator
======================================================================
Generating data for 10 currency pairs
Date range: 2015-01-01 to 2025-01-01 (10 years)
Crisis periods included: 3
======================================================================

Processing USD/AED
  Initial rate: 3.6725
  Volatility: 0.1%
  Spread: 0.5 bps
Generating 2608 days of data for USD/AED...
Inserted 2608 daily records for USD/AED
Inserted 2518 volatility records for USD/AED
✓ Completed USD/AED

... (9 more pairs) ...

✓ All historical data generated successfully!

Data summary:
  - Daily OHLC data: ~2,500 records per pair
  - Volatility metrics: ~2,400 records per pair
  - Total pairs: 10
  - Total records: ~49,000
```

## Проверка данных

```sql
-- Подключиться к базе
psql -h localhost -U postgres -d deltran

-- Проверить количество записей
SELECT
    currency_pair,
    COUNT(*) as num_days,
    MIN(trade_date) as first_date,
    MAX(trade_date) as last_date
FROM fx_rate_daily
GROUP BY currency_pair
ORDER BY currency_pair;

-- Проверить волатильность
SELECT
    currency_pair,
    calculation_date,
    volatility_30d,
    var_99_1d,
    max_drawdown_30d
FROM fx_rate_volatility
WHERE calculation_date >= '2024-01-01'
ORDER BY volatility_30d DESC
LIMIT 10;

-- Найти кризисные периоды (большие движения)
SELECT
    trade_date,
    currency_pair,
    daily_return,
    daily_volatility
FROM fx_rate_daily
WHERE ABS(daily_return) > 3  -- >3% движение за день
ORDER BY trade_date DESC, ABS(daily_return) DESC
LIMIT 20;
```

**Ожидаемый результат:** Вы увидите спайки волатильности в периоды Brexit (2016), COVID-19 (2020), Ukraine conflict (2022).

## Что дальше?

### Использование в Risk Engine

Данные готовы для:

1. **Position Limit Monitoring**
   ```sql
   -- Динамический лимит на основе волатильности
   SELECT currency_pair, max_exposure_usd,
          CASE WHEN volatility_30d > 10 THEN max_exposure_usd * 0.5
               ELSE max_exposure_usd END AS adjusted_limit
   FROM fx_currency_pairs
   JOIN fx_rate_volatility USING(currency_pair);
   ```

2. **Circuit Breaker Checks**
   ```sql
   -- Проверка превышения thresholds
   SELECT * FROM fx_rate_daily d
   JOIN fx_currency_pairs p USING(currency_pair)
   WHERE ABS(d.daily_return) > p.alert_threshold;
   ```

3. **VaR Calculations**
   ```sql
   -- Value at Risk по всем парам
   SELECT currency_pair, var_95_1d, var_99_1d
   FROM fx_rate_volatility
   WHERE calculation_date = (SELECT MAX(calculation_date) FROM fx_rate_volatility);
   ```

### Визуализация (опционально)

Можно создать Grafana dashboard для мониторинга:
- Real-time spread monitoring
- Volatility trends
- Circuit breaker alerts

## Валютные пары

| Pair | Rate (avg) | Volatility | Liquidity | DelTran Use Case |
|------|-----------|------------|-----------|------------------|
| **USD/AED** | 3.6725 | 0.1% | 95 | Pegged (stable) |
| **USD/INR** | 75.50 | 6% | 90 | **India corridor** |
| **EUR/USD** | 1.1200 | 8% | 100 | Global benchmark |
| **GBP/USD** | 1.3000 | 10% | 98 | Brexit volatility |
| **AED/INR** | 20.56 | 6% | 50 | **KEY: UAE-India direct!** |
| **EUR/AED** | 4.1100 | 8% | 75 | Europe-UAE flows |
| **GBP/AED** | 4.7700 | 10% | 70 | UK-UAE flows |
| **EUR/INR** | 84.60 | 9% | 65 | Europe-India flows |
| **GBP/INR** | 98.15 | 11% | 60 | UK-India flows |
| **SAR/INR** | 20.13 | 6% | 45 | Saudi-India flows |

**Ключевая пара для DelTran: AED/INR** - прямой коридор UAE→India!

## Кризисные периоды

Генератор включает реалистичную повышенную волатильность:

1. **Brexit (23 Jun - 31 Dec 2016)**
   - Волатильность × 2.5
   - GBP пары особенно волатильны

2. **COVID-19 (1 Mar - 30 Jun 2020)**
   - Волатильность × 3.0
   - Все пары затронуты

3. **Ukraine Conflict (24 Feb - 31 May 2022)**
   - Волатильность × 2.0
   - EUR/USD наиболее затронут

## Troubleshooting

### Ошибка: "psycopg2 not found"
```bash
pip install psycopg2-binary
```

### Ошибка: "connection refused"
```bash
# Проверить что PostgreSQL запущен
docker ps | grep postgres

# Или запустить:
docker run -d --name deltran-postgres \
  -p 5432:5432 \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=deltran2025 \
  -e POSTGRES_DB=deltran \
  postgres:14
```

### Ошибка: "table already exists"
Это нормально - скрипт использует `ON CONFLICT DO UPDATE`, можно перезапустить безопасно.

## Файлы

- **Migration**: `infrastructure/database/migrations/003-fx-rates-historical.sql`
- **Generator**: `scripts/generate_fx_historical_data.py`
- **Documentation**: `docs/FX_HISTORICAL_DATA.md` (полная документация)
- **This file**: `scripts/README_FX_DATA.md` (quick start)

## Контакты

Для вопросов по данным или интеграции с Risk Engine - см. документацию `FINAL_STATUS.md` и `IMPLEMENTATION_GUIDE.md`.

---

**Ready for Risk Engine demo! 🎉**
