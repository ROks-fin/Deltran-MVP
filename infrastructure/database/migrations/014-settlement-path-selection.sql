-- Migration 014: Settlement Path Selection
-- Stores Risk Engine settlement path decisions for audit and analysis
-- Paths: INSTANT_BUY (via Liquidity Engine), HEDGING (full/partial), CLEARING

-- ============================================================================
-- PART 1: SETTLEMENT PATH RECOMMENDATIONS TABLE
-- ============================================================================

CREATE TABLE IF NOT EXISTS settlement_path_recommendations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID NOT NULL REFERENCES transactions(id),

    -- Path selection
    path_type VARCHAR(20) NOT NULL, -- 'INSTANT_BUY', 'FULL_HEDGE', 'PARTIAL_HEDGE', 'CLEARING'
    confidence NUMERIC(3,2) NOT NULL, -- 0.00 - 1.00
    estimated_cost_bps INTEGER NOT NULL,
    estimated_time_ms BIGINT NOT NULL,

    -- Market conditions at decision time
    volatility_score NUMERIC(5,2),
    risk_score NUMERIC(5,2),
    liquidity_depth VARCHAR(20), -- 'DEEP', 'NORMAL', 'THIN', 'STRESSED'

    -- Provider selection (for INSTANT_BUY)
    fx_provider VARCHAR(50),
    estimated_fx_rate NUMERIC(20,8),

    -- Hedge details (for FULL_HEDGE, PARTIAL_HEDGE)
    hedge_ratio NUMERIC(3,2), -- 0.00 - 1.00
    hedge_instrument VARCHAR(100),
    hedge_notional NUMERIC(20,2),

    -- Clearing details (for CLEARING, PARTIAL_HEDGE)
    clearing_window_id UUID REFERENCES clearing_windows(id),
    expected_netting_benefit_pct NUMERIC(5,2),

    -- Decision reasoning
    reasoning TEXT NOT NULL,
    market_conditions JSONB,

    -- Timestamps
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    executed_at TIMESTAMPTZ,
    execution_status VARCHAR(20) DEFAULT 'PENDING', -- 'PENDING', 'EXECUTED', 'FAILED', 'CANCELLED'

    CONSTRAINT valid_path_type CHECK (path_type IN ('INSTANT_BUY', 'FULL_HEDGE', 'PARTIAL_HEDGE', 'CLEARING'))
);

-- Indexes for fast lookup
CREATE INDEX IF NOT EXISTS idx_spr_transaction ON settlement_path_recommendations(transaction_id);
CREATE INDEX IF NOT EXISTS idx_spr_path_type ON settlement_path_recommendations(path_type, calculated_at DESC);
CREATE INDEX IF NOT EXISTS idx_spr_status ON settlement_path_recommendations(execution_status) WHERE execution_status = 'PENDING';
CREATE INDEX IF NOT EXISTS idx_spr_calculated ON settlement_path_recommendations(calculated_at DESC);

-- Unique constraint to ensure one recommendation per transaction
CREATE UNIQUE INDEX IF NOT EXISTS idx_spr_unique_txn ON settlement_path_recommendations(transaction_id);

COMMENT ON TABLE settlement_path_recommendations IS 'Settlement path decisions from Risk Engine: Instant Buy, Hedging, or Clearing';
COMMENT ON COLUMN settlement_path_recommendations.path_type IS 'Selected settlement path: INSTANT_BUY (liquidity buy), FULL_HEDGE (100% hedge), PARTIAL_HEDGE, CLEARING (multilateral netting)';

-- ============================================================================
-- PART 2: FX PROVIDERS TABLE (for Liquidity Engine)
-- ============================================================================

CREATE TABLE IF NOT EXISTS fx_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_code VARCHAR(20) NOT NULL UNIQUE,
    provider_name VARCHAR(100) NOT NULL,

    -- Supported corridors
    supported_currencies TEXT[] NOT NULL, -- ['USD', 'AED', 'INR', 'EUR']

    -- Pricing
    base_spread_bps INTEGER NOT NULL, -- Base spread in basis points
    volume_discount_threshold NUMERIC(20,2), -- Amount above which discount applies
    volume_discount_bps INTEGER, -- Discount for large volumes

    -- Performance metrics
    avg_execution_time_ms INTEGER DEFAULT 500,
    success_rate NUMERIC(5,2) DEFAULT 99.50,

    -- Status
    is_active BOOLEAN DEFAULT true,
    priority INTEGER DEFAULT 5, -- 1-10, 1 being highest priority

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Insert default FX providers
INSERT INTO fx_providers (provider_code, provider_name, supported_currencies, base_spread_bps, volume_discount_threshold, volume_discount_bps, avg_execution_time_ms, priority)
VALUES
    ('ENBD-FX', 'Emirates NBD FX', ARRAY['USD', 'AED', 'INR', 'GBP', 'EUR'], 3, 500000, 1, 300, 1),
    ('UAE-EXCHANGE', 'UAE Exchange', ARRAY['USD', 'AED', 'INR', 'PKR', 'PHP'], 8, 100000, 2, 500, 2),
    ('HDFC-FX', 'HDFC Bank FX', ARRAY['USD', 'INR', 'EUR', 'GBP'], 10, 250000, 3, 600, 3),
    ('CLS-SETTLEMENT', 'CLS Bank Settlement', ARRAY['USD', 'EUR', 'GBP', 'JPY', 'CHF'], 2, 1000000, 1, 200, 1),
    ('GLOBAL-FX-POOL', 'Global FX Pool (Aggregator)', ARRAY['USD', 'EUR', 'GBP', 'AED', 'INR', 'JPY', 'CHF'], 15, 50000, 5, 800, 5)
ON CONFLICT (provider_code) DO NOTHING;

CREATE INDEX IF NOT EXISTS idx_fx_providers_active ON fx_providers(is_active, priority) WHERE is_active = true;

COMMENT ON TABLE fx_providers IS 'FX liquidity providers for instant buy settlement path';

-- ============================================================================
-- PART 3: FX RATES CACHE TABLE
-- ============================================================================

CREATE TABLE IF NOT EXISTS fx_rates_cache (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id UUID NOT NULL REFERENCES fx_providers(id),
    from_currency VARCHAR(3) NOT NULL,
    to_currency VARCHAR(3) NOT NULL,

    -- Rates
    bid_rate NUMERIC(20,8) NOT NULL,
    ask_rate NUMERIC(20,8) NOT NULL,
    mid_rate NUMERIC(20,8) NOT NULL,
    spread_bps INTEGER NOT NULL,

    -- Validity
    valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_until TIMESTAMPTZ NOT NULL,

    -- Metadata
    source VARCHAR(50) DEFAULT 'API',
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fx_rates_pair ON fx_rates_cache(from_currency, to_currency, valid_until DESC);
CREATE INDEX IF NOT EXISTS idx_fx_rates_provider ON fx_rates_cache(provider_id, valid_until DESC);

-- Unique constraint for current rate per provider/pair
CREATE UNIQUE INDEX IF NOT EXISTS idx_fx_rates_unique
    ON fx_rates_cache(provider_id, from_currency, to_currency)
    WHERE valid_until > NOW();

COMMENT ON TABLE fx_rates_cache IS 'Cached FX rates from providers for Liquidity Engine';

-- ============================================================================
-- PART 4: FUNCTION TO SELECT BEST FX PROVIDER
-- ============================================================================

CREATE OR REPLACE FUNCTION select_best_fx_provider(
    p_from_currency VARCHAR(3),
    p_to_currency VARCHAR(3),
    p_amount NUMERIC(20,2)
) RETURNS TABLE (
    provider_id UUID,
    provider_code VARCHAR(20),
    effective_rate NUMERIC(20,8),
    effective_spread_bps INTEGER,
    estimated_cost NUMERIC(20,2)
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        fp.id as provider_id,
        fp.provider_code,
        frc.mid_rate as effective_rate,
        CASE
            WHEN p_amount >= fp.volume_discount_threshold THEN fp.base_spread_bps - COALESCE(fp.volume_discount_bps, 0)
            ELSE fp.base_spread_bps
        END as effective_spread_bps,
        (p_amount * (
            CASE
                WHEN p_amount >= fp.volume_discount_threshold THEN fp.base_spread_bps - COALESCE(fp.volume_discount_bps, 0)
                ELSE fp.base_spread_bps
            END::NUMERIC / 10000
        )) as estimated_cost
    FROM fx_providers fp
    JOIN fx_rates_cache frc ON frc.provider_id = fp.id
    WHERE fp.is_active = true
      AND p_from_currency = ANY(fp.supported_currencies)
      AND p_to_currency = ANY(fp.supported_currencies)
      AND frc.from_currency = p_from_currency
      AND frc.to_currency = p_to_currency
      AND frc.valid_until > NOW()
    ORDER BY effective_spread_bps ASC, fp.priority ASC
    LIMIT 5;
END;
$$ LANGUAGE plpgsql STABLE;

COMMENT ON FUNCTION select_best_fx_provider IS 'Returns top 5 FX providers sorted by best effective rate for given corridor and amount';

-- ============================================================================
-- PART 5: SETTLEMENT PATH ANALYTICS VIEW
-- ============================================================================

CREATE MATERIALIZED VIEW IF NOT EXISTS settlement_path_analytics AS
SELECT
    path_type,
    DATE_TRUNC('hour', calculated_at) as hour,
    COUNT(*) as total_count,
    AVG(confidence) as avg_confidence,
    AVG(estimated_cost_bps) as avg_cost_bps,
    AVG(estimated_time_ms) as avg_time_ms,
    AVG(volatility_score) as avg_volatility,
    AVG(risk_score) as avg_risk,
    COUNT(*) FILTER (WHERE execution_status = 'EXECUTED') as executed_count,
    COUNT(*) FILTER (WHERE execution_status = 'FAILED') as failed_count
FROM settlement_path_recommendations
WHERE calculated_at > NOW() - INTERVAL '7 days'
GROUP BY path_type, DATE_TRUNC('hour', calculated_at);

CREATE UNIQUE INDEX IF NOT EXISTS idx_spa_path_hour ON settlement_path_analytics(path_type, hour);

COMMENT ON MATERIALIZED VIEW settlement_path_analytics IS 'Hourly aggregated statistics for settlement path selection';

-- Function to refresh analytics
CREATE OR REPLACE FUNCTION refresh_settlement_path_analytics()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY settlement_path_analytics;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 6: UPDATE TRANSACTION_DECISIONS TO INCLUDE PATH
-- ============================================================================

ALTER TABLE transaction_decisions
ADD COLUMN IF NOT EXISTS settlement_path_type VARCHAR(20),
ADD COLUMN IF NOT EXISTS settlement_path_provider VARCHAR(50),
ADD COLUMN IF NOT EXISTS settlement_path_cost_bps INTEGER;

COMMENT ON COLUMN transaction_decisions.settlement_path_type IS 'Selected settlement path from Risk Engine';
COMMENT ON COLUMN transaction_decisions.settlement_path_provider IS 'FX provider for instant buy path';

-- ============================================================================
-- ANALYZE TABLES
-- ============================================================================

ANALYZE settlement_path_recommendations;
ANALYZE fx_providers;
ANALYZE fx_rates_cache;
