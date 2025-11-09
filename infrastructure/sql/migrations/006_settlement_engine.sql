-- ============================================================================
-- SETTLEMENT ENGINE SCHEMA
-- Version: 1.0
-- Description: Tables for atomic settlement operations with external banks
-- ============================================================================

-- Settlement transactions with full lifecycle tracking
CREATE TABLE IF NOT EXISTS settlement_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    obligation_id UUID NOT NULL,
    clearing_window_id BIGINT,

    -- Parties
    from_bank VARCHAR(20) NOT NULL,
    to_bank VARCHAR(20) NOT NULL,
    from_bank_id UUID NOT NULL REFERENCES banks(id),
    to_bank_id UUID NOT NULL REFERENCES banks(id),

    -- Amount
    amount DECIMAL(20, 2) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Settlement details
    settlement_type VARCHAR(30) DEFAULT 'Standard', -- Standard, Express, Emergency
    settlement_method VARCHAR(30) NOT NULL, -- SWIFT, SEPA, LocalACH, Mock
    priority VARCHAR(10) DEFAULT 'normal', -- normal, high, urgent

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    substatus VARCHAR(50),

    -- External references
    external_reference VARCHAR(100),
    bank_confirmation VARCHAR(100),
    payment_rail VARCHAR(30),

    -- Atomic operation tracking
    atomic_operation_id UUID,
    lock_id UUID,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    validated_at TIMESTAMPTZ,
    funds_locked_at TIMESTAMPTZ,
    initiated_at TIMESTAMPTZ,
    executed_at TIMESTAMPTZ,
    confirmed_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,
    rolled_back_at TIMESTAMPTZ,

    -- Error tracking
    error_message TEXT,
    error_code VARCHAR(50),
    retry_count INT DEFAULT 0,
    max_retries INT DEFAULT 3,
    last_retry_at TIMESTAMPTZ,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT positive_amount CHECK (amount > 0),
    CONSTRAINT valid_retry_count CHECK (retry_count <= max_retries),
    INDEX idx_settlements_obligation (obligation_id),
    INDEX idx_settlements_window (clearing_window_id),
    INDEX idx_settlements_from_bank (from_bank_id),
    INDEX idx_settlements_to_bank (to_bank_id),
    INDEX idx_settlements_status (status),
    INDEX idx_settlements_created (created_at DESC),
    INDEX idx_settlements_atomic_op (atomic_operation_id)
);

-- Nostro accounts (our accounts at correspondent banks)
CREATE TABLE IF NOT EXISTS nostro_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank VARCHAR(20) NOT NULL,
    bank_id UUID NOT NULL REFERENCES banks(id),
    account_number VARCHAR(50) NOT NULL,
    account_name VARCHAR(255) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Balances
    ledger_balance DECIMAL(20, 2) NOT NULL DEFAULT 0,
    available_balance DECIMAL(20, 2) NOT NULL DEFAULT 0,
    locked_balance DECIMAL(20, 2) NOT NULL DEFAULT 0,

    -- Computed field for verification
    computed_available AS (ledger_balance - locked_balance) STORED,

    -- Settlement windows
    settlement_window_start TIME,
    settlement_window_end TIME,

    -- Account status
    is_active BOOLEAN DEFAULT true,
    is_reconciled BOOLEAN DEFAULT false,
    last_reconciled TIMESTAMPTZ,
    next_reconciliation TIMESTAMPTZ,

    -- Metadata
    swift_code VARCHAR(11),
    iban VARCHAR(34),
    correspondent_bank VARCHAR(255),
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(bank, currency),
    CONSTRAINT positive_balances CHECK (
        ledger_balance >= 0 AND
        available_balance >= 0 AND
        locked_balance >= 0
    ),
    CONSTRAINT balance_integrity CHECK (available_balance + locked_balance <= ledger_balance),
    INDEX idx_nostro_bank (bank_id),
    INDEX idx_nostro_currency (currency),
    INDEX idx_nostro_active (is_active),
    INDEX idx_nostro_reconciled (is_reconciled, next_reconciliation)
);

-- Vostro accounts (correspondent banks' accounts with us)
CREATE TABLE IF NOT EXISTS vostro_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank VARCHAR(20) NOT NULL,
    bank_id UUID NOT NULL REFERENCES banks(id),
    account_number VARCHAR(50) NOT NULL,
    account_name VARCHAR(255) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Balances
    balance DECIMAL(20, 2) NOT NULL DEFAULT 0,
    available_balance DECIMAL(20, 2) NOT NULL DEFAULT 0,
    reserved_balance DECIMAL(20, 2) NOT NULL DEFAULT 0,

    -- Credit facility
    credit_limit DECIMAL(20, 2) DEFAULT 0,
    utilized_credit DECIMAL(20, 2) DEFAULT 0,

    -- Account status
    is_active BOOLEAN DEFAULT true,
    last_activity TIMESTAMPTZ,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(bank, currency),
    CONSTRAINT positive_vostro_balances CHECK (
        balance >= 0 AND
        available_balance >= 0 AND
        reserved_balance >= 0
    ),
    CONSTRAINT credit_limit_check CHECK (utilized_credit <= credit_limit),
    INDEX idx_vostro_bank (bank_id),
    INDEX idx_vostro_currency (currency),
    INDEX idx_vostro_active (is_active)
);

-- Fund locks for atomic settlement operations
CREATE TABLE IF NOT EXISTS fund_locks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    settlement_id UUID REFERENCES settlement_transactions(id),
    atomic_operation_id UUID NOT NULL,

    -- Lock details
    nostro_account_id UUID NOT NULL REFERENCES nostro_accounts(id),
    bank VARCHAR(20) NOT NULL,
    amount DECIMAL(20, 2) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Lock status
    status VARCHAR(20) NOT NULL DEFAULT 'active', -- active, released, expired

    -- Timestamps
    locked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    released_at TIMESTAMPTZ,
    released_by VARCHAR(100),

    -- Metadata
    lock_reason TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT positive_lock_amount CHECK (amount > 0),
    CONSTRAINT valid_lock_expiry CHECK (expires_at > locked_at),
    INDEX idx_locks_settlement (settlement_id),
    INDEX idx_locks_operation (atomic_operation_id),
    INDEX idx_locks_account (nostro_account_id),
    INDEX idx_locks_status (status),
    INDEX idx_locks_expires (expires_at)
);

-- Atomic operations for settlement
CREATE TABLE IF NOT EXISTS settlement_atomic_operations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    settlement_id UUID NOT NULL REFERENCES settlement_transactions(id),
    operation_type VARCHAR(50) NOT NULL,
    state VARCHAR(20) NOT NULL DEFAULT 'InProgress',

    -- Checkpoints
    current_checkpoint VARCHAR(100),
    checkpoints JSONB DEFAULT '[]'::jsonb,

    -- Timestamps
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    committed_at TIMESTAMPTZ,
    rolled_back_at TIMESTAMPTZ,

    -- Rollback data
    rollback_reason TEXT,
    rollback_data JSONB,
    auto_rollback BOOLEAN DEFAULT true,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    INDEX idx_settlement_atomic_settlement (settlement_id),
    INDEX idx_settlement_atomic_state (state),
    INDEX idx_settlement_atomic_type (operation_type)
);

-- Operation checkpoints for granular rollback
CREATE TABLE IF NOT EXISTS settlement_operation_checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    operation_id UUID NOT NULL REFERENCES settlement_atomic_operations(id) ON DELETE CASCADE,
    checkpoint_name VARCHAR(100) NOT NULL,
    checkpoint_order INTEGER NOT NULL,

    -- Checkpoint data
    checkpoint_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    rollback_data JSONB NOT NULL DEFAULT '{}'::jsonb,

    -- Status
    status VARCHAR(20) DEFAULT 'completed', -- pending, completed, rolled_back

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    rolled_back_at TIMESTAMPTZ,

    UNIQUE(operation_id, checkpoint_order),
    INDEX idx_settlement_checkpoints_operation (operation_id, checkpoint_order)
);

-- Reconciliation reports
CREATE TABLE IF NOT EXISTS settlement_reconciliation_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_date DATE NOT NULL,
    report_type VARCHAR(30) NOT NULL, -- Daily, Weekly, Monthly

    -- Accounts reconciled
    total_accounts INTEGER NOT NULL DEFAULT 0,
    balanced_accounts INTEGER NOT NULL DEFAULT 0,
    discrepancy_accounts INTEGER NOT NULL DEFAULT 0,

    -- Financial summary
    total_discrepancy DECIMAL(20, 2) DEFAULT 0,
    max_discrepancy DECIMAL(20, 2) DEFAULT 0,

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'InProgress',
    reconciliation_started TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    reconciliation_completed TIMESTAMPTZ,

    -- Details
    account_details JSONB DEFAULT '[]'::jsonb,
    discrepancies JSONB DEFAULT '[]'::jsonb,

    -- Metadata
    reconciled_by VARCHAR(100),
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(report_date, report_type),
    INDEX idx_reconciliation_date (report_date DESC),
    INDEX idx_reconciliation_status (status)
);

-- Account reconciliation details
CREATE TABLE IF NOT EXISTS settlement_account_reconciliations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id UUID NOT NULL REFERENCES settlement_reconciliation_reports(id),
    account_type VARCHAR(20) NOT NULL, -- Nostro, Vostro
    account_id UUID NOT NULL,

    -- Balances
    internal_balance DECIMAL(20, 2) NOT NULL,
    external_balance DECIMAL(20, 2) NOT NULL,
    discrepancy DECIMAL(20, 2) NOT NULL,

    -- Status
    reconciliation_status VARCHAR(30) NOT NULL, -- Balanced, Unresolved, Identified, InvestigationRequired

    -- Unmatched transactions
    unmatched_transactions JSONB DEFAULT '[]'::jsonb,
    investigation_notes TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_account_recon_report (report_id),
    INDEX idx_account_recon_status (reconciliation_status)
);

-- Settlement windows configuration
CREATE TABLE IF NOT EXISTS settlement_windows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    currency VARCHAR(3) NOT NULL,
    payment_rail VARCHAR(30) NOT NULL, -- SWIFT, SEPA, LocalACH

    -- Window times (UTC)
    window_start TIME NOT NULL,
    window_end TIME NOT NULL,
    days_of_week INT[] DEFAULT ARRAY[1,2,3,4,5], -- Mon-Fri

    -- Cutoff times
    same_day_cutoff TIME,
    next_day_cutoff TIME,

    -- Status
    is_active BOOLEAN DEFAULT true,

    -- Metadata
    timezone VARCHAR(50),
    holiday_calendar VARCHAR(50),
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(currency, payment_rail, window_start),
    CONSTRAINT valid_window_times CHECK (window_end > window_start),
    INDEX idx_settlement_windows_currency (currency),
    INDEX idx_settlement_windows_active (is_active)
);

-- External bank API configurations
CREATE TABLE IF NOT EXISTS external_bank_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_code VARCHAR(20) NOT NULL UNIQUE,
    bank_name VARCHAR(255) NOT NULL,

    -- API configuration
    api_type VARCHAR(30) NOT NULL, -- SWIFT, SEPA, Mock, Custom
    api_endpoint VARCHAR(500),
    api_version VARCHAR(20),

    -- Authentication
    auth_type VARCHAR(30), -- OAuth2, APIKey, Certificate, Mock
    auth_credentials_encrypted BYTEA,

    -- Settings
    timeout_seconds INTEGER DEFAULT 60,
    retry_attempts INTEGER DEFAULT 3,
    retry_delay_seconds INTEGER DEFAULT 10,

    -- Features
    supports_instant_settlement BOOLEAN DEFAULT false,
    supports_status_polling BOOLEAN DEFAULT true,
    supports_cancellation BOOLEAN DEFAULT false,

    -- Status
    is_active BOOLEAN DEFAULT true,
    is_mock BOOLEAN DEFAULT false,

    -- Mock settings (for MVP)
    mock_latency_ms INTEGER DEFAULT 500,
    mock_success_rate DECIMAL(3,2) DEFAULT 0.95,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_external_banks_active (is_active),
    INDEX idx_external_banks_mock (is_mock)
);

-- Transfer status tracking
CREATE TABLE IF NOT EXISTS external_transfer_status (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    settlement_id UUID NOT NULL REFERENCES settlement_transactions(id),
    external_reference VARCHAR(100) NOT NULL,
    bank_code VARCHAR(20) NOT NULL,

    -- Status
    status VARCHAR(30) NOT NULL, -- Pending, Processing, Completed, Failed, Cancelled
    status_message TEXT,

    -- Timestamps
    initiated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,

    -- Polling
    poll_count INTEGER DEFAULT 0,
    max_polls INTEGER DEFAULT 100,
    poll_interval_seconds INTEGER DEFAULT 5,

    -- Metadata
    status_details JSONB DEFAULT '{}'::jsonb,

    INDEX idx_transfer_status_settlement (settlement_id),
    INDEX idx_transfer_status_external_ref (external_reference),
    INDEX idx_transfer_status_status (status)
);

-- Functions and triggers

-- Update nostro account trigger
CREATE OR REPLACE FUNCTION update_nostro_available_balance()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE nostro_accounts
    SET available_balance = ledger_balance - locked_balance,
        updated_at = NOW()
    WHERE id = NEW.nostro_account_id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_nostro_on_lock
    AFTER INSERT OR UPDATE ON fund_locks
    FOR EACH ROW
    WHEN (NEW.status = 'active')
    EXECUTE FUNCTION update_nostro_available_balance();

-- Automatically release expired locks
CREATE OR REPLACE FUNCTION release_expired_fund_locks()
RETURNS INTEGER AS $$
DECLARE
    released_count INTEGER;
BEGIN
    UPDATE fund_locks
    SET status = 'expired',
        released_at = NOW(),
        released_by = 'system_auto'
    WHERE status = 'active'
        AND expires_at < NOW();

    GET DIAGNOSTICS released_count = ROW_COUNT;
    RETURN released_count;
END;
$$ LANGUAGE plpgsql;

-- Update settlement status trigger
CREATE OR REPLACE FUNCTION log_settlement_status_change()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status IS DISTINCT FROM OLD.status THEN
        INSERT INTO audit_log (
            entity_type,
            entity_id,
            action,
            changes
        ) VALUES (
            'settlement_transaction',
            NEW.id,
            'status_changed',
            jsonb_build_object(
                'old_status', OLD.status,
                'new_status', NEW.status,
                'obligation_id', NEW.obligation_id,
                'amount', NEW.amount,
                'currency', NEW.currency
            )
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_log_settlement_status
    AFTER UPDATE OF status ON settlement_transactions
    FOR EACH ROW
    EXECUTE FUNCTION log_settlement_status_change();

-- Views

-- Active settlements summary
CREATE OR REPLACE VIEW v_settlement_active_summary AS
SELECT
    st.status,
    st.settlement_method,
    st.priority,
    COUNT(*) as count,
    SUM(st.amount) as total_amount,
    st.currency,
    AVG(EXTRACT(EPOCH FROM (NOW() - st.created_at))) as avg_age_seconds
FROM settlement_transactions st
WHERE st.status IN ('pending', 'validated', 'initiated', 'executed')
GROUP BY st.status, st.settlement_method, st.priority, st.currency
ORDER BY st.status, st.settlement_method;

-- Nostro account balances
CREATE OR REPLACE VIEW v_nostro_balances AS
SELECT
    na.id,
    na.bank,
    b.bank_name,
    na.currency,
    na.ledger_balance,
    na.available_balance,
    na.locked_balance,
    na.is_active,
    na.last_reconciled,
    COALESCE(fl.active_locks_count, 0) as active_locks,
    COALESCE(fl.locked_amount, 0) as total_locked_amount
FROM nostro_accounts na
JOIN banks b ON na.bank_id = b.id
LEFT JOIN (
    SELECT
        nostro_account_id,
        COUNT(*) as active_locks_count,
        SUM(amount) as locked_amount
    FROM fund_locks
    WHERE status = 'active'
    GROUP BY nostro_account_id
) fl ON na.id = fl.nostro_account_id
WHERE na.is_active = true
ORDER BY na.bank, na.currency;

-- Settlement metrics
CREATE OR REPLACE VIEW v_settlement_metrics_24h AS
SELECT
    DATE_TRUNC('hour', st.created_at) as hour,
    st.status,
    st.settlement_method,
    COUNT(*) as settlement_count,
    SUM(st.amount) as total_amount,
    st.currency,
    AVG(EXTRACT(EPOCH FROM (COALESCE(st.completed_at, NOW()) - st.created_at))) as avg_duration_seconds,
    COUNT(CASE WHEN st.retry_count > 0 THEN 1 END) as retried_count,
    COUNT(CASE WHEN st.status = 'failed' THEN 1 END) as failed_count
FROM settlement_transactions st
WHERE st.created_at > NOW() - INTERVAL '24 hours'
GROUP BY hour, st.status, st.settlement_method, st.currency
ORDER BY hour DESC, st.settlement_method;

-- Initialize default settlement windows
INSERT INTO settlement_windows (currency, payment_rail, window_start, window_end, same_day_cutoff, timezone) VALUES
    ('USD', 'SWIFT', '14:00', '22:00', '20:00', 'America/New_York'),
    ('EUR', 'SEPA', '08:00', '16:00', '14:00', 'Europe/Frankfurt'),
    ('GBP', 'SWIFT', '08:00', '16:00', '14:00', 'Europe/London'),
    ('AED', 'LocalACH', '08:00', '16:00', '14:00', 'Asia/Dubai'),
    ('INR', 'LocalACH', '09:00', '17:00', '15:00', 'Asia/Kolkata')
ON CONFLICT DO NOTHING;

-- Initialize mock bank configurations
INSERT INTO external_bank_configs (bank_code, bank_name, api_type, is_mock, mock_latency_ms, mock_success_rate) VALUES
    ('MOCK_BANK', 'Mock Bank for MVP', 'Mock', true, 500, 0.95)
ON CONFLICT DO NOTHING;

-- Grant permissions
GRANT SELECT, INSERT, UPDATE ON settlement_transactions TO deltran;
GRANT SELECT, INSERT, UPDATE ON nostro_accounts TO deltran;
GRANT SELECT, INSERT, UPDATE ON vostro_accounts TO deltran;
GRANT SELECT, INSERT, UPDATE, DELETE ON fund_locks TO deltran;
GRANT SELECT, INSERT, UPDATE ON settlement_atomic_operations TO deltran;
GRANT SELECT, INSERT ON settlement_operation_checkpoints TO deltran;
GRANT SELECT, INSERT, UPDATE ON settlement_reconciliation_reports TO deltran;
GRANT SELECT, INSERT ON settlement_account_reconciliations TO deltran;
GRANT SELECT ON settlement_windows TO deltran;
GRANT SELECT ON external_bank_configs TO deltran;
GRANT SELECT, INSERT, UPDATE ON external_transfer_status TO deltran;

-- Audit log entry
INSERT INTO audit_log (entity_type, action, changes)
VALUES ('database', 'MIGRATION_006_SETTLEMENT_ENGINE', '{"message": "Settlement engine schema created successfully"}'::jsonb);

COMMIT;
