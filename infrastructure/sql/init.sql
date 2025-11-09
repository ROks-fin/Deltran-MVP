-- DelTran MVP Database Schema
-- Complete initialization script for all services

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";  -- For fuzzy text matching
CREATE EXTENSION IF NOT EXISTS "timescaledb";  -- For time-series data

-- ============================================================================
-- CORE TABLES
-- ============================================================================

-- Banks participating in the network
CREATE TABLE IF NOT EXISTS banks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_code VARCHAR(20) UNIQUE NOT NULL,
    bank_name VARCHAR(255) NOT NULL,
    country VARCHAR(2) NOT NULL,
    bic_code VARCHAR(11),
    status VARCHAR(20) NOT NULL DEFAULT 'Active', -- Active, Suspended, Closed
    onboarded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Accounts for each bank
CREATE TABLE IF NOT EXISTS accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_id UUID NOT NULL REFERENCES banks(id),
    account_number VARCHAR(50) NOT NULL,
    account_type VARCHAR(20) NOT NULL, -- Settlement, Liquidity, Reserve
    currency VARCHAR(3) NOT NULL,
    balance DECIMAL(20,2) NOT NULL DEFAULT 0,
    reserved_balance DECIMAL(20,2) NOT NULL DEFAULT 0,
    available_balance DECIMAL(20,2) GENERATED ALWAYS AS (balance - reserved_balance) STORED,
    status VARCHAR(20) NOT NULL DEFAULT 'Active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(bank_id, account_number, currency)
);

CREATE INDEX idx_accounts_bank ON accounts(bank_id);
CREATE INDEX idx_accounts_currency ON accounts(currency);
CREATE INDEX idx_accounts_status ON accounts(status);

-- ============================================================================
-- TRANSACTION TABLES
-- ============================================================================

-- Main transactions table
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_ref VARCHAR(50) UNIQUE NOT NULL,
    sender_bank_id UUID NOT NULL REFERENCES banks(id),
    sender_account VARCHAR(50) NOT NULL,
    sender_name VARCHAR(255) NOT NULL,
    sender_country VARCHAR(2) NOT NULL,
    receiver_bank_id UUID NOT NULL REFERENCES banks(id),
    receiver_account VARCHAR(50) NOT NULL,
    receiver_name VARCHAR(255) NOT NULL,
    receiver_country VARCHAR(2) NOT NULL,
    sent_amount DECIMAL(20,2) NOT NULL,
    sent_currency VARCHAR(3) NOT NULL,
    received_amount DECIMAL(20,2) NOT NULL,
    received_currency VARCHAR(3) NOT NULL,
    exchange_rate DECIMAL(12,6),
    purpose TEXT,
    status VARCHAR(20) NOT NULL, -- Pending, RiskCheck, ComplianceCheck, Approved, Processing, Completed, Failed, Rejected
    settlement_type VARCHAR(20), -- Instant, Netted, Deferred
    clearing_window_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_transactions_sender ON transactions(sender_bank_id);
CREATE INDEX idx_transactions_receiver ON transactions(receiver_bank_id);
CREATE INDEX idx_transactions_status ON transactions(status);
CREATE INDEX idx_transactions_created ON transactions(created_at DESC);
CREATE INDEX idx_transactions_ref ON transactions(transaction_ref);

-- ============================================================================
-- TOKEN ENGINE TABLES
-- ============================================================================

-- Token issuances
CREATE TABLE IF NOT EXISTS token_issuances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_id UUID NOT NULL REFERENCES banks(id),
    currency VARCHAR(3) NOT NULL,
    amount DECIMAL(20,2) NOT NULL,
    token_count BIGINT NOT NULL,
    collateral_type VARCHAR(50),
    collateral_reference TEXT,
    status VARCHAR(20) NOT NULL, -- Pending, Issued, Redeemed, Failed
    issued_at TIMESTAMPTZ,
    redeemed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_token_issuances_bank ON token_issuances(bank_id);
CREATE INDEX idx_token_issuances_status ON token_issuances(status);
CREATE INDEX idx_token_issuances_created ON token_issuances(created_at DESC);

-- Token balances (current state)
CREATE TABLE IF NOT EXISTS token_balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_id UUID NOT NULL REFERENCES banks(id),
    currency VARCHAR(3) NOT NULL,
    total_tokens BIGINT NOT NULL DEFAULT 0,
    available_tokens BIGINT NOT NULL DEFAULT 0,
    reserved_tokens BIGINT NOT NULL DEFAULT 0,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(bank_id, currency)
);

CREATE INDEX idx_token_balances_bank ON token_balances(bank_id);

-- ============================================================================
-- OBLIGATION ENGINE TABLES
-- ============================================================================

-- Obligations (bilateral IOUs)
CREATE TABLE IF NOT EXISTS obligations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID REFERENCES transactions(id),
    debtor_bank_id UUID NOT NULL REFERENCES banks(id),
    creditor_bank_id UUID NOT NULL REFERENCES banks(id),
    currency VARCHAR(3) NOT NULL,
    amount DECIMAL(20,2) NOT NULL,
    obligation_type VARCHAR(20) NOT NULL, -- Instant, Deferred, Netted
    status VARCHAR(20) NOT NULL, -- Created, Active, Settled, Netted, Cancelled
    clearing_window_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    settled_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_obligations_debtor ON obligations(debtor_bank_id);
CREATE INDEX idx_obligations_creditor ON obligations(creditor_bank_id);
CREATE INDEX idx_obligations_status ON obligations(status);
CREATE INDEX idx_obligations_created ON obligations(created_at DESC);
CREATE INDEX idx_obligations_window ON obligations(clearing_window_id);

-- Netting results
CREATE TABLE IF NOT EXISTS netting_results (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    clearing_window_id UUID NOT NULL,
    bank_a_id UUID NOT NULL REFERENCES banks(id),
    bank_b_id UUID NOT NULL REFERENCES banks(id),
    currency VARCHAR(3) NOT NULL,
    gross_obligations_a_to_b DECIMAL(20,2) NOT NULL,
    gross_obligations_b_to_a DECIMAL(20,2) NOT NULL,
    net_amount DECIMAL(20,2) NOT NULL,
    net_payer_id UUID NOT NULL REFERENCES banks(id),
    obligations_netted INTEGER NOT NULL,
    netting_efficiency DECIMAL(5,2), -- Percentage saved
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    settled_at TIMESTAMPTZ
);

CREATE INDEX idx_netting_window ON netting_results(clearing_window_id);
CREATE INDEX idx_netting_created ON netting_results(created_at DESC);

-- ============================================================================
-- LIQUIDITY ROUTER TABLES
-- ============================================================================

-- Liquidity positions
CREATE TABLE IF NOT EXISTS liquidity_positions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_id UUID NOT NULL REFERENCES banks(id),
    currency VARCHAR(3) NOT NULL,
    available_liquidity DECIMAL(20,2) NOT NULL,
    reserved_liquidity DECIMAL(20,2) NOT NULL DEFAULT 0,
    total_inflow_24h DECIMAL(20,2) NOT NULL DEFAULT 0,
    total_outflow_24h DECIMAL(20,2) NOT NULL DEFAULT 0,
    position_score DECIMAL(5,2), -- Health score 0-100
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(bank_id, currency)
);

CREATE INDEX idx_liquidity_bank ON liquidity_positions(bank_id);
CREATE INDEX idx_liquidity_currency ON liquidity_positions(currency);

-- Liquidity corridors (currency pairs with routing info)
CREATE TABLE IF NOT EXISTS liquidity_corridors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_currency VARCHAR(3) NOT NULL,
    to_currency VARCHAR(3) NOT NULL,
    primary_route JSONB NOT NULL,
    alternative_routes JSONB DEFAULT '[]'::jsonb,
    average_liquidity DECIMAL(20,2),
    transaction_volume_24h DECIMAL(20,2),
    status VARCHAR(20) NOT NULL DEFAULT 'Active',
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(from_currency, to_currency)
);

CREATE INDEX idx_corridors_from ON liquidity_corridors(from_currency);
CREATE INDEX idx_corridors_to ON liquidity_corridors(to_currency);

-- ============================================================================
-- RISK ENGINE TABLES
-- ============================================================================

-- Risk scores history
CREATE TABLE IF NOT EXISTS risk_scores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID NOT NULL REFERENCES transactions(id),
    overall_score DECIMAL(5,2) NOT NULL,
    decision VARCHAR(20) NOT NULL, -- Approve, ApproveWithLimit, Review, Reject
    confidence DECIMAL(3,2) NOT NULL,
    factors JSONB NOT NULL,
    explanation TEXT,
    calculated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_risk_transaction ON risk_scores(transaction_id);
CREATE INDEX idx_risk_decision ON risk_scores(decision);
CREATE INDEX idx_risk_calculated ON risk_scores(calculated_at DESC);

-- Dynamic limits
CREATE TABLE IF NOT EXISTS dynamic_limits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_id UUID NOT NULL REFERENCES banks(id),
    corridor VARCHAR(20) NOT NULL,
    base_limit DECIMAL(20,2) NOT NULL,
    current_limit DECIMAL(20,2) NOT NULL,
    adjustment_factor DECIMAL(3,2) NOT NULL,
    reason TEXT,
    valid_until TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(bank_id, corridor)
);

CREATE INDEX idx_limits_bank ON dynamic_limits(bank_id);
CREATE INDEX idx_limits_valid ON dynamic_limits(valid_until);

-- Circuit breakers state
CREATE TABLE IF NOT EXISTS circuit_breakers (
    id VARCHAR(50) PRIMARY KEY,
    state VARCHAR(20) NOT NULL, -- Closed, Open, HalfOpen
    failure_count INT NOT NULL DEFAULT 0,
    success_count INT NOT NULL DEFAULT 0,
    last_failure_time TIMESTAMPTZ,
    last_reset_time TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Risk events for audit
CREATE TABLE IF NOT EXISTS risk_events (
    id BIGSERIAL PRIMARY KEY,
    event_type VARCHAR(50) NOT NULL,
    transaction_id UUID REFERENCES transactions(id),
    bank_id UUID REFERENCES banks(id),
    details JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_risk_events_type ON risk_events(event_type);
CREATE INDEX idx_risk_events_created ON risk_events(created_at DESC);

-- ============================================================================
-- COMPLIANCE ENGINE TABLES
-- ============================================================================

-- Compliance check results
CREATE TABLE IF NOT EXISTS compliance_checks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID NOT NULL REFERENCES transactions(id),
    overall_status VARCHAR(20) NOT NULL, -- Approved, ReviewRequired, Rejected, Hold
    risk_rating VARCHAR(20) NOT NULL, -- Low, Medium, High, VeryHigh, Prohibited
    sanctions_result JSONB NOT NULL,
    aml_result JSONB NOT NULL,
    pep_result JSONB NOT NULL,
    pattern_result JSONB NOT NULL,
    required_actions TEXT[],
    checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_compliance_transaction ON compliance_checks(transaction_id);
CREATE INDEX idx_compliance_status ON compliance_checks(overall_status);
CREATE INDEX idx_compliance_checked ON compliance_checks(checked_at DESC);

-- Sanctions lists cache
CREATE TABLE IF NOT EXISTS sanctions_lists (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    list_type VARCHAR(20) NOT NULL,
    entity_name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    aliases TEXT[],
    countries TEXT[],
    identifiers JSONB,
    reasons TEXT[],
    added_date DATE,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sanctions_normalized ON sanctions_lists(normalized_name);
CREATE INDEX idx_sanctions_list ON sanctions_lists(list_type);
CREATE INDEX idx_sanctions_trgm ON sanctions_lists USING gin(normalized_name gin_trgm_ops);

-- PEP database
CREATE TABLE IF NOT EXISTS pep_list (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    full_name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    pep_type VARCHAR(50) NOT NULL,
    position TEXT,
    country VARCHAR(2),
    risk_level VARCHAR(20),
    start_date DATE,
    end_date DATE,
    is_active BOOLEAN DEFAULT true,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pep_normalized ON pep_list(normalized_name);
CREATE INDEX idx_pep_active ON pep_list(is_active);
CREATE INDEX idx_pep_trgm ON pep_list USING gin(normalized_name gin_trgm_ops);

-- SAR/CTR reports
CREATE TABLE IF NOT EXISTS regulatory_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_type VARCHAR(20) NOT NULL, -- SAR, CTR
    report_number VARCHAR(50) UNIQUE NOT NULL,
    transaction_id UUID REFERENCES transactions(id),
    filing_date TIMESTAMPTZ NOT NULL,
    due_date TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL, -- Draft, Filed, Accepted, Rejected
    content JSONB NOT NULL,
    filed_by UUID,
    filing_receipt TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reports_type ON regulatory_reports(report_type);
CREATE INDEX idx_reports_status ON regulatory_reports(status);
CREATE INDEX idx_reports_filed ON regulatory_reports(filing_date DESC);

-- AML patterns for ML training
CREATE TABLE IF NOT EXISTS aml_patterns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pattern_type VARCHAR(50) NOT NULL,
    pattern_data JSONB NOT NULL,
    is_suspicious BOOLEAN NOT NULL,
    confidence_score DECIMAL(3,2),
    detected_at TIMESTAMPTZ NOT NULL,
    reviewed_by UUID,
    review_outcome VARCHAR(20)
);

CREATE INDEX idx_patterns_type ON aml_patterns(pattern_type);
CREATE INDEX idx_patterns_detected ON aml_patterns(detected_at DESC);

-- ============================================================================
-- CLEARING ENGINE TABLES
-- ============================================================================

-- Clearing windows
CREATE TABLE IF NOT EXISTS clearing_windows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    window_name VARCHAR(50) NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    cutoff_time TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL, -- Open, Processing, Completed, Failed
    transactions_count INTEGER DEFAULT 0,
    total_gross_value DECIMAL(20,2) DEFAULT 0,
    total_net_value DECIMAL(20,2) DEFAULT 0,
    netting_efficiency DECIMAL(5,2),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_clearing_windows_status ON clearing_windows(status);
CREATE INDEX idx_clearing_windows_start ON clearing_windows(start_time DESC);

-- ============================================================================
-- SETTLEMENT ENGINE TABLES
-- ============================================================================

-- Settlement instructions
CREATE TABLE IF NOT EXISTS settlement_instructions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    clearing_window_id UUID REFERENCES clearing_windows(id),
    netting_result_id UUID REFERENCES netting_results(id),
    payer_bank_id UUID NOT NULL REFERENCES banks(id),
    payee_bank_id UUID NOT NULL REFERENCES banks(id),
    currency VARCHAR(3) NOT NULL,
    amount DECIMAL(20,2) NOT NULL,
    instruction_type VARCHAR(20) NOT NULL, -- SWIFT_MT202, SWIFT_MT103, ISO20022_PACS008
    instruction_data JSONB NOT NULL,
    status VARCHAR(20) NOT NULL, -- Pending, Sent, Confirmed, Failed
    sent_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_settlement_window ON settlement_instructions(clearing_window_id);
CREATE INDEX idx_settlement_status ON settlement_instructions(status);
CREATE INDEX idx_settlement_created ON settlement_instructions(created_at DESC);

-- ============================================================================
-- REPORTING ENGINE TABLES
-- ============================================================================

-- Report definitions
CREATE TABLE IF NOT EXISTS report_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_name VARCHAR(100) NOT NULL,
    report_type VARCHAR(50) NOT NULL,
    description TEXT,
    schedule VARCHAR(50), -- Hourly, Daily, Weekly, Monthly
    query_template TEXT,
    format VARCHAR(20), -- CSV, JSON, XLSX, PDF
    recipients TEXT[],
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Generated reports
CREATE TABLE IF NOT EXISTS generated_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_definition_id UUID REFERENCES report_definitions(id),
    report_name VARCHAR(100) NOT NULL,
    report_period_start TIMESTAMPTZ,
    report_period_end TIMESTAMPTZ,
    file_path TEXT,
    file_size BIGINT,
    row_count INTEGER,
    status VARCHAR(20) NOT NULL, -- Generating, Completed, Failed
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error_message TEXT
);

CREATE INDEX idx_generated_reports_def ON generated_reports(report_definition_id);
CREATE INDEX idx_generated_reports_generated ON generated_reports(generated_at DESC);

-- ============================================================================
-- NOTIFICATION ENGINE TABLES
-- ============================================================================

-- Notification subscriptions
CREATE TABLE IF NOT EXISTS notification_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_id UUID REFERENCES banks(id),
    user_id UUID,
    event_type VARCHAR(50) NOT NULL,
    channel VARCHAR(20) NOT NULL, -- Email, SMS, WebSocket, Webhook
    destination TEXT NOT NULL, -- email address, phone, webhook URL
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notif_subs_bank ON notification_subscriptions(bank_id);
CREATE INDEX idx_notif_subs_event ON notification_subscriptions(event_type);

-- Notification history
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id UUID REFERENCES notification_subscriptions(id),
    event_type VARCHAR(50) NOT NULL,
    channel VARCHAR(20) NOT NULL,
    destination TEXT NOT NULL,
    subject VARCHAR(255),
    content TEXT NOT NULL,
    status VARCHAR(20) NOT NULL, -- Pending, Sent, Failed, Delivered
    sent_at TIMESTAMPTZ,
    delivered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    error_message TEXT
);

CREATE INDEX idx_notifications_status ON notifications(status);
CREATE INDEX idx_notifications_created ON notifications(created_at DESC);

-- ============================================================================
-- AUDIT AND LOGGING
-- ============================================================================

-- Audit log for all critical operations
CREATE TABLE IF NOT EXISTS audit_log (
    id BIGSERIAL PRIMARY KEY,
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID,
    action VARCHAR(50) NOT NULL,
    performed_by UUID,
    ip_address INET,
    user_agent TEXT,
    changes JSONB,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_timestamp ON audit_log(timestamp DESC);

-- System metrics (for internal monitoring)
CREATE TABLE IF NOT EXISTS system_metrics (
    id BIGSERIAL PRIMARY KEY,
    metric_name VARCHAR(100) NOT NULL,
    metric_value DECIMAL(20,2),
    metric_labels JSONB,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_metrics_name ON system_metrics(metric_name);
CREATE INDEX idx_metrics_recorded ON system_metrics(recorded_at DESC);

-- ============================================================================
-- SEED DATA
-- ============================================================================

-- Insert test banks
INSERT INTO banks (id, bank_code, bank_name, country, bic_code, status) VALUES
    ('11111111-1111-1111-1111-111111111111', 'HDFC_IN', 'HDFC Bank India', 'IN', 'HDFCINBB', 'Active'),
    ('22222222-2222-2222-2222-222222222222', 'ICICI_IN', 'ICICI Bank India', 'IN', 'ICICINBB', 'Active'),
    ('33333333-3333-3333-3333-333333333333', 'ADCB_AE', 'Abu Dhabi Commercial Bank', 'AE', 'ADCBAEAA', 'Active'),
    ('44444444-4444-4444-4444-444444444444', 'FAB_AE', 'First Abu Dhabi Bank', 'AE', 'NBADAEAA', 'Active'),
    ('55555555-5555-5555-5555-555555555555', 'CITI_US', 'Citibank N.A.', 'US', 'CITIUS33', 'Active')
ON CONFLICT (id) DO NOTHING;

-- Initialize accounts for each bank
INSERT INTO accounts (bank_id, account_number, account_type, currency, balance) VALUES
    ('11111111-1111-1111-1111-111111111111', 'INR_SETTLEMENT', 'Settlement', 'INR', 100000000.00),
    ('11111111-1111-1111-1111-111111111111', 'USD_SETTLEMENT', 'Settlement', 'USD', 1000000.00),
    ('22222222-2222-2222-2222-222222222222', 'INR_SETTLEMENT', 'Settlement', 'INR', 80000000.00),
    ('22222222-2222-2222-2222-222222222222', 'USD_SETTLEMENT', 'Settlement', 'USD', 800000.00),
    ('33333333-3333-3333-3333-333333333333', 'AED_SETTLEMENT', 'Settlement', 'AED', 50000000.00),
    ('33333333-3333-3333-3333-333333333333', 'USD_SETTLEMENT', 'Settlement', 'USD', 5000000.00),
    ('44444444-4444-4444-4444-444444444444', 'AED_SETTLEMENT', 'Settlement', 'AED', 60000000.00),
    ('44444444-4444-4444-4444-444444444444', 'USD_SETTLEMENT', 'Settlement', 'USD', 6000000.00),
    ('55555555-5555-5555-5555-555555555555', 'USD_SETTLEMENT', 'Settlement', 'USD', 10000000.00)
ON CONFLICT (bank_id, account_number, currency) DO NOTHING;

-- Initialize liquidity positions
INSERT INTO liquidity_positions (bank_id, currency, available_liquidity) VALUES
    ('11111111-1111-1111-1111-111111111111', 'INR', 100000000.00),
    ('11111111-1111-1111-1111-111111111111', 'USD', 1000000.00),
    ('22222222-2222-2222-2222-222222222222', 'INR', 80000000.00),
    ('22222222-2222-2222-2222-222222222222', 'USD', 800000.00),
    ('33333333-3333-3333-3333-333333333333', 'AED', 50000000.00),
    ('33333333-3333-3333-3333-333333333333', 'USD', 5000000.00),
    ('44444444-4444-4444-4444-444444444444', 'AED', 60000000.00),
    ('44444444-4444-4444-4444-444444444444', 'USD', 6000000.00),
    ('55555555-5555-5555-5555-555555555555', 'USD', 10000000.00)
ON CONFLICT (bank_id, currency) DO NOTHING;

-- Initialize common corridors
INSERT INTO liquidity_corridors (from_currency, to_currency, primary_route, average_liquidity) VALUES
    ('INR', 'AED', '{"path": ["INR", "USD", "AED"], "banks": []}'::jsonb, 5000000.00),
    ('AED', 'INR', '{"path": ["AED", "USD", "INR"], "banks": []}'::jsonb, 5000000.00),
    ('USD', 'INR', '{"path": ["USD", "INR"], "banks": []}'::jsonb, 10000000.00),
    ('USD', 'AED', '{"path": ["USD", "AED"], "banks": []}'::jsonb, 10000000.00)
ON CONFLICT (from_currency, to_currency) DO NOTHING;

-- Initialize circuit breakers
INSERT INTO circuit_breakers (id, state, failure_count, success_count) VALUES
    ('risk_engine', 'Closed', 0, 0),
    ('compliance_engine', 'Closed', 0, 0),
    ('token_engine', 'Closed', 0, 0),
    ('obligation_engine', 'Closed', 0, 0),
    ('liquidity_router', 'Closed', 0, 0)
ON CONFLICT (id) DO NOTHING;

-- ============================================================================
-- FUNCTIONS AND TRIGGERS
-- ============================================================================

-- Function to update timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply trigger to relevant tables
CREATE TRIGGER update_transactions_updated_at BEFORE UPDATE ON transactions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_banks_updated_at BEFORE UPDATE ON banks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_accounts_updated_at BEFORE UPDATE ON accounts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to audit changes
CREATE OR REPLACE FUNCTION audit_changes()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO audit_log (entity_type, entity_id, action, changes)
    VALUES (
        TG_TABLE_NAME,
        COALESCE(NEW.id, OLD.id),
        TG_OP,
        jsonb_build_object(
            'old', to_jsonb(OLD),
            'new', to_jsonb(NEW)
        )
    );
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply audit trigger to critical tables
CREATE TRIGGER audit_transactions AFTER INSERT OR UPDATE OR DELETE ON transactions
    FOR EACH ROW EXECUTE FUNCTION audit_changes();

CREATE TRIGGER audit_obligations AFTER INSERT OR UPDATE OR DELETE ON obligations
    FOR EACH ROW EXECUTE FUNCTION audit_changes();

-- ============================================================================
-- VIEWS FOR REPORTING
-- ============================================================================

-- Transaction summary view
CREATE OR REPLACE VIEW v_transaction_summary AS
SELECT
    DATE_TRUNC('hour', created_at) as hour,
    status,
    COUNT(*) as transaction_count,
    SUM(sent_amount) as total_volume,
    sent_currency as currency
FROM transactions
WHERE created_at > NOW() - INTERVAL '24 hours'
GROUP BY hour, status, sent_currency
ORDER BY hour DESC;

-- Bank liquidity view
CREATE OR REPLACE VIEW v_bank_liquidity AS
SELECT
    b.id as bank_id,
    b.bank_code,
    b.bank_name,
    lp.currency,
    lp.available_liquidity,
    lp.reserved_liquidity,
    lp.total_inflow_24h,
    lp.total_outflow_24h,
    lp.position_score,
    lp.last_updated
FROM banks b
JOIN liquidity_positions lp ON b.id = lp.bank_id
WHERE b.status = 'Active';

-- Risk metrics view
CREATE OR REPLACE VIEW v_risk_metrics AS
SELECT
    DATE_TRUNC('day', calculated_at) as date,
    decision,
    COUNT(*) as count,
    AVG(overall_score) as avg_score,
    MIN(overall_score) as min_score,
    MAX(overall_score) as max_score
FROM risk_scores
WHERE calculated_at > NOW() - INTERVAL '30 days'
GROUP BY date, decision
ORDER BY date DESC;

-- ============================================================================
-- COMPLETION
-- ============================================================================

-- Log schema initialization
INSERT INTO audit_log (entity_type, action, changes)
VALUES ('database', 'SCHEMA_INIT', '{"message": "DelTran MVP schema initialized successfully"}'::jsonb);

-- Grant permissions (adjust as needed)
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO deltran;
-- GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO deltran;

COMMIT;
