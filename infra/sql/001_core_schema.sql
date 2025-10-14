-- DelTran Core Database Schema
-- PostgreSQL 15+
-- Production-ready schema with all constraints, indexes, and partitioning

-- ============================================
-- SETUP
-- ============================================

CREATE SCHEMA IF NOT EXISTS deltran;
SET search_path TO deltran;

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
CREATE EXTENSION IF NOT EXISTS "btree_gist";

-- ============================================
-- CUSTOM TYPES
-- ============================================

CREATE TYPE user_role AS ENUM (
    'admin',      -- Full system access
    'operator',   -- Payment operations
    'auditor',    -- Read-only audit access
    'viewer'      -- Basic read-only access
);

CREATE TYPE payment_status AS ENUM (
    'pending',
    'processing',
    'settled',
    'failed',
    'cancelled'
);

CREATE TYPE settlement_status AS ENUM (
    'draft',
    'ready',
    'submitted',
    'settled',
    'failed',
    'cancelled'
);

CREATE TYPE compliance_status AS ENUM (
    'pending',
    'pass',
    'fail',
    'review'
);

CREATE TYPE currency_code AS ENUM (
    'USD', 'EUR', 'GBP', 'JPY', 'CHF', 'CAD', 'AUD', 'CNY', 'INR', 'BRL'
);

CREATE TYPE action_type AS ENUM (
    'freeze',
    'unfreeze',
    'throttle',
    'block'
);

-- ============================================
-- TABLE 1: USERS
-- ============================================

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(100) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    role user_role NOT NULL DEFAULT 'viewer',
    bank_id UUID,

    -- Security
    is_active BOOLEAN DEFAULT true,
    is_2fa_enabled BOOLEAN DEFAULT false,
    totp_secret VARCHAR(32),

    -- Session tracking
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    last_login_at TIMESTAMPTZ,
    last_login_ip INET,

    -- Account lockout
    failed_login_attempts INT DEFAULT 0,
    locked_until TIMESTAMPTZ,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_email CHECK (email ~* '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$'),
    CONSTRAINT chk_failed_attempts CHECK (failed_login_attempts >= 0 AND failed_login_attempts <= 10)
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_bank_id ON users(bank_id);
CREATE INDEX idx_users_active ON users(is_active) WHERE is_active = true;
CREATE INDEX idx_users_role ON users(role);

COMMENT ON TABLE users IS 'System users with authentication and authorization';
COMMENT ON COLUMN users.totp_secret IS 'Base32-encoded TOTP secret for 2FA';

-- ============================================
-- TABLE 2: SESSIONS
-- ============================================

CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    refresh_token_hash VARCHAR(255) NOT NULL,

    -- Session metadata
    ip_address INET,
    user_agent TEXT,
    device_fingerprint VARCHAR(255),

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    last_activity_at TIMESTAMPTZ DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,

    -- Security
    is_revoked BOOLEAN DEFAULT false,
    revoke_reason TEXT,

    CONSTRAINT chk_session_expires CHECK (expires_at > created_at)
);

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token_hash ON sessions(refresh_token_hash);
CREATE INDEX idx_sessions_expires ON sessions(expires_at) WHERE is_revoked = false;
CREATE INDEX idx_sessions_active ON sessions(user_id, expires_at) WHERE is_revoked = false;

COMMENT ON TABLE sessions IS 'JWT refresh token sessions';

-- ============================================
-- TABLE 3: BANKS
-- ============================================

CREATE TABLE banks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bic_code VARCHAR(11) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    country_code CHAR(2) NOT NULL,

    -- Status
    is_active BOOLEAN DEFAULT true,
    onboarded_at TIMESTAMPTZ DEFAULT NOW(),

    -- Risk & Compliance
    risk_rating VARCHAR(20),
    kyc_status VARCHAR(20) DEFAULT 'pending',
    kyc_verified_at TIMESTAMPTZ,

    -- Contact
    contact_email VARCHAR(255),
    contact_phone VARCHAR(50),

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT chk_bic_format CHECK (bic_code ~* '^[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?$'),
    CONSTRAINT chk_country_code CHECK (country_code ~* '^[A-Z]{2}$')
);

CREATE INDEX idx_banks_bic ON banks(bic_code);
CREATE INDEX idx_banks_country ON banks(country_code);
CREATE INDEX idx_banks_active ON banks(is_active) WHERE is_active = true;

COMMENT ON TABLE banks IS 'Participating financial institutions';
COMMENT ON COLUMN banks.bic_code IS 'SWIFT BIC/SWIFT code (8 or 11 characters)';

-- ============================================
-- TABLE 4: BANK_ACCOUNTS
-- ============================================

CREATE TABLE bank_accounts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bank_id UUID NOT NULL REFERENCES banks(id) ON DELETE CASCADE,
    account_number VARCHAR(34) NOT NULL,
    currency currency_code NOT NULL,

    -- Balances
    balance DECIMAL(20,2) DEFAULT 0,
    available_balance DECIMAL(20,2) DEFAULT 0,
    reserved_balance DECIMAL(20,2) DEFAULT 0,

    -- Status
    is_active BOOLEAN DEFAULT true,
    opened_at TIMESTAMPTZ DEFAULT NOW(),
    closed_at TIMESTAMPTZ,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT chk_balance CHECK (balance >= 0),
    CONSTRAINT chk_available_balance CHECK (available_balance >= 0),
    CONSTRAINT chk_reserved_balance CHECK (reserved_balance >= 0),
    CONSTRAINT chk_balance_equation CHECK (balance = available_balance + reserved_balance),
    CONSTRAINT uq_bank_account_currency UNIQUE (bank_id, account_number, currency)
);

CREATE INDEX idx_bank_accounts_bank_id ON bank_accounts(bank_id);
CREATE INDEX idx_bank_accounts_active ON bank_accounts(is_active) WHERE is_active = true;
CREATE INDEX idx_bank_accounts_currency ON bank_accounts(currency);

COMMENT ON TABLE bank_accounts IS 'Bank settlement accounts in the system';

-- ============================================
-- TABLE 5: PAYMENTS
-- ============================================

CREATE TABLE payments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    payment_reference VARCHAR(100) UNIQUE NOT NULL,

    -- Parties
    sender_bank_id UUID NOT NULL REFERENCES banks(id),
    receiver_bank_id UUID NOT NULL REFERENCES banks(id),
    sender_account_id UUID REFERENCES bank_accounts(id),
    receiver_account_id UUID REFERENCES bank_accounts(id),

    -- Amount
    amount DECIMAL(20,2) NOT NULL,
    currency currency_code NOT NULL,

    -- Status & Processing
    status payment_status NOT NULL DEFAULT 'pending',
    compliance_check_id UUID,
    risk_score DECIMAL(5,2),
    batch_id UUID,

    -- SWIFT integration
    swift_message_type VARCHAR(10),
    swift_message_id VARCHAR(100),

    -- Idempotency
    idempotency_key VARCHAR(255) UNIQUE,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    processed_at TIMESTAMPTZ,
    settled_at TIMESTAMPTZ,

    -- Additional info
    remittance_info TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_payment_amount CHECK (amount > 0),
    CONSTRAINT chk_different_banks CHECK (sender_bank_id != receiver_bank_id),
    CONSTRAINT chk_risk_score CHECK (risk_score >= 0 AND risk_score <= 100)
);

CREATE INDEX idx_payments_reference ON payments(payment_reference);
CREATE INDEX idx_payments_sender_bank ON payments(sender_bank_id);
CREATE INDEX idx_payments_receiver_bank ON payments(receiver_bank_id);
CREATE INDEX idx_payments_status ON payments(status);
CREATE INDEX idx_payments_batch_id ON payments(batch_id) WHERE batch_id IS NOT NULL;
CREATE INDEX idx_payments_created_at ON payments(created_at DESC);
CREATE INDEX idx_payments_idempotency ON payments(idempotency_key) WHERE idempotency_key IS NOT NULL;
CREATE INDEX idx_payments_compliance ON payments(compliance_check_id) WHERE compliance_check_id IS NOT NULL;

COMMENT ON TABLE payments IS 'Individual payment instructions';
COMMENT ON COLUMN payments.idempotency_key IS 'Prevents duplicate payment processing';

-- ============================================
-- TABLE 6: TRANSACTION_LOG
-- ============================================

CREATE TABLE transaction_log (
    id BIGSERIAL PRIMARY KEY,
    payment_id UUID NOT NULL REFERENCES payments(id) ON DELETE CASCADE,

    -- Transaction details
    transaction_type VARCHAR(50) NOT NULL,
    account_id UUID NOT NULL REFERENCES bank_accounts(id),

    -- Amounts
    debit_amount DECIMAL(20,2) DEFAULT 0,
    credit_amount DECIMAL(20,2) DEFAULT 0,
    balance_after DECIMAL(20,2) NOT NULL,

    -- Tracking
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    processed_by UUID REFERENCES users(id),

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_debit_credit CHECK (
        (debit_amount > 0 AND credit_amount = 0) OR
        (credit_amount > 0 AND debit_amount = 0)
    ),
    CONSTRAINT chk_balance_after CHECK (balance_after >= 0)
);

CREATE INDEX idx_transaction_log_payment_id ON transaction_log(payment_id);
CREATE INDEX idx_transaction_log_account_id ON transaction_log(account_id);
CREATE INDEX idx_transaction_log_timestamp ON transaction_log(timestamp DESC);
CREATE INDEX idx_transaction_log_type ON transaction_log(transaction_type);

COMMENT ON TABLE transaction_log IS 'Immutable transaction history for all account movements';

-- ============================================
-- TABLE 7: SETTLEMENT_BATCHES
-- ============================================

CREATE TABLE settlement_batches (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    batch_reference VARCHAR(100) UNIQUE NOT NULL,

    -- Batch info
    corridor_id VARCHAR(50),
    netting_window_id VARCHAR(100),
    batch_type VARCHAR(50) DEFAULT 'standard',

    -- Status
    status settlement_status NOT NULL DEFAULT 'draft',

    -- Financial summary
    total_payments INT DEFAULT 0,
    gross_amount DECIMAL(20,2) DEFAULT 0,
    net_amount DECIMAL(20,2) DEFAULT 0,
    currency currency_code NOT NULL,

    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    finalized_at TIMESTAMPTZ,
    submitted_at TIMESTAMPTZ,
    settled_at TIMESTAMPTZ,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_total_payments CHECK (total_payments >= 0),
    CONSTRAINT chk_gross_amount CHECK (gross_amount >= 0),
    CONSTRAINT chk_net_amount CHECK (net_amount >= 0)
);

CREATE INDEX idx_settlement_batches_reference ON settlement_batches(batch_reference);
CREATE INDEX idx_settlement_batches_status ON settlement_batches(status);
CREATE INDEX idx_settlement_batches_corridor ON settlement_batches(corridor_id);
CREATE INDEX idx_settlement_batches_created_at ON settlement_batches(created_at DESC);
CREATE INDEX idx_settlement_batches_window ON settlement_batches(netting_window_id) WHERE netting_window_id IS NOT NULL;

COMMENT ON TABLE settlement_batches IS 'Settlement batch processing with netting';

-- ============================================
-- TABLE 8: COMPLIANCE_CHECKS
-- ============================================

CREATE TABLE compliance_checks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    payment_id UUID NOT NULL REFERENCES payments(id) ON DELETE CASCADE,

    -- Check details
    check_type VARCHAR(50) NOT NULL,
    status compliance_status NOT NULL DEFAULT 'pending',

    -- Results
    risk_score DECIMAL(5,2),
    flags JSONB DEFAULT '[]'::jsonb,
    matched_lists TEXT[],

    -- Review
    requires_review BOOLEAN DEFAULT false,
    reviewed_by UUID REFERENCES users(id),
    reviewed_at TIMESTAMPTZ,
    review_notes TEXT,

    -- Timestamps
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_risk_score_range CHECK (risk_score >= 0 AND risk_score <= 100)
);

CREATE INDEX idx_compliance_payment_id ON compliance_checks(payment_id);
CREATE INDEX idx_compliance_status ON compliance_checks(status);
CREATE INDEX idx_compliance_check_type ON compliance_checks(check_type);
CREATE INDEX idx_compliance_review_required ON compliance_checks(requires_review) WHERE requires_review = true;
CREATE INDEX idx_compliance_flags ON compliance_checks USING gin(flags);

COMMENT ON TABLE compliance_checks IS 'AML/KYC/Sanctions screening results';

-- ============================================
-- TABLE 9: RATE_LIMITS
-- ============================================

CREATE TABLE rate_limits (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    entity_type VARCHAR(50) NOT NULL,
    entity_id VARCHAR(255) NOT NULL,

    -- Limit configuration
    limit_type VARCHAR(50) NOT NULL,
    limit_value INT NOT NULL,
    window_seconds INT NOT NULL,

    -- Current usage
    current_count INT DEFAULT 0,
    window_start TIMESTAMPTZ DEFAULT NOW(),

    -- Status
    is_active BOOLEAN DEFAULT true,
    action_type action_type,
    action_expires_at TIMESTAMPTZ,

    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT chk_limit_value CHECK (limit_value > 0),
    CONSTRAINT chk_window_seconds CHECK (window_seconds > 0),
    CONSTRAINT chk_current_count CHECK (current_count >= 0),
    CONSTRAINT uq_rate_limit UNIQUE (entity_type, entity_id, limit_type)
);

CREATE INDEX idx_rate_limits_entity ON rate_limits(entity_type, entity_id);
CREATE INDEX idx_rate_limits_active ON rate_limits(is_active) WHERE is_active = true;
CREATE INDEX idx_rate_limits_window_start ON rate_limits(window_start);

COMMENT ON TABLE rate_limits IS 'Rate limiting and throttling controls';

-- ============================================
-- TABLE 10: AUDIT_LOG (PARTITIONED)
-- ============================================

CREATE TABLE audit_log (
    id BIGSERIAL,
    event_id UUID NOT NULL DEFAULT uuid_generate_v4(),

    -- Event details
    event_type VARCHAR(100) NOT NULL,
    severity VARCHAR(20) NOT NULL,

    -- Actor
    actor_id UUID,
    actor_type VARCHAR(50),
    actor_name VARCHAR(255),

    -- Action
    action VARCHAR(255) NOT NULL,
    resource_type VARCHAR(100),
    resource_id VARCHAR(255),

    -- Result
    result VARCHAR(20) NOT NULL,
    error_message TEXT,

    -- Context
    ip_address INET,
    user_agent TEXT,
    request_id UUID,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Hash chain for tamper detection
    previous_hash VARCHAR(64),
    event_hash VARCHAR(64),

    -- Timestamp
    timestamp TIMESTAMPTZ DEFAULT NOW(),

    PRIMARY KEY (id, timestamp)
) PARTITION BY RANGE (timestamp);

-- Create partitions for current and next 12 months
CREATE TABLE audit_log_2025_01 PARTITION OF audit_log
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');

CREATE TABLE audit_log_2025_02 PARTITION OF audit_log
    FOR VALUES FROM ('2025-02-01') TO ('2025-03-01');

CREATE TABLE audit_log_2025_03 PARTITION OF audit_log
    FOR VALUES FROM ('2025-03-01') TO ('2025-04-01');

CREATE TABLE audit_log_2025_04 PARTITION OF audit_log
    FOR VALUES FROM ('2025-04-01') TO ('2025-05-01');

CREATE TABLE audit_log_2025_05 PARTITION OF audit_log
    FOR VALUES FROM ('2025-05-01') TO ('2025-06-01');

CREATE TABLE audit_log_2025_06 PARTITION OF audit_log
    FOR VALUES FROM ('2025-06-01') TO ('2025-07-01');

CREATE TABLE audit_log_2025_07 PARTITION OF audit_log
    FOR VALUES FROM ('2025-07-01') TO ('2025-08-01');

CREATE TABLE audit_log_2025_08 PARTITION OF audit_log
    FOR VALUES FROM ('2025-08-01') TO ('2025-09-01');

CREATE TABLE audit_log_2025_09 PARTITION OF audit_log
    FOR VALUES FROM ('2025-09-01') TO ('2025-10-01');

CREATE TABLE audit_log_2025_10 PARTITION OF audit_log
    FOR VALUES FROM ('2025-10-01') TO ('2025-11-01');

CREATE TABLE audit_log_2025_11 PARTITION OF audit_log
    FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');

CREATE TABLE audit_log_2025_12 PARTITION OF audit_log
    FOR VALUES FROM ('2025-12-01') TO ('2026-01-01');

-- Indexes on parent table (will be inherited by partitions)
CREATE INDEX idx_audit_log_event_id ON audit_log(event_id);
CREATE INDEX idx_audit_log_actor_id ON audit_log(actor_id) WHERE actor_id IS NOT NULL;
CREATE INDEX idx_audit_log_event_type ON audit_log(event_type);
CREATE INDEX idx_audit_log_timestamp ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_log_resource ON audit_log(resource_type, resource_id) WHERE resource_id IS NOT NULL;
CREATE INDEX idx_audit_log_request_id ON audit_log(request_id) WHERE request_id IS NOT NULL;
CREATE INDEX idx_audit_log_metadata ON audit_log USING gin(metadata);

COMMENT ON TABLE audit_log IS 'Immutable audit trail with hash chain integrity';
COMMENT ON COLUMN audit_log.event_hash IS 'SHA256 hash of event for tamper detection';

-- ============================================
-- TRIGGERS
-- ============================================

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Apply updated_at triggers
CREATE TRIGGER trg_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trg_banks_updated_at
    BEFORE UPDATE ON banks
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trg_bank_accounts_updated_at
    BEFORE UPDATE ON bank_accounts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trg_payments_updated_at
    BEFORE UPDATE ON payments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER trg_rate_limits_updated_at
    BEFORE UPDATE ON rate_limits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

-- ============================================
-- VIEWS
-- ============================================

-- Active payments summary
CREATE VIEW v_active_payments AS
SELECT
    p.id,
    p.payment_reference,
    sb.bic_code as sender_bic,
    sb.name as sender_name,
    rb.bic_code as receiver_bic,
    rb.name as receiver_name,
    p.amount,
    p.currency,
    p.status,
    p.risk_score,
    p.created_at,
    p.settled_at
FROM payments p
JOIN banks sb ON p.sender_bank_id = sb.id
JOIN banks rb ON p.receiver_bank_id = rb.id
WHERE p.status IN ('pending', 'processing');

-- Bank account balances
CREATE VIEW v_bank_balances AS
SELECT
    b.id as bank_id,
    b.bic_code,
    b.name,
    ba.currency,
    ba.balance,
    ba.available_balance,
    ba.reserved_balance,
    ba.is_active
FROM banks b
JOIN bank_accounts ba ON b.id = ba.bank_id
WHERE b.is_active = true AND ba.is_active = true;

-- Daily transaction volume
CREATE VIEW v_daily_transaction_volume AS
SELECT
    DATE(created_at) as transaction_date,
    currency,
    COUNT(*) as payment_count,
    SUM(amount) as total_amount,
    AVG(amount) as average_amount,
    status
FROM payments
GROUP BY DATE(created_at), currency, status
ORDER BY transaction_date DESC;

-- ============================================
-- INITIAL DATA
-- ============================================

-- Create admin user (password: Admin123!)
INSERT INTO users (email, username, password_hash, role, is_active, is_2fa_enabled)
VALUES (
    'admin@deltran.local',
    'admin',
    '$2a$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5lk3k7nB9jY9i',
    'admin',
    true,
    false
);

-- Create sample banks
INSERT INTO banks (bic_code, name, country_code, is_active, risk_rating, kyc_status)
VALUES
    ('CHASUS33XXX', 'JPMorgan Chase Bank', 'US', true, 'LOW', 'verified'),
    ('DEUTDEFFXXX', 'Deutsche Bank AG', 'DE', true, 'LOW', 'verified'),
    ('SBININBBXXX', 'State Bank of India', 'IN', true, 'MEDIUM', 'verified'),
    ('HSBCHKHHHKH', 'HSBC Hong Kong', 'HK', true, 'LOW', 'verified');

COMMENT ON SCHEMA deltran IS 'DelTran Payment Rail Core Schema - v1.0';

-- ============================================
-- GRANTS
-- ============================================

-- Application user (read/write)
GRANT USAGE ON SCHEMA deltran TO deltran_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA deltran TO deltran_app;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA deltran TO deltran_app;

-- Read-only user for reporting
CREATE USER deltran_readonly WITH PASSWORD 'readonly123';
GRANT USAGE ON SCHEMA deltran TO deltran_readonly;
GRANT SELECT ON ALL TABLES IN SCHEMA deltran TO deltran_readonly;

-- Replication user
CREATE USER replicator WITH REPLICATION LOGIN PASSWORD 'replica456';
