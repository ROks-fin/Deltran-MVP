-- Settlement Engine Database Schema
-- Manages final settlement transactions, nostro/vostro accounts, and reconciliation

-- ============================================================================
-- SETTLEMENT TRANSACTIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS settlement_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    obligation_id UUID NOT NULL,

    -- Parties
    from_bank VARCHAR(20) NOT NULL,
    to_bank VARCHAR(20) NOT NULL,

    -- Amount details
    amount DECIMAL(20, 2) NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL,

    -- Status tracking
    status VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    priority VARCHAR(10) DEFAULT 'normal',

    -- External references
    external_reference VARCHAR(100),
    bank_confirmation VARCHAR(100),
    settlement_method VARCHAR(20) DEFAULT 'Mock',

    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    validated_at TIMESTAMP,
    executed_at TIMESTAMP,
    completed_at TIMESTAMP,
    failed_at TIMESTAMP,
    rolled_back_at TIMESTAMP,

    -- Error tracking
    error_message TEXT,
    retry_count INT DEFAULT 0,
    last_retry_at TIMESTAMP,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Constraints
    CONSTRAINT fk_obligation FOREIGN KEY (obligation_id)
        REFERENCES obligations(id) ON DELETE RESTRICT,
    CONSTRAINT fk_from_bank FOREIGN KEY (from_bank)
        REFERENCES banks(bank_code) ON DELETE RESTRICT,
    CONSTRAINT fk_to_bank FOREIGN KEY (to_bank)
        REFERENCES banks(bank_code) ON DELETE RESTRICT,
    CONSTRAINT chk_different_banks CHECK (from_bank != to_bank)
);

CREATE INDEX idx_settlements_obligation ON settlement_transactions(obligation_id);
CREATE INDEX idx_settlements_status ON settlement_transactions(status);
CREATE INDEX idx_settlements_created ON settlement_transactions(created_at DESC);
CREATE INDEX idx_settlements_from_bank ON settlement_transactions(from_bank);
CREATE INDEX idx_settlements_to_bank ON settlement_transactions(to_bank);
CREATE INDEX idx_settlements_currency ON settlement_transactions(currency);
CREATE INDEX idx_settlements_retry ON settlement_transactions(status, retry_count)
    WHERE status = 'FAILED';

-- ============================================================================
-- NOSTRO ACCOUNTS (Our accounts at other banks)
-- ============================================================================

CREATE TABLE IF NOT EXISTS nostro_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank VARCHAR(20) NOT NULL,
    account_number VARCHAR(50) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Balances
    ledger_balance DECIMAL(20, 2) NOT NULL DEFAULT 0,
    available_balance DECIMAL(20, 2) NOT NULL DEFAULT 0,
    locked_balance DECIMAL(20, 2) NOT NULL DEFAULT 0,

    -- Metadata
    last_reconciled TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    -- Constraints
    UNIQUE(bank, currency),
    CONSTRAINT fk_nostro_bank FOREIGN KEY (bank)
        REFERENCES banks(bank_code) ON DELETE RESTRICT,
    CONSTRAINT chk_nostro_balances CHECK (
        ledger_balance >= 0 AND
        available_balance >= 0 AND
        locked_balance >= 0 AND
        available_balance + locked_balance = ledger_balance
    )
);

CREATE INDEX idx_nostro_bank ON nostro_accounts(bank);
CREATE INDEX idx_nostro_currency ON nostro_accounts(currency);
CREATE INDEX idx_nostro_active ON nostro_accounts(is_active) WHERE is_active = TRUE;

-- ============================================================================
-- VOSTRO ACCOUNTS (Other banks' accounts with us)
-- ============================================================================

CREATE TABLE IF NOT EXISTS vostro_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank VARCHAR(20) NOT NULL,
    account_number VARCHAR(50) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Balance and limits
    ledger_balance DECIMAL(20, 2) NOT NULL DEFAULT 0,
    credit_limit DECIMAL(20, 2),

    -- Metadata
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    -- Constraints
    UNIQUE(bank, currency),
    CONSTRAINT fk_vostro_bank FOREIGN KEY (bank)
        REFERENCES banks(bank_code) ON DELETE RESTRICT
);

CREATE INDEX idx_vostro_bank ON vostro_accounts(bank);
CREATE INDEX idx_vostro_currency ON vostro_accounts(currency);
CREATE INDEX idx_vostro_active ON vostro_accounts(is_active) WHERE is_active = TRUE;

-- ============================================================================
-- FUND LOCKS
-- ============================================================================

CREATE TABLE IF NOT EXISTS fund_locks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- References
    nostro_account_id UUID NOT NULL,
    settlement_id UUID NOT NULL,

    -- Lock details
    amount DECIMAL(20, 2) NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL,
    bank VARCHAR(20) NOT NULL,

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'active',

    -- Timestamps
    locked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    released_at TIMESTAMP,
    released_by VARCHAR(50),

    -- Constraints
    CONSTRAINT fk_fund_lock_nostro FOREIGN KEY (nostro_account_id)
        REFERENCES nostro_accounts(id) ON DELETE RESTRICT,
    CONSTRAINT fk_fund_lock_settlement FOREIGN KEY (settlement_id)
        REFERENCES settlement_transactions(id) ON DELETE RESTRICT,
    CONSTRAINT chk_lock_expiry CHECK (expires_at > locked_at)
);

CREATE INDEX idx_locks_nostro ON fund_locks(nostro_account_id);
CREATE INDEX idx_locks_settlement ON fund_locks(settlement_id);
CREATE INDEX idx_locks_status ON fund_locks(status);
CREATE INDEX idx_locks_expires ON fund_locks(expires_at) WHERE status = 'active';

-- Trigger to update locked_balance when fund lock is created
CREATE OR REPLACE FUNCTION update_locked_balance_on_lock()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'active' THEN
        UPDATE nostro_accounts
        SET locked_balance = locked_balance + NEW.amount,
            available_balance = available_balance - NEW.amount
        WHERE id = NEW.nostro_account_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_fund_lock_created
    AFTER INSERT ON fund_locks
    FOR EACH ROW
    EXECUTE FUNCTION update_locked_balance_on_lock();

-- Trigger to update locked_balance when fund lock is released
CREATE OR REPLACE FUNCTION update_locked_balance_on_release()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status != OLD.status AND NEW.status IN ('released', 'expired') THEN
        UPDATE nostro_accounts
        SET locked_balance = locked_balance - OLD.amount,
            available_balance = available_balance + OLD.amount
        WHERE id = OLD.nostro_account_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_fund_lock_released
    AFTER UPDATE ON fund_locks
    FOR EACH ROW
    WHEN (OLD.status = 'active' AND NEW.status != 'active')
    EXECUTE FUNCTION update_locked_balance_on_release();

-- ============================================================================
-- ATOMIC OPERATIONS TRACKING
-- ============================================================================

CREATE TABLE IF NOT EXISTS settlement_atomic_operations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    settlement_id UUID NOT NULL,
    operation_type VARCHAR(50) NOT NULL,

    -- State tracking
    state VARCHAR(20) NOT NULL DEFAULT 'InProgress',
    current_checkpoint VARCHAR(50),
    checkpoints JSONB DEFAULT '[]'::jsonb,

    -- Timestamps
    started_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP,
    committed_at TIMESTAMP,
    rolled_back_at TIMESTAMP,

    -- Rollback info
    rollback_reason TEXT,

    -- Constraints
    CONSTRAINT fk_atomic_settlement FOREIGN KEY (settlement_id)
        REFERENCES settlement_transactions(id) ON DELETE CASCADE
);

CREATE INDEX idx_atomic_settlement ON settlement_atomic_operations(settlement_id);
CREATE INDEX idx_atomic_state ON settlement_atomic_operations(state);

-- ============================================================================
-- OPERATION CHECKPOINTS
-- ============================================================================

CREATE TABLE IF NOT EXISTS settlement_operation_checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    operation_id UUID NOT NULL,

    -- Checkpoint details
    checkpoint_name VARCHAR(50) NOT NULL,
    checkpoint_order INT NOT NULL,
    checkpoint_data JSONB DEFAULT '{}'::jsonb,
    rollback_data JSONB DEFAULT '{}'::jsonb,

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'completed',

    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    rolled_back_at TIMESTAMP,

    -- Constraints
    CONSTRAINT fk_checkpoint_operation FOREIGN KEY (operation_id)
        REFERENCES settlement_atomic_operations(id) ON DELETE CASCADE,
    UNIQUE(operation_id, checkpoint_name)
);

CREATE INDEX idx_checkpoints_operation ON settlement_operation_checkpoints(operation_id);
CREATE INDEX idx_checkpoints_order ON settlement_operation_checkpoints(operation_id, checkpoint_order);

-- ============================================================================
-- RECONCILIATION REPORTS
-- ============================================================================

CREATE TABLE IF NOT EXISTS reconciliation_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_date DATE NOT NULL,

    -- Statistics
    total_accounts INT NOT NULL,
    balanced_accounts INT NOT NULL,
    discrepancy_accounts INT NOT NULL,
    total_discrepancy DECIMAL(20, 2),

    -- Details
    details JSONB,

    -- Timestamps
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    -- Constraints
    UNIQUE(report_date)
);

CREATE INDEX idx_reconciliation_date ON reconciliation_reports(report_date DESC);

-- ============================================================================
-- COMPENSATION TRANSACTIONS
-- ============================================================================

CREATE TABLE IF NOT EXISTS compensation_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_settlement_id UUID NOT NULL,
    compensation_settlement_id UUID NOT NULL,

    reason TEXT NOT NULL,

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    -- Constraints
    CONSTRAINT fk_compensation_original FOREIGN KEY (original_settlement_id)
        REFERENCES settlement_transactions(id) ON DELETE RESTRICT,
    CONSTRAINT fk_compensation_settlement FOREIGN KEY (compensation_settlement_id)
        REFERENCES settlement_transactions(id) ON DELETE RESTRICT
);

CREATE INDEX idx_compensation_original ON compensation_transactions(original_settlement_id);
CREATE INDEX idx_compensation_settlement ON compensation_transactions(compensation_settlement_id);

-- ============================================================================
-- SETTLEMENT WINDOWS
-- ============================================================================

CREATE TABLE IF NOT EXISTS settlement_windows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    currency VARCHAR(3) NOT NULL,

    -- Time windows
    window_start TIME NOT NULL,
    window_end TIME NOT NULL,
    days_of_week INT[] DEFAULT ARRAY[1,2,3,4,5], -- Mon-Fri

    -- Status
    is_active BOOLEAN DEFAULT TRUE,

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

    -- Constraints
    UNIQUE(currency, window_start),
    CONSTRAINT chk_window_times CHECK (window_end > window_start)
);

CREATE INDEX idx_settlement_windows_currency ON settlement_windows(currency);
CREATE INDEX idx_settlement_windows_active ON settlement_windows(is_active) WHERE is_active = TRUE;

-- Initialize settlement windows for major currencies
INSERT INTO settlement_windows (currency, window_start, window_end) VALUES
    ('USD', '14:00:00', '22:00:00'),  -- US business hours (UTC)
    ('EUR', '08:00:00', '16:00:00'),  -- European business hours
    ('GBP', '08:00:00', '16:00:00'),  -- UK business hours
    ('AED', '08:00:00', '16:00:00'),  -- UAE business hours
    ('INR', '09:00:00', '17:00:00')   -- India business hours
ON CONFLICT (currency, window_start) DO NOTHING;

-- ============================================================================
-- VIEWS FOR REPORTING
-- ============================================================================

-- Settlement summary view
CREATE OR REPLACE VIEW v_settlement_summary AS
SELECT
    DATE(created_at) as settlement_date,
    currency,
    status,
    COUNT(*) as count,
    SUM(amount) as total_amount,
    AVG(amount) as avg_amount,
    AVG(EXTRACT(EPOCH FROM (completed_at - created_at))) as avg_duration_seconds
FROM settlement_transactions
WHERE created_at > NOW() - INTERVAL '30 days'
GROUP BY DATE(created_at), currency, status;

-- Account balance view
CREATE OR REPLACE VIEW v_account_balances AS
SELECT
    'nostro' as account_type,
    bank,
    currency,
    ledger_balance,
    available_balance as available,
    locked_balance as locked,
    is_active,
    last_reconciled
FROM nostro_accounts
UNION ALL
SELECT
    'vostro' as account_type,
    bank,
    currency,
    ledger_balance,
    ledger_balance as available,
    0 as locked,
    is_active,
    NULL as last_reconciled
FROM vostro_accounts;

-- ============================================================================
-- SAMPLE DATA FOR TESTING
-- ============================================================================

-- Insert sample nostro accounts
INSERT INTO nostro_accounts (bank, account_number, currency, ledger_balance, available_balance, locked_balance) VALUES
    ('BANK001', 'NOSTRO-USD-001', 'USD', 10000000.00, 10000000.00, 0.00),
    ('BANK001', 'NOSTRO-EUR-001', 'EUR', 5000000.00, 5000000.00, 0.00),
    ('BANK002', 'NOSTRO-USD-002', 'USD', 8000000.00, 8000000.00, 0.00),
    ('BANK002', 'NOSTRO-GBP-002', 'GBP', 6000000.00, 6000000.00, 0.00)
ON CONFLICT (bank, currency) DO NOTHING;

-- Insert sample vostro accounts
INSERT INTO vostro_accounts (bank, account_number, currency, ledger_balance, credit_limit) VALUES
    ('BANK003', 'VOSTRO-USD-003', 'USD', 0.00, 5000000.00),
    ('BANK003', 'VOSTRO-EUR-003', 'EUR', 0.00, 3000000.00),
    ('BANK004', 'VOSTRO-USD-004', 'USD', 0.00, 7000000.00)
ON CONFLICT (bank, currency) DO NOTHING;

-- ============================================================================
-- GRANTS
-- ============================================================================

GRANT SELECT, INSERT, UPDATE ON settlement_transactions TO deltran;
GRANT SELECT, INSERT, UPDATE ON nostro_accounts TO deltran;
GRANT SELECT, INSERT, UPDATE ON vostro_accounts TO deltran;
GRANT SELECT, INSERT, UPDATE, DELETE ON fund_locks TO deltran;
GRANT SELECT, INSERT, UPDATE ON settlement_atomic_operations TO deltran;
GRANT SELECT, INSERT, UPDATE ON settlement_operation_checkpoints TO deltran;
GRANT SELECT, INSERT ON reconciliation_reports TO deltran;
GRANT SELECT, INSERT ON compensation_transactions TO deltran;
GRANT SELECT ON settlement_windows TO deltran;
GRANT SELECT ON v_settlement_summary TO deltran;
GRANT SELECT ON v_account_balances TO deltran;

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE settlement_transactions IS 'Final settlement transactions between banks';
COMMENT ON TABLE nostro_accounts IS 'Our accounts held at other correspondent banks';
COMMENT ON TABLE vostro_accounts IS 'Other banks accounts held with us';
COMMENT ON TABLE fund_locks IS 'Active fund locks to prevent double-spending';
COMMENT ON TABLE settlement_atomic_operations IS 'Atomic operation tracking for rollback';
COMMENT ON TABLE settlement_operation_checkpoints IS 'Checkpoints within atomic operations';
COMMENT ON TABLE reconciliation_reports IS 'Daily reconciliation reports';
COMMENT ON TABLE compensation_transactions IS 'Reversal/compensation transactions';
COMMENT ON TABLE settlement_windows IS 'Operating hours for settlements by currency';

COMMENT ON COLUMN settlement_transactions.external_reference IS 'Reference from external bank system';
COMMENT ON COLUMN settlement_transactions.bank_confirmation IS 'Confirmation code from bank';
COMMENT ON COLUMN nostro_accounts.ledger_balance IS 'Total balance (available + locked)';
COMMENT ON COLUMN nostro_accounts.available_balance IS 'Balance available for new settlements';
COMMENT ON COLUMN nostro_accounts.locked_balance IS 'Balance locked in pending settlements';
