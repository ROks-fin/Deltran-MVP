-- Advanced Settlement Features
-- Clearing Windows, Liquidity Management, Netting
-- PostgreSQL 15+

SET search_path TO deltran;

-- ============================================
-- CLEARING WINDOWS
-- ============================================

CREATE TABLE clearing_windows (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bank_id UUID NOT NULL REFERENCES banks(id) ON DELETE CASCADE,

    -- Window configuration
    window_name VARCHAR(100) NOT NULL,
    timezone VARCHAR(50) NOT NULL, -- e.g., 'Asia/Dubai', 'Asia/Jerusalem'

    -- Time ranges (local bank time)
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,

    -- Days of week (1=Monday, 7=Sunday)
    active_days INT[] NOT NULL DEFAULT ARRAY[1,2,3,4,5], -- Mon-Fri by default

    -- Holiday calendar
    exclude_holidays BOOLEAN DEFAULT true,
    holiday_calendar_id UUID,

    -- Status
    is_active BOOLEAN DEFAULT true,
    priority INT DEFAULT 100, -- For banks with multiple windows

    -- Currency-specific windows
    currencies VARCHAR(3)[], -- NULL = applies to all currencies

    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_window_times CHECK (start_time < end_time),
    CONSTRAINT chk_active_days CHECK (array_length(active_days, 1) > 0)
);

CREATE INDEX idx_clearing_windows_bank ON clearing_windows(bank_id);
CREATE INDEX idx_clearing_windows_active ON clearing_windows(is_active) WHERE is_active = true;
CREATE INDEX idx_clearing_windows_currencies ON clearing_windows USING gin(currencies);

COMMENT ON TABLE clearing_windows IS 'Bank-specific clearing window configurations';

-- ============================================
-- HOLIDAYS CALENDAR
-- ============================================

CREATE TABLE holidays (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    calendar_id UUID NOT NULL,
    holiday_date DATE NOT NULL,
    holiday_name VARCHAR(255) NOT NULL,
    country_code CHAR(2) NOT NULL,
    is_full_day BOOLEAN DEFAULT true,

    -- Partial day holiday
    start_time TIME,
    end_time TIME,

    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT uq_holiday_date UNIQUE (calendar_id, holiday_date)
);

CREATE INDEX idx_holidays_calendar ON holidays(calendar_id);
CREATE INDEX idx_holidays_date ON holidays(holiday_date);
CREATE INDEX idx_holidays_country ON holidays(country_code);

COMMENT ON TABLE holidays IS 'Bank holiday calendars for clearing window exclusions';

-- ============================================
-- LIQUIDITY POOLS
-- ============================================

CREATE TABLE liquidity_pools (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pool_name VARCHAR(100) NOT NULL UNIQUE,
    pool_type VARCHAR(50) NOT NULL, -- 'provider', 'internal', 'reserve'

    -- Owner/Manager
    owner_entity_id UUID, -- Can be a bank or the platform
    owner_entity_type VARCHAR(50),

    -- Status
    is_active BOOLEAN DEFAULT true,

    -- Limits
    total_capacity_usd DECIMAL(20,2),
    min_reserve_ratio DECIMAL(5,4), -- e.g., 0.10 = 10%
    max_exposure_ratio DECIMAL(5,4), -- e.g., 0.80 = 80%

    -- Rebalancing
    auto_rebalance BOOLEAN DEFAULT true,
    rebalance_threshold_pct DECIMAL(5,2) DEFAULT 20.00, -- Trigger at 20% deviation

    -- Risk
    risk_rating VARCHAR(20),

    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_liquidity_pools_active ON liquidity_pools(is_active) WHERE is_active = true;
CREATE INDEX idx_liquidity_pools_type ON liquidity_pools(pool_type);

COMMENT ON TABLE liquidity_pools IS 'Liquidity provider pools for currency rebalancing';

-- ============================================
-- LIQUIDITY POOL BALANCES
-- ============================================

CREATE TABLE liquidity_pool_balances (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pool_id UUID NOT NULL REFERENCES liquidity_pools(id) ON DELETE CASCADE,
    currency VARCHAR(3) NOT NULL,

    -- Balances
    available_balance DECIMAL(20,2) DEFAULT 0,
    reserved_balance DECIMAL(20,2) DEFAULT 0,
    total_balance DECIMAL(20,2) GENERATED ALWAYS AS (available_balance + reserved_balance) STORED,

    -- Thresholds
    min_threshold DECIMAL(20,2) NOT NULL,
    max_threshold DECIMAL(20,2) NOT NULL,
    target_balance DECIMAL(20,2) NOT NULL,

    -- Tracking
    last_rebalance_at TIMESTAMPTZ,
    rebalance_count INT DEFAULT 0,

    -- Status
    is_active BOOLEAN DEFAULT true,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT chk_lpb_balances CHECK (available_balance >= 0 AND reserved_balance >= 0),
    CONSTRAINT chk_lpb_thresholds CHECK (min_threshold < target_balance AND target_balance < max_threshold),
    CONSTRAINT uq_pool_currency UNIQUE (pool_id, currency)
);

CREATE INDEX idx_lpb_pool ON liquidity_pool_balances(pool_id);
CREATE INDEX idx_lpb_currency ON liquidity_pool_balances(currency);
CREATE INDEX idx_lpb_active ON liquidity_pool_balances(is_active) WHERE is_active = true;

COMMENT ON TABLE liquidity_pool_balances IS 'Currency-specific balances in liquidity pools';

-- ============================================
-- LIQUIDITY TRANSACTIONS
-- ============================================

CREATE TABLE liquidity_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    pool_id UUID NOT NULL REFERENCES liquidity_pools(id),

    -- Transaction details
    transaction_type VARCHAR(50) NOT NULL, -- 'deposit', 'withdrawal', 'rebalance', 'fee'
    currency VARCHAR(3) NOT NULL,
    amount DECIMAL(20,2) NOT NULL,

    -- Related entities
    related_payment_id UUID REFERENCES payments(id),
    related_bank_id UUID REFERENCES banks(id),

    -- Before/After
    balance_before DECIMAL(20,2) NOT NULL,
    balance_after DECIMAL(20,2) NOT NULL,

    -- Rebalancing
    is_rebalance BOOLEAN DEFAULT false,
    rebalance_reason TEXT,
    target_balance DECIMAL(20,2),

    -- Tracking
    initiated_by UUID REFERENCES users(id),
    timestamp TIMESTAMPTZ DEFAULT NOW(),

    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_lt_amount CHECK (amount != 0),
    CONSTRAINT chk_lt_balance_after CHECK (balance_after >= 0)
);

CREATE INDEX idx_lt_pool ON liquidity_transactions(pool_id);
CREATE INDEX idx_lt_currency ON liquidity_transactions(currency);
CREATE INDEX idx_lt_timestamp ON liquidity_transactions(timestamp DESC);
CREATE INDEX idx_lt_type ON liquidity_transactions(transaction_type);
CREATE INDEX idx_lt_rebalance ON liquidity_transactions(is_rebalance) WHERE is_rebalance = true;

COMMENT ON TABLE liquidity_transactions IS 'Audit trail of all liquidity pool movements';

-- ============================================
-- NETTING CYCLES
-- ============================================

CREATE TABLE netting_cycles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    cycle_reference VARCHAR(100) UNIQUE NOT NULL,

    -- Cycle configuration
    cycle_type VARCHAR(50) NOT NULL, -- 'bilateral', 'multilateral', 'continuous'
    currency VARCHAR(3) NOT NULL,

    -- Participants
    participant_bank_ids UUID[] NOT NULL,
    participant_count INT GENERATED ALWAYS AS (array_length(participant_bank_ids, 1)) STORED,

    -- Time window
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,

    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'open', -- 'open', 'closed', 'calculating', 'settled', 'failed'

    -- Financial summary
    total_payments_included INT DEFAULT 0,
    gross_payment_value DECIMAL(20,2) DEFAULT 0,
    net_settlement_value DECIMAL(20,2) DEFAULT 0,
    netting_efficiency_pct DECIMAL(5,2) GENERATED ALWAYS AS (
        CASE WHEN gross_payment_value > 0
        THEN ((gross_payment_value - net_settlement_value) / gross_payment_value * 100)
        ELSE 0 END
    ) STORED,

    -- Processing
    calculation_started_at TIMESTAMPTZ,
    calculation_completed_at TIMESTAMPTZ,
    settlement_initiated_at TIMESTAMPTZ,
    settlement_completed_at TIMESTAMPTZ,

    -- Error handling
    error_message TEXT,
    retry_count INT DEFAULT 0,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_nc_window CHECK (window_start < window_end),
    CONSTRAINT chk_nc_participants CHECK (array_length(participant_bank_ids, 1) >= 2)
);

CREATE INDEX idx_netting_cycles_reference ON netting_cycles(cycle_reference);
CREATE INDEX idx_netting_cycles_status ON netting_cycles(status);
CREATE INDEX idx_netting_cycles_currency ON netting_cycles(currency);
CREATE INDEX idx_netting_cycles_window_start ON netting_cycles(window_start DESC);
CREATE INDEX idx_netting_cycles_participants ON netting_cycles USING gin(participant_bank_ids);

COMMENT ON TABLE netting_cycles IS 'Netting cycles for multilateral settlement';

-- ============================================
-- NETTING POSITIONS
-- ============================================

CREATE TABLE netting_positions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    cycle_id UUID NOT NULL REFERENCES netting_cycles(id) ON DELETE CASCADE,
    bank_id UUID NOT NULL REFERENCES banks(id),

    -- Position details
    currency VARCHAR(3) NOT NULL,

    -- Gross flows
    total_sent_amount DECIMAL(20,2) DEFAULT 0,
    total_sent_count INT DEFAULT 0,
    total_received_amount DECIMAL(20,2) DEFAULT 0,
    total_received_count INT DEFAULT 0,

    -- Net position
    net_position DECIMAL(20,2) GENERATED ALWAYS AS (total_received_amount - total_sent_amount) STORED,
    position_type VARCHAR(20) GENERATED ALWAYS AS (
        CASE
            WHEN (total_received_amount - total_sent_amount) > 0 THEN 'creditor'
            WHEN (total_received_amount - total_sent_amount) < 0 THEN 'debtor'
            ELSE 'neutral'
        END
    ) STORED,

    -- Settlement
    settlement_amount DECIMAL(20,2),
    settlement_status VARCHAR(50) DEFAULT 'pending', -- 'pending', 'settled', 'failed'
    settled_at TIMESTAMPTZ,

    -- Counterparty breakdown
    counterparties JSONB DEFAULT '[]'::jsonb,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT chk_np_counts CHECK (total_sent_count >= 0 AND total_received_count >= 0),
    CONSTRAINT chk_np_amounts CHECK (total_sent_amount >= 0 AND total_received_amount >= 0),
    CONSTRAINT uq_netting_position UNIQUE (cycle_id, bank_id, currency)
);

CREATE INDEX idx_netting_positions_cycle ON netting_positions(cycle_id);
CREATE INDEX idx_netting_positions_bank ON netting_positions(bank_id);
CREATE INDEX idx_netting_positions_currency ON netting_positions(currency);
CREATE INDEX idx_netting_positions_type ON netting_positions(position_type);

COMMENT ON TABLE netting_positions IS 'Individual bank positions within a netting cycle';

-- ============================================
-- FX RATES
-- ============================================

CREATE TABLE fx_rates (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- Currency pair
    from_currency VARCHAR(3) NOT NULL,
    to_currency VARCHAR(3) NOT NULL,

    -- Rate details
    rate DECIMAL(18,8) NOT NULL,
    inverse_rate DECIMAL(18,8) GENERATED ALWAYS AS (1.0 / rate) STORED,

    -- Source
    rate_source VARCHAR(100) NOT NULL, -- 'internal', 'bloomberg', 'reuters', 'manual'
    provider_reference VARCHAR(255),

    -- Validity
    valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_until TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,

    -- Volatility tracking
    volatility_pct DECIMAL(5,2),
    daily_change_pct DECIMAL(6,3),

    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_fx_rate CHECK (rate > 0),
    CONSTRAINT chk_fx_different_currencies CHECK (from_currency != to_currency),
    CONSTRAINT chk_fx_valid_period CHECK (valid_until IS NULL OR valid_until > valid_from)
);

CREATE INDEX idx_fx_rates_pair ON fx_rates(from_currency, to_currency);
CREATE INDEX idx_fx_rates_active ON fx_rates(is_active, valid_from DESC) WHERE is_active = true;
CREATE INDEX idx_fx_rates_valid_from ON fx_rates(valid_from DESC);

COMMENT ON TABLE fx_rates IS 'Foreign exchange rates with time validity';

-- ============================================
-- FX RATE HISTORY (for volatility analysis)
-- ============================================

CREATE TABLE fx_rate_history (
    id BIGSERIAL PRIMARY KEY,
    from_currency VARCHAR(3) NOT NULL,
    to_currency VARCHAR(3) NOT NULL,
    rate DECIMAL(18,8) NOT NULL,
    timestamp TIMESTAMPTZ DEFAULT NOW(),

    -- Metadata
    source VARCHAR(100),
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_fxh_rate CHECK (rate > 0)
);

CREATE INDEX idx_fxh_pair ON fx_rate_history(from_currency, to_currency);
CREATE INDEX idx_fxh_timestamp ON fx_rate_history(timestamp DESC);

COMMENT ON TABLE fx_rate_history IS 'Historical FX rates for analytics and volatility tracking';

-- ============================================
-- VIEWS
-- ============================================

-- Active clearing windows by bank
CREATE VIEW v_active_clearing_windows AS
SELECT
    b.id as bank_id,
    b.bic_code,
    b.name as bank_name,
    b.country_code,
    cw.window_name,
    cw.timezone,
    cw.start_time,
    cw.end_time,
    cw.active_days,
    cw.currencies,
    cw.is_active
FROM banks b
JOIN clearing_windows cw ON b.id = cw.bank_id
WHERE cw.is_active = true AND b.is_active = true;

-- Liquidity pool status
CREATE VIEW v_liquidity_pool_status AS
SELECT
    lp.id as pool_id,
    lp.pool_name,
    lp.pool_type,
    lpb.currency,
    lpb.available_balance,
    lpb.reserved_balance,
    lpb.total_balance,
    lpb.min_threshold,
    lpb.target_balance,
    lpb.max_threshold,
    CASE
        WHEN lpb.total_balance < lpb.min_threshold THEN 'LOW'
        WHEN lpb.total_balance > lpb.max_threshold THEN 'HIGH'
        ELSE 'NORMAL'
    END as status,
    ((lpb.total_balance - lpb.target_balance) / lpb.target_balance * 100) as deviation_pct,
    lpb.rebalance_count,
    lpb.last_rebalance_at
FROM liquidity_pools lp
JOIN liquidity_pool_balances lpb ON lp.id = lpb.pool_id
WHERE lp.is_active = true AND lpb.is_active = true;

-- Current netting cycles
CREATE VIEW v_active_netting_cycles AS
SELECT
    nc.id,
    nc.cycle_reference,
    nc.cycle_type,
    nc.currency,
    nc.participant_count,
    nc.status,
    nc.window_start,
    nc.window_end,
    nc.total_payments_included,
    nc.gross_payment_value,
    nc.net_settlement_value,
    nc.netting_efficiency_pct,
    nc.created_at
FROM netting_cycles nc
WHERE nc.status IN ('open', 'calculating', 'closed')
ORDER BY nc.window_start DESC;

-- Latest FX rates
CREATE VIEW v_latest_fx_rates AS
WITH ranked_rates AS (
    SELECT
        from_currency,
        to_currency,
        rate,
        inverse_rate,
        rate_source,
        valid_from,
        ROW_NUMBER() OVER (
            PARTITION BY from_currency, to_currency
            ORDER BY valid_from DESC
        ) as rn
    FROM fx_rates
    WHERE is_active = true
    AND (valid_until IS NULL OR valid_until > NOW())
)
SELECT
    from_currency,
    to_currency,
    rate,
    inverse_rate,
    rate_source,
    valid_from
FROM ranked_rates
WHERE rn = 1;

-- ============================================
-- FUNCTIONS
-- ============================================

-- Check if bank is in clearing window
CREATE OR REPLACE FUNCTION is_bank_in_clearing_window(
    p_bank_id UUID,
    p_currency VARCHAR(3) DEFAULT NULL,
    p_check_time TIMESTAMPTZ DEFAULT NOW()
)
RETURNS BOOLEAN AS $$
DECLARE
    v_result BOOLEAN;
BEGIN
    SELECT EXISTS(
        SELECT 1
        FROM clearing_windows cw
        WHERE cw.bank_id = p_bank_id
        AND cw.is_active = true
        AND (p_currency IS NULL OR cw.currencies IS NULL OR p_currency = ANY(cw.currencies))
        AND EXTRACT(DOW FROM p_check_time AT TIME ZONE cw.timezone) + 1 = ANY(cw.active_days)
        AND (p_check_time AT TIME ZONE cw.timezone)::TIME BETWEEN cw.start_time AND cw.end_time
    ) INTO v_result;

    RETURN v_result;
END;
$$ LANGUAGE plpgsql STABLE;

COMMENT ON FUNCTION is_bank_in_clearing_window IS 'Check if a bank is currently in its clearing window';

-- Get current FX rate
CREATE OR REPLACE FUNCTION get_fx_rate(
    p_from_currency VARCHAR(3),
    p_to_currency VARCHAR(3),
    p_at_time TIMESTAMPTZ DEFAULT NOW()
)
RETURNS DECIMAL(18,8) AS $$
DECLARE
    v_rate DECIMAL(18,8);
BEGIN
    IF p_from_currency = p_to_currency THEN
        RETURN 1.0;
    END IF;

    SELECT rate INTO v_rate
    FROM fx_rates
    WHERE from_currency = p_from_currency
    AND to_currency = p_to_currency
    AND is_active = true
    AND valid_from <= p_at_time
    AND (valid_until IS NULL OR valid_until > p_at_time)
    ORDER BY valid_from DESC
    LIMIT 1;

    IF v_rate IS NULL THEN
        -- Try inverse rate
        SELECT (1.0 / rate) INTO v_rate
        FROM fx_rates
        WHERE from_currency = p_to_currency
        AND to_currency = p_from_currency
        AND is_active = true
        AND valid_from <= p_at_time
        AND (valid_until IS NULL OR valid_until > p_at_time)
        ORDER BY valid_from DESC
        LIMIT 1;
    END IF;

    RETURN v_rate;
END;
$$ LANGUAGE plpgsql STABLE;

COMMENT ON FUNCTION get_fx_rate IS 'Get FX rate for currency conversion';

-- ============================================
-- TRIGGERS
-- ============================================

CREATE TRIGGER trg_clearing_windows_updated_at
    BEFORE UPDATE ON clearing_windows
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trg_liquidity_pools_updated_at
    BEFORE UPDATE ON liquidity_pools
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trg_liquidity_pool_balances_updated_at
    BEFORE UPDATE ON liquidity_pool_balances
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trg_netting_cycles_updated_at
    BEFORE UPDATE ON netting_cycles
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trg_netting_positions_updated_at
    BEFORE UPDATE ON netting_positions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================
-- INITIAL DATA
-- ============================================

-- Create default liquidity pool
INSERT INTO liquidity_pools (pool_name, pool_type, is_active, auto_rebalance, rebalance_threshold_pct)
VALUES ('Primary Liquidity Pool', 'provider', true, true, 20.00);

-- Get the pool ID
DO $$
DECLARE
    v_pool_id UUID;
BEGIN
    SELECT id INTO v_pool_id FROM liquidity_pools WHERE pool_name = 'Primary Liquidity Pool';

    -- Initialize pool balances for common currencies
    INSERT INTO liquidity_pool_balances (pool_id, currency, available_balance, min_threshold, target_balance, max_threshold)
    VALUES
        (v_pool_id, 'USD', 2000000000.00, 500000000.00, 1000000000.00, 3000000000.00),
        (v_pool_id, 'EUR', 500000000.00, 100000000.00, 250000000.00, 750000000.00),
        (v_pool_id, 'GBP', 300000000.00, 75000000.00, 150000000.00, 450000000.00),
        (v_pool_id, 'AED', 1000000000.00, 250000000.00, 500000000.00, 1500000000.00),
        (v_pool_id, 'ILS', 500000000.00, 125000000.00, 250000000.00, 750000000.00),
        (v_pool_id, 'PKR', 50000000000.00, 12500000000.00, 25000000000.00, 75000000000.00),
        (v_pool_id, 'INR', 60000000000.00, 15000000000.00, 30000000000.00, 90000000000.00);
END $$;

-- Initialize FX rates (base: USD)
INSERT INTO fx_rates (from_currency, to_currency, rate, rate_source, is_active)
VALUES
    ('USD', 'EUR', 0.92, 'internal', true),
    ('USD', 'GBP', 0.79, 'internal', true),
    ('USD', 'AED', 3.6725, 'internal', true),
    ('USD', 'ILS', 3.65, 'internal', true),
    ('USD', 'PKR', 278.50, 'internal', true),
    ('USD', 'INR', 83.25, 'internal', true),
    ('USD', 'SAR', 3.75, 'internal', true),
    ('USD', 'JPY', 149.50, 'internal', true),
    ('USD', 'CHF', 0.88, 'internal', true);

-- Add clearing windows for existing banks
DO $$
DECLARE
    v_bank_id UUID;
BEGIN
    -- UAE Bank
    SELECT id INTO v_bank_id FROM banks WHERE bic_code = 'ENBXAEADXXX';
    IF v_bank_id IS NOT NULL THEN
        INSERT INTO clearing_windows (bank_id, window_name, timezone, start_time, end_time, active_days, currencies)
        VALUES (v_bank_id, 'UAE Standard Window', 'Asia/Dubai', '08:00:00', '16:00:00', ARRAY[1,2,3,4,5], ARRAY['AED', 'USD', 'EUR']);
    END IF;

    -- Israel Bank
    SELECT id INTO v_bank_id FROM banks WHERE bic_code = 'LUMIILITXXX';
    IF v_bank_id IS NOT NULL THEN
        INSERT INTO clearing_windows (bank_id, window_name, timezone, start_time, end_time, active_days, currencies)
        VALUES (v_bank_id, 'Israel Standard Window', 'Asia/Jerusalem', '09:00:00', '17:00:00', ARRAY[1,2,3,4,5], ARRAY['ILS', 'USD', 'EUR']);
    END IF;

    -- Pakistan Bank
    SELECT id INTO v_bank_id FROM banks WHERE bic_code = 'HABBPKKKXXX';
    IF v_bank_id IS NOT NULL THEN
        INSERT INTO clearing_windows (bank_id, window_name, timezone, start_time, end_time, active_days, currencies)
        VALUES (v_bank_id, 'Pakistan Standard Window', 'Asia/Karachi', '09:00:00', '17:00:00', ARRAY[1,2,3,4,5], ARRAY['PKR', 'USD']);
    END IF;

    -- India Bank
    SELECT id INTO v_bank_id FROM banks WHERE bic_code = 'SBININBBXXX';
    IF v_bank_id IS NOT NULL THEN
        INSERT INTO clearing_windows (bank_id, window_name, timezone, start_time, end_time, active_days, currencies)
        VALUES (v_bank_id, 'India Standard Window', 'Asia/Kolkata', '10:00:00', '18:00:00', ARRAY[1,2,3,4,5,6], ARRAY['INR', 'USD']);
    END IF;
END $$;

COMMENT ON SCHEMA deltran IS 'DelTran Payment Rail - Extended Schema with Clearing Windows, Liquidity Management, and Netting - v2.0';
