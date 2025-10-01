-- 006_fx_orchestration.sql
-- FX Orchestration, Market Maker Integration, and PvP Settlement Schema

-- Market Makers Registry
CREATE TABLE IF NOT EXISTS fx_market_makers (
    market_maker_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    market_maker_name VARCHAR(200) NOT NULL,
    legal_entity_identifier VARCHAR(20), -- LEI

    -- API Configuration
    quote_endpoint VARCHAR(500),
    execution_endpoint VARCHAR(500),
    status_endpoint VARCHAR(500),
    api_key_hash VARCHAR(128), -- Hashed API key
    public_key BYTEA, -- For signature verification

    -- Limits
    min_transaction_amount DECIMAL(20,2),
    max_transaction_amount DECIMAL(20,2),
    daily_limit DECIMAL(20,2),
    current_daily_volume DECIMAL(20,2) DEFAULT 0,

    -- Compliance
    regulator VARCHAR(50),
    license_number VARCHAR(100),

    -- Status
    is_active BOOLEAN DEFAULT TRUE,
    health_status VARCHAR(20) DEFAULT 'healthy', -- healthy, degraded, offline

    -- Metadata
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMP,

    metadata JSONB
);

CREATE INDEX idx_fx_market_makers_active ON fx_market_makers(is_active);
CREATE INDEX idx_fx_market_makers_health ON fx_market_makers(health_status);

-- Market Maker Currency Pair Configuration
CREATE TABLE IF NOT EXISTS fx_mm_currency_pairs (
    config_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    market_maker_id UUID NOT NULL REFERENCES fx_market_makers(market_maker_id) ON DELETE CASCADE,

    -- Currency pair
    from_currency CHAR(3) NOT NULL,
    to_currency CHAR(3) NOT NULL,

    -- Pricing
    min_spread_bps INTEGER, -- Minimum spread in basis points
    typical_spread_bps INTEGER,

    -- Liquidity
    min_amount DECIMAL(20,2),
    max_amount DECIMAL(20,2),
    available_liquidity DECIMAL(20,2),

    -- Settlement
    supported_quote_types TEXT[], -- spot, tom, forward, swap
    supported_settlement_methods TEXT[], -- pvp, dvp, netting, gross

    -- Status
    is_active BOOLEAN DEFAULT TRUE,

    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),

    UNIQUE(market_maker_id, from_currency, to_currency)
);

CREATE INDEX idx_fx_mm_pairs_mm ON fx_mm_currency_pairs(market_maker_id);
CREATE INDEX idx_fx_mm_pairs_currencies ON fx_mm_currency_pairs(from_currency, to_currency);
CREATE INDEX idx_fx_mm_pairs_active ON fx_mm_currency_pairs(is_active);

-- FX Quote Requests
CREATE TABLE IF NOT EXISTS fx_quote_requests (
    request_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_id UUID, -- Associated payment

    -- Quote details
    from_currency CHAR(3) NOT NULL,
    to_currency CHAR(3) NOT NULL,
    amount DECIMAL(20,2) NOT NULL,

    -- Quote type
    quote_type VARCHAR(20) NOT NULL, -- spot, tom, forward, swap
    value_date TIMESTAMP,

    -- Requester
    requester_id VARCHAR(100) NOT NULL,
    requester_name VARCHAR(200),

    -- Constraints
    max_slippage_bps INTEGER,
    ttl_seconds INTEGER DEFAULT 10,
    allow_split_execution BOOLEAN DEFAULT FALSE,

    -- Preferences
    preferred_market_makers UUID[],

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, quoted, accepted, expired, cancelled
    best_quote_id UUID, -- Best quote selected

    -- Timing
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    responded_at TIMESTAMP,

    metadata JSONB
);

CREATE INDEX idx_fx_quote_requests_payment ON fx_quote_requests(payment_id);
CREATE INDEX idx_fx_quote_requests_status ON fx_quote_requests(status);
CREATE INDEX idx_fx_quote_requests_created ON fx_quote_requests(created_at DESC);
CREATE INDEX idx_fx_quote_requests_currencies ON fx_quote_requests(from_currency, to_currency);

-- FX Quotes from Market Makers
CREATE TABLE IF NOT EXISTS fx_quotes (
    quote_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    request_id UUID NOT NULL REFERENCES fx_quote_requests(request_id) ON DELETE CASCADE,
    market_maker_id UUID NOT NULL REFERENCES fx_market_makers(market_maker_id),

    -- Quote details
    from_currency CHAR(3) NOT NULL,
    to_currency CHAR(3) NOT NULL,
    from_amount DECIMAL(20,2) NOT NULL,
    to_amount DECIMAL(20,2) NOT NULL,

    -- Rates
    rate DECIMAL(18,8) NOT NULL, -- Exchange rate
    inverse_rate DECIMAL(18,8),
    mid_rate DECIMAL(18,8),
    spread_bps INTEGER,
    markup_bps INTEGER,

    -- Liquidity
    available_liquidity DECIMAL(20,2),
    liquidity_tier VARCHAR(20), -- TIER_1, TIER_2, TIER_3, TIER_4

    -- Quote metadata
    quote_type VARCHAR(20) NOT NULL,
    value_date TIMESTAMP,
    quoted_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    ttl_seconds INTEGER,

    -- Risk
    risk_score DECIMAL(5,4), -- 0.0000 to 1.0000
    risk_reason TEXT,

    -- Selection
    is_best_quote BOOLEAN DEFAULT FALSE,
    selection_reason TEXT,

    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fx_quotes_request ON fx_quotes(request_id);
CREATE INDEX idx_fx_quotes_mm ON fx_quotes(market_maker_id);
CREATE INDEX idx_fx_quotes_best ON fx_quotes(is_best_quote);
CREATE INDEX idx_fx_quotes_created ON fx_quotes(created_at DESC);

-- FX Deals (Accepted Quotes)
CREATE TABLE IF NOT EXISTS fx_deals (
    deal_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_id UUID NOT NULL,
    request_id UUID NOT NULL REFERENCES fx_quote_requests(request_id),

    -- Trade details
    from_currency CHAR(3) NOT NULL,
    to_currency CHAR(3) NOT NULL,
    from_amount DECIMAL(20,2) NOT NULL,
    to_amount DECIMAL(20,2) NOT NULL,
    rate DECIMAL(18,8) NOT NULL,

    -- Parties
    buyer_id VARCHAR(100) NOT NULL,
    seller_id VARCHAR(100) NOT NULL,

    -- Settlement
    deal_type VARCHAR(20) NOT NULL, -- spot, tom, forward, swap
    value_date TIMESTAMP NOT NULL,
    settlement_method VARCHAR(20) NOT NULL, -- pvp, dvp, netting, gross

    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'pending',

    -- Timing
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    settled_at TIMESTAMP,

    -- Risk
    risk_score DECIMAL(5,4),
    risk_flags TEXT[],

    metadata JSONB
);

CREATE INDEX idx_fx_deals_payment ON fx_deals(payment_id);
CREATE INDEX idx_fx_deals_status ON fx_deals(status);
CREATE INDEX idx_fx_deals_created ON fx_deals(created_at DESC);
CREATE INDEX idx_fx_deals_currencies ON fx_deals(from_currency, to_currency);
CREATE INDEX idx_fx_deals_value_date ON fx_deals(value_date);

-- Market Maker Executions (for split orders)
CREATE TABLE IF NOT EXISTS fx_mm_executions (
    execution_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    deal_id UUID NOT NULL REFERENCES fx_deals(deal_id) ON DELETE CASCADE,
    market_maker_id UUID NOT NULL REFERENCES fx_market_makers(market_maker_id),
    quote_id UUID REFERENCES fx_quotes(quote_id),
    leg_id UUID, -- For split execution

    -- Execution details
    from_amount DECIMAL(20,2) NOT NULL,
    to_amount DECIMAL(20,2) NOT NULL,
    rate DECIMAL(18,8) NOT NULL,

    -- Allocation
    allocation_percentage DECIMAL(5,2), -- 0.00 to 100.00
    sequence INTEGER, -- Execution order

    -- Fees
    fee DECIMAL(20,2),
    fee_currency CHAR(3),

    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    executed_at TIMESTAMP,

    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fx_mm_executions_deal ON fx_mm_executions(deal_id);
CREATE INDEX idx_fx_mm_executions_mm ON fx_mm_executions(market_maker_id);
CREATE INDEX idx_fx_mm_executions_status ON fx_mm_executions(status);

-- PvP (Payment vs Payment) Settlements
CREATE TABLE IF NOT EXISTS fx_pvp_settlements (
    settlement_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    deal_id UUID NOT NULL REFERENCES fx_deals(deal_id),
    payment_id UUID NOT NULL,

    -- PvP mode
    pvp_mode VARCHAR(20) NOT NULL, -- simultaneous, sequential, escrow, cls
    pvp_status VARCHAR(50) NOT NULL DEFAULT 'initiated',

    -- Timing
    started_at TIMESTAMP NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP,
    duration_ms INTEGER,

    -- Atomicity
    atomic_completion BOOLEAN DEFAULT FALSE,
    failure_reason TEXT,

    -- Constraints
    timeout_seconds INTEGER DEFAULT 60,
    allow_partial_settlement BOOLEAN DEFAULT FALSE,

    -- Proof
    settlement_proof BYTEA,
    signature BYTEA,

    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fx_pvp_deal ON fx_pvp_settlements(deal_id);
CREATE INDEX idx_fx_pvp_payment ON fx_pvp_settlements(payment_id);
CREATE INDEX idx_fx_pvp_status ON fx_pvp_settlements(pvp_status);
CREATE INDEX idx_fx_pvp_started ON fx_pvp_settlements(started_at DESC);

-- PvP Legs (two legs per settlement)
CREATE TABLE IF NOT EXISTS fx_pvp_legs (
    leg_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    settlement_id UUID NOT NULL REFERENCES fx_pvp_settlements(settlement_id) ON DELETE CASCADE,

    -- Leg identifier
    leg_side VARCHAR(10) NOT NULL, -- 'A' or 'B'

    -- Payment details
    currency CHAR(3) NOT NULL,
    amount DECIMAL(20,2) NOT NULL,

    -- Accounts
    from_account VARCHAR(100) NOT NULL,
    to_account VARCHAR(100) NOT NULL,

    -- Settlement
    settlement_reference VARCHAR(100),
    value_date TIMESTAMP NOT NULL,

    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- pending, locked, executing, completed, failed, rolled_back

    -- Timing
    locked_at TIMESTAMP,
    executed_at TIMESTAMP,
    completed_at TIMESTAMP,

    -- Ledger transaction IDs
    lock_transaction_id UUID,
    settlement_transaction_id UUID,

    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),

    UNIQUE(settlement_id, leg_side)
);

CREATE INDEX idx_fx_pvp_legs_settlement ON fx_pvp_legs(settlement_id);
CREATE INDEX idx_fx_pvp_legs_status ON fx_pvp_legs(status);
CREATE INDEX idx_fx_pvp_legs_accounts ON fx_pvp_legs(from_account, to_account);

-- Market Depth Snapshots (for liquidity aggregation)
CREATE TABLE IF NOT EXISTS fx_market_depth (
    depth_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    currency_pair VARCHAR(7) NOT NULL, -- e.g., EUR/USD

    -- Aggregated rates
    mid_rate DECIMAL(18,8) NOT NULL,
    spread_bps INTEGER,

    -- Bid/Ask summary
    best_bid_rate DECIMAL(18,8),
    best_ask_rate DECIMAL(18,8),
    bid_liquidity DECIMAL(20,2),
    ask_liquidity DECIMAL(20,2),

    -- Market makers count
    mm_count INTEGER,

    -- Timestamp
    timestamp TIMESTAMP NOT NULL DEFAULT NOW(),

    -- TTL (for cleanup)
    expires_at TIMESTAMP NOT NULL,

    metadata JSONB
);

CREATE INDEX idx_fx_market_depth_pair ON fx_market_depth(currency_pair);
CREATE INDEX idx_fx_market_depth_timestamp ON fx_market_depth(timestamp DESC);
CREATE INDEX idx_fx_market_depth_expires ON fx_market_depth(expires_at);

-- Market Depth Levels (order book)
CREATE TABLE IF NOT EXISTS fx_depth_levels (
    level_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    depth_id UUID NOT NULL REFERENCES fx_market_depth(depth_id) ON DELETE CASCADE,

    -- Side
    side VARCHAR(3) NOT NULL, -- bid or ask

    -- Level details
    rate DECIMAL(18,8) NOT NULL,
    amount DECIMAL(20,2) NOT NULL,
    market_maker_count INTEGER,

    -- Contributing market makers
    market_maker_ids UUID[],

    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fx_depth_levels_depth ON fx_depth_levels(depth_id);
CREATE INDEX idx_fx_depth_levels_side ON fx_depth_levels(side);

-- FX Deal Events (audit trail)
CREATE TABLE IF NOT EXISTS fx_deal_events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    deal_id UUID NOT NULL REFERENCES fx_deals(deal_id) ON DELETE CASCADE,

    -- Event type
    event_type VARCHAR(50) NOT NULL,

    -- Event details
    previous_status VARCHAR(50),
    new_status VARCHAR(50),

    -- Metadata
    actor VARCHAR(100), -- Who/what triggered the event
    metadata JSONB,

    timestamp TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_fx_deal_events_deal ON fx_deal_events(deal_id);
CREATE INDEX idx_fx_deal_events_type ON fx_deal_events(event_type);
CREATE INDEX idx_fx_deal_events_timestamp ON fx_deal_events(timestamp DESC);

-- Market Maker Liquidity Updates (real-time)
CREATE TABLE IF NOT EXISTS fx_mm_liquidity_updates (
    update_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    market_maker_id UUID NOT NULL REFERENCES fx_market_makers(market_maker_id) ON DELETE CASCADE,

    -- Currency pair
    currency_pair VARCHAR(7) NOT NULL,

    -- Bid side
    bid_liquidity DECIMAL(20,2),
    bid_rate DECIMAL(18,8),

    -- Ask side
    ask_liquidity DECIMAL(20,2),
    ask_rate DECIMAL(18,8),

    -- TTL
    timestamp TIMESTAMP NOT NULL DEFAULT NOW(),
    ttl_seconds INTEGER DEFAULT 60,
    expires_at TIMESTAMP NOT NULL
);

CREATE INDEX idx_fx_mm_liquidity_mm ON fx_mm_liquidity_updates(market_maker_id);
CREATE INDEX idx_fx_mm_liquidity_pair ON fx_mm_liquidity_updates(currency_pair);
CREATE INDEX idx_fx_mm_liquidity_timestamp ON fx_mm_liquidity_updates(timestamp DESC);
CREATE INDEX idx_fx_mm_liquidity_expires ON fx_mm_liquidity_updates(expires_at);

-- Materialized View: FX Statistics by Currency Pair
CREATE MATERIALIZED VIEW fx_stats_by_pair AS
SELECT
    from_currency || '/' || to_currency AS currency_pair,
    COUNT(DISTINCT deal_id) AS total_deals,
    SUM(from_amount) AS total_from_amount,
    SUM(to_amount) AS total_to_amount,
    AVG(rate) AS avg_rate,
    MIN(rate) AS min_rate,
    MAX(rate) AS max_rate,
    STDDEV(rate) AS rate_volatility,
    COUNT(DISTINCT CASE WHEN status = 'settled' THEN deal_id END) AS settled_deals,
    COUNT(DISTINCT CASE WHEN status = 'failed' THEN deal_id END) AS failed_deals,
    AVG(EXTRACT(EPOCH FROM (settled_at - created_at)) * 1000)::INTEGER AS avg_settlement_time_ms,
    DATE(created_at) AS trade_date
FROM fx_deals
WHERE created_at >= NOW() - INTERVAL '30 days'
GROUP BY from_currency, to_currency, DATE(created_at)
ORDER BY trade_date DESC, total_deals DESC;

CREATE UNIQUE INDEX idx_fx_stats_pair_date ON fx_stats_by_pair(currency_pair, trade_date);

-- Materialized View: Market Maker Performance
CREATE MATERIALIZED VIEW fx_mm_performance AS
SELECT
    mm.market_maker_id,
    mm.market_maker_name,
    COUNT(DISTINCT e.execution_id) AS deals_executed,
    COUNT(DISTINCT d.deal_id) AS deals_participated,
    SUM(e.from_amount) AS total_volume_from,
    SUM(e.to_amount) AS total_volume_to,
    AVG(q.spread_bps) AS avg_spread_bps,
    AVG(EXTRACT(EPOCH FROM (q.quoted_at - r.created_at)) * 1000)::INTEGER AS avg_response_time_ms,
    COUNT(DISTINCT q.quote_id)::FLOAT / NULLIF(COUNT(DISTINCT r.request_id), 0) AS quote_rate,
    COUNT(DISTINCT CASE WHEN e.status = 'completed' THEN e.execution_id END)::FLOAT / NULLIF(COUNT(DISTINCT e.execution_id), 0) AS fill_rate,
    DATE(d.created_at) AS trade_date
FROM fx_market_makers mm
LEFT JOIN fx_mm_executions e ON mm.market_maker_id = e.market_maker_id
LEFT JOIN fx_deals d ON e.deal_id = d.deal_id
LEFT JOIN fx_quotes q ON e.quote_id = q.quote_id
LEFT JOIN fx_quote_requests r ON q.request_id = r.request_id
WHERE d.created_at >= NOW() - INTERVAL '30 days' OR d.created_at IS NULL
GROUP BY mm.market_maker_id, mm.market_maker_name, DATE(d.created_at)
ORDER BY trade_date DESC, deals_executed DESC;

CREATE UNIQUE INDEX idx_fx_mm_performance_mm_date ON fx_mm_performance(market_maker_id, trade_date);

-- Materialized View: PvP Settlement Statistics
CREATE MATERIALIZED VIEW fx_pvp_stats AS
SELECT
    pvp_mode,
    pvp_status,
    COUNT(DISTINCT settlement_id) AS settlement_count,
    AVG(duration_ms) AS avg_duration_ms,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY duration_ms) AS p50_duration_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) AS p95_duration_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) AS p99_duration_ms,
    COUNT(DISTINCT CASE WHEN atomic_completion = TRUE THEN settlement_id END) AS atomic_completions,
    COUNT(DISTINCT CASE WHEN pvp_status = 'failed' THEN settlement_id END) AS failed_settlements,
    COUNT(DISTINCT CASE WHEN pvp_status = 'rolled_back' THEN settlement_id END) AS rolled_back_settlements,
    DATE(started_at) AS settlement_date
FROM fx_pvp_settlements
WHERE started_at >= NOW() - INTERVAL '30 days'
GROUP BY pvp_mode, pvp_status, DATE(started_at)
ORDER BY settlement_date DESC, settlement_count DESC;

CREATE UNIQUE INDEX idx_fx_pvp_stats_mode_status_date ON fx_pvp_stats(pvp_mode, pvp_status, settlement_date);

-- Function: Refresh FX statistics materialized views
CREATE OR REPLACE FUNCTION refresh_fx_stats()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY fx_stats_by_pair;
    REFRESH MATERIALIZED VIEW CONCURRENTLY fx_mm_performance;
    REFRESH MATERIALIZED VIEW CONCURRENTLY fx_pvp_stats;
END;
$$ LANGUAGE plpgsql;

-- Function: Cleanup expired market depth and liquidity data
CREATE OR REPLACE FUNCTION cleanup_expired_fx_data()
RETURNS INTEGER AS $$
DECLARE
    deleted_count INTEGER := 0;
BEGIN
    -- Delete expired market depth
    DELETE FROM fx_market_depth WHERE expires_at < NOW();
    GET DIAGNOSTICS deleted_count = ROW_COUNT;

    -- Delete expired liquidity updates
    DELETE FROM fx_mm_liquidity_updates WHERE expires_at < NOW();

    -- Delete old quote requests (> 7 days)
    DELETE FROM fx_quote_requests WHERE created_at < NOW() - INTERVAL '7 days';

    RETURN deleted_count;
END;
$$ LANGUAGE plpgsql;

-- Comments
COMMENT ON TABLE fx_market_makers IS 'Registered market makers providing FX liquidity';
COMMENT ON TABLE fx_mm_currency_pairs IS 'Currency pairs supported by each market maker';
COMMENT ON TABLE fx_quote_requests IS 'FX quote requests from clients';
COMMENT ON TABLE fx_quotes IS 'Quotes received from market makers';
COMMENT ON TABLE fx_deals IS 'Executed FX deals';
COMMENT ON TABLE fx_mm_executions IS 'Market maker participation in split deals';
COMMENT ON TABLE fx_pvp_settlements IS 'PvP (Payment vs Payment) atomic settlements';
COMMENT ON TABLE fx_pvp_legs IS 'Individual legs of PvP settlements';
COMMENT ON TABLE fx_market_depth IS 'Aggregated market depth snapshots';
COMMENT ON TABLE fx_depth_levels IS 'Order book levels (bid/ask)';
COMMENT ON TABLE fx_deal_events IS 'FX deal event history';
COMMENT ON TABLE fx_mm_liquidity_updates IS 'Real-time liquidity updates from market makers';

-- Insert migration record
INSERT INTO schema_migrations (version) VALUES ('006_fx_orchestration')
ON CONFLICT (version) DO NOTHING;
