-- DelTran Analytics Database Schema
-- Purpose: Store transaction analytics, performance metrics, and token data

-- Main analytics table for all transactions
CREATE TABLE IF NOT EXISTS transaction_analytics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id VARCHAR(255) UNIQUE NOT NULL,
    timestamp TIMESTAMPTZ DEFAULT NOW(),

    -- Transaction details
    sender_id VARCHAR(255),
    receiver_id VARCHAR(255),
    amount DECIMAL(20, 2),
    currency VARCHAR(3),

    -- Performance metrics (in milliseconds)
    gateway_received_at TIMESTAMPTZ,
    clearing_started_at TIMESTAMPTZ,
    clearing_completed_at TIMESTAMPTZ,
    settlement_started_at TIMESTAMPTZ,
    settlement_completed_at TIMESTAMPTZ,

    -- Latency calculations (in ms)
    gateway_latency INTEGER,
    clearing_latency INTEGER,
    settlement_latency INTEGER,
    total_latency INTEGER,

    -- Status tracking
    status VARCHAR(50),
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,

    -- Test metadata
    test_run_id VARCHAR(255),
    test_scenario VARCHAR(255),
    load_level VARCHAR(50), -- 'low', 'medium', 'high'

    -- Service responses
    risk_score DECIMAL(3, 2),
    compliance_status VARCHAR(50),
    liquidity_source VARCHAR(255),

    -- Token data
    token_id VARCHAR(255),
    token_type VARCHAR(50),

    -- Raw data for debugging
    raw_request JSONB,
    raw_response JSONB,

    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_transaction_timestamp ON transaction_analytics(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_transaction_status ON transaction_analytics(status);
CREATE INDEX IF NOT EXISTS idx_test_run ON transaction_analytics(test_run_id);
CREATE INDEX IF NOT EXISTS idx_performance ON transaction_analytics(total_latency);
CREATE INDEX IF NOT EXISTS idx_sender ON transaction_analytics(sender_id);
CREATE INDEX IF NOT EXISTS idx_receiver ON transaction_analytics(receiver_id);

-- Token management table
CREATE TABLE IF NOT EXISTS tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token_id VARCHAR(255) UNIQUE NOT NULL,
    token_type VARCHAR(50) NOT NULL, -- 'obligation', 'asset', 'liquidity'

    -- Token details
    issuer VARCHAR(255) NOT NULL,
    owner VARCHAR(255) NOT NULL,
    amount DECIMAL(20, 8),
    currency VARCHAR(10),

    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ,

    -- Status
    status VARCHAR(50) DEFAULT 'active', -- 'active', 'locked', 'redeemed', 'expired'
    locked_until TIMESTAMPTZ,
    locked_by VARCHAR(255),

    -- Properties
    properties JSONB DEFAULT '{}'::jsonb,

    -- Audit trail
    transaction_history JSONB DEFAULT '[]'::jsonb
);

-- Token indexes
CREATE INDEX IF NOT EXISTS idx_token_owner ON tokens(owner);
CREATE INDEX IF NOT EXISTS idx_token_status ON tokens(status);
CREATE INDEX IF NOT EXISTS idx_token_type ON tokens(token_type);
CREATE INDEX IF NOT EXISTS idx_token_expires ON tokens(expires_at);

-- Performance metrics aggregation table
CREATE TABLE IF NOT EXISTS performance_metrics (
    id SERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    interval_minutes INTEGER DEFAULT 1,

    -- Throughput metrics
    total_transactions INTEGER,
    successful_transactions INTEGER,
    failed_transactions INTEGER,

    -- Latency metrics (ms)
    avg_latency DECIMAL(10, 2),
    p50_latency DECIMAL(10, 2),
    p95_latency DECIMAL(10, 2),
    p99_latency DECIMAL(10, 2),
    max_latency DECIMAL(10, 2),

    -- System metrics
    cpu_usage DECIMAL(5, 2),
    memory_usage DECIMAL(5, 2),
    active_connections INTEGER,

    -- Business metrics
    total_volume DECIMAL(20, 2),
    avg_transaction_size DECIMAL(20, 2),
    unique_participants INTEGER
);

-- Performance metrics indexes
CREATE INDEX IF NOT EXISTS idx_perf_timestamp ON performance_metrics(timestamp DESC);

-- Real-time dashboard view
CREATE OR REPLACE VIEW dashboard_metrics AS
SELECT
    COUNT(*) as total_transactions,
    COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed,
    COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed,
    COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending,
    AVG(total_latency)::INTEGER as avg_latency,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY total_latency)::INTEGER as median_latency,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY total_latency)::INTEGER as p95_latency,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY total_latency)::INTEGER as p99_latency,
    MAX(total_latency) as max_latency,
    SUM(amount)::DECIMAL(20,2) as total_volume,
    COUNT(DISTINCT sender_id) as unique_senders,
    COUNT(DISTINCT receiver_id) as unique_receivers
FROM transaction_analytics
WHERE timestamp > NOW() - INTERVAL '5 minutes';

-- Test run summary view
CREATE OR REPLACE VIEW test_run_summary AS
SELECT
    test_run_id,
    test_scenario,
    load_level,
    COUNT(*) as total_requests,
    COUNT(CASE WHEN status = 'completed' THEN 1 END) as successful,
    COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed,
    AVG(total_latency)::INTEGER as avg_latency,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY total_latency)::INTEGER as p95_latency,
    MIN(timestamp) as test_start,
    MAX(timestamp) as test_end,
    EXTRACT(EPOCH FROM (MAX(timestamp) - MIN(timestamp)))::INTEGER as duration_seconds
FROM transaction_analytics
WHERE test_run_id IS NOT NULL
GROUP BY test_run_id, test_scenario, load_level;

-- Function to update latency calculations
CREATE OR REPLACE FUNCTION calculate_transaction_latencies()
RETURNS TRIGGER AS $$
BEGIN
    -- Calculate latencies if timestamps are set
    IF NEW.gateway_received_at IS NOT NULL AND NEW.clearing_started_at IS NOT NULL THEN
        NEW.gateway_latency := EXTRACT(EPOCH FROM (NEW.clearing_started_at - NEW.gateway_received_at)) * 1000;
    END IF;

    IF NEW.clearing_started_at IS NOT NULL AND NEW.clearing_completed_at IS NOT NULL THEN
        NEW.clearing_latency := EXTRACT(EPOCH FROM (NEW.clearing_completed_at - NEW.clearing_started_at)) * 1000;
    END IF;

    IF NEW.settlement_started_at IS NOT NULL AND NEW.settlement_completed_at IS NOT NULL THEN
        NEW.settlement_latency := EXTRACT(EPOCH FROM (NEW.settlement_completed_at - NEW.settlement_started_at)) * 1000;
    END IF;

    IF NEW.gateway_received_at IS NOT NULL AND NEW.settlement_completed_at IS NOT NULL THEN
        NEW.total_latency := EXTRACT(EPOCH FROM (NEW.settlement_completed_at - NEW.gateway_received_at)) * 1000;
    END IF;

    NEW.updated_at := NOW();

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for automatic latency calculation
DROP TRIGGER IF EXISTS trigger_calculate_latencies ON transaction_analytics;
CREATE TRIGGER trigger_calculate_latencies
    BEFORE INSERT OR UPDATE ON transaction_analytics
    FOR EACH ROW
    EXECUTE FUNCTION calculate_transaction_latencies();

-- Function to update token updated_at timestamp
CREATE OR REPLACE FUNCTION update_token_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at := NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger for token timestamp updates
DROP TRIGGER IF EXISTS trigger_update_token_timestamp ON tokens;
CREATE TRIGGER trigger_update_token_timestamp
    BEFORE UPDATE ON tokens
    FOR EACH ROW
    EXECUTE FUNCTION update_token_timestamp();

-- Grant permissions (adjust username as needed)
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO deltran_user;
-- GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO deltran_user;

-- Insert sample data for testing
INSERT INTO transaction_analytics (
    transaction_id, sender_id, receiver_id, amount, currency,
    status, test_run_id, test_scenario, load_level
) VALUES
    ('TXN-SAMPLE-001', 'ACC001', 'ACC002', 1000.00, 'USD', 'completed', 'TEST-001', 'smoke', 'low'),
    ('TXN-SAMPLE-002', 'ACC003', 'ACC004', 5000.00, 'EUR', 'completed', 'TEST-001', 'smoke', 'low')
ON CONFLICT (transaction_id) DO NOTHING;

COMMENT ON TABLE transaction_analytics IS 'Stores all transaction data for performance analysis and testing';
COMMENT ON TABLE tokens IS 'Manages tokenized assets and obligations';
COMMENT ON TABLE performance_metrics IS 'Aggregated performance metrics over time';
COMMENT ON VIEW dashboard_metrics IS 'Real-time metrics for the last 5 minutes';
COMMENT ON VIEW test_run_summary IS 'Summary statistics for test runs';
