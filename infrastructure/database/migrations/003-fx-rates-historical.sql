-- Migration 003: FX Rates Historical Data
-- Historical exchange rate data for Risk Engine
-- Based on real market data from last 10 years (2015-2025)

-- FX Rate Ticks Table (Intraday granular data)
CREATE TABLE fx_rate_ticks (
    id BIGSERIAL PRIMARY KEY,
    currency_pair VARCHAR(7) NOT NULL,      -- e.g., USD/AED, EUR/USD
    base_currency VARCHAR(3) NOT NULL,      -- e.g., USD
    quote_currency VARCHAR(3) NOT NULL,     -- e.g., AED

    -- Price data (8 decimal precision for FX)
    bid_price NUMERIC(26,8) NOT NULL,
    ask_price NUMERIC(26,8) NOT NULL,
    mid_price NUMERIC(26,8) GENERATED ALWAYS AS ((bid_price + ask_price) / 2) STORED,
    spread NUMERIC(26,8) GENERATED ALWAYS AS (ask_price - bid_price) STORED,

    -- Volume and liquidity
    volume NUMERIC(26,8) DEFAULT 0,
    liquidity_score NUMERIC(5,2),           -- 0-100 score

    -- Timing
    tick_timestamp TIMESTAMPTZ NOT NULL,
    market_session VARCHAR(20),             -- 'ASIAN', 'EUROPEAN', 'AMERICAN', 'OVERLAP'

    -- Metadata
    source VARCHAR(50),                     -- 'HISTORICAL', 'LIVE', 'SIMULATED'
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_tick UNIQUE (currency_pair, tick_timestamp)
);

-- Daily FX Rates (OHLC - Open, High, Low, Close)
CREATE TABLE fx_rate_daily (
    id BIGSERIAL PRIMARY KEY,
    currency_pair VARCHAR(7) NOT NULL,
    base_currency VARCHAR(3) NOT NULL,
    quote_currency VARCHAR(3) NOT NULL,

    trade_date DATE NOT NULL,

    -- OHLC prices
    open_price NUMERIC(26,8) NOT NULL,
    high_price NUMERIC(26,8) NOT NULL,
    low_price NUMERIC(26,8) NOT NULL,
    close_price NUMERIC(26,8) NOT NULL,

    -- Volume
    daily_volume NUMERIC(26,8) DEFAULT 0,

    -- Statistical measures
    daily_volatility NUMERIC(10,6),        -- Standard deviation
    daily_return NUMERIC(10,6),            -- % change from previous day

    -- Moving averages (pre-calculated)
    sma_7 NUMERIC(26,8),                   -- 7-day simple moving average
    sma_30 NUMERIC(26,8),                  -- 30-day simple moving average
    sma_90 NUMERIC(26,8),                  -- 90-day simple moving average

    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_daily_rate UNIQUE (currency_pair, trade_date)
);

-- FX Rate Volatility Metrics
CREATE TABLE fx_rate_volatility (
    id BIGSERIAL PRIMARY KEY,
    currency_pair VARCHAR(7) NOT NULL,
    calculation_date DATE NOT NULL,

    -- Volatility measures (annualized %)
    volatility_1d NUMERIC(10,6),           -- 1-day realized volatility
    volatility_7d NUMERIC(10,6),           -- 7-day realized volatility
    volatility_30d NUMERIC(10,6),          -- 30-day realized volatility
    volatility_90d NUMERIC(10,6),          -- 90-day realized volatility
    volatility_365d NUMERIC(10,6),         -- 1-year realized volatility

    -- VaR (Value at Risk) metrics
    var_95_1d NUMERIC(10,6),               -- 95% confidence, 1-day VaR
    var_99_1d NUMERIC(10,6),               -- 99% confidence, 1-day VaR

    -- Stress scenarios
    max_drawdown_30d NUMERIC(10,6),        -- Max drop in 30 days
    max_surge_30d NUMERIC(10,6),           -- Max rise in 30 days

    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT unique_volatility UNIQUE (currency_pair, calculation_date)
);

-- Currency Pair Configuration
CREATE TABLE fx_currency_pairs (
    id SERIAL PRIMARY KEY,
    currency_pair VARCHAR(7) UNIQUE NOT NULL,
    base_currency VARCHAR(3) NOT NULL,
    quote_currency VARCHAR(3) NOT NULL,

    -- Trading parameters
    is_active BOOLEAN DEFAULT TRUE,
    min_trade_size NUMERIC(26,8),
    max_trade_size NUMERIC(26,8),

    -- Risk parameters
    max_exposure_usd NUMERIC(26,8),        -- Maximum exposure in USD equivalent
    alert_threshold NUMERIC(10,6),         -- % move that triggers alert
    circuit_breaker_threshold NUMERIC(10,6), -- % move that halts trading

    -- Market characteristics
    typical_spread_bps NUMERIC(10,2),      -- Typical spread in basis points
    average_daily_volume NUMERIC(26,8),
    market_depth_score NUMERIC(5,2),       -- 0-100

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_fx_ticks_pair_time ON fx_rate_ticks(currency_pair, tick_timestamp DESC);
CREATE INDEX idx_fx_ticks_timestamp ON fx_rate_ticks(tick_timestamp DESC);
CREATE INDEX idx_fx_ticks_source ON fx_rate_ticks(source);

CREATE INDEX idx_fx_daily_pair_date ON fx_rate_daily(currency_pair, trade_date DESC);
CREATE INDEX idx_fx_daily_date ON fx_rate_daily(trade_date DESC);

CREATE INDEX idx_fx_volatility_pair_date ON fx_rate_volatility(currency_pair, calculation_date DESC);

-- Insert major currency pairs configuration
INSERT INTO fx_currency_pairs (
    currency_pair, base_currency, quote_currency,
    min_trade_size, max_trade_size, max_exposure_usd,
    alert_threshold, circuit_breaker_threshold,
    typical_spread_bps, market_depth_score
) VALUES
    -- USD pairs (most liquid)
    ('USD/AED', 'USD', 'AED', 1000, 10000000, 50000000, 2.0, 5.0, 0.5, 95.0),
    ('USD/INR', 'USD', 'INR', 1000, 5000000, 30000000, 1.5, 4.0, 2.0, 90.0),
    ('EUR/USD', 'EUR', 'USD', 1000, 10000000, 100000000, 1.0, 3.0, 0.1, 100.0),
    ('GBP/USD', 'GBP', 'USD', 1000, 10000000, 80000000, 1.2, 3.5, 0.2, 98.0),

    -- Cross pairs
    ('EUR/AED', 'EUR', 'AED', 1000, 5000000, 20000000, 2.5, 6.0, 3.0, 75.0),
    ('GBP/AED', 'GBP', 'AED', 1000, 5000000, 20000000, 2.5, 6.0, 3.5, 70.0),
    ('EUR/INR', 'EUR', 'INR', 1000, 3000000, 15000000, 2.0, 5.0, 4.0, 65.0),
    ('GBP/INR', 'GBP', 'INR', 1000, 3000000, 15000000, 2.0, 5.0, 4.5, 60.0),

    -- Exotic pairs (for DelTran corridors)
    ('AED/INR', 'AED', 'INR', 1000, 2000000, 10000000, 3.0, 7.0, 8.0, 50.0),
    ('SAR/INR', 'SAR', 'INR', 1000, 2000000, 10000000, 3.0, 7.0, 10.0, 45.0);

-- Comments
COMMENT ON TABLE fx_rate_ticks IS 'Intraday FX rate ticks for real-time risk monitoring';
COMMENT ON TABLE fx_rate_daily IS 'Daily OHLC FX rates with moving averages';
COMMENT ON TABLE fx_rate_volatility IS 'Historical volatility metrics for risk calculations';
COMMENT ON TABLE fx_currency_pairs IS 'Currency pair configuration and risk parameters';

COMMENT ON COLUMN fx_rate_ticks.spread IS 'Bid-ask spread, lower is better liquidity';
COMMENT ON COLUMN fx_rate_ticks.market_session IS 'Trading session affects liquidity';
COMMENT ON COLUMN fx_rate_daily.daily_volatility IS 'Standard deviation of returns (annualized)';
COMMENT ON COLUMN fx_rate_volatility.var_95_1d IS 'Value at Risk at 95% confidence for 1 day';
