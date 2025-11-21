-- ============================================================================
-- CLEARING ENGINE SCHEMA
-- Version: 1.0
-- Description: Tables for clearing window management and orchestration
-- ============================================================================

-- Clearing windows with detailed tracking
CREATE TABLE IF NOT EXISTS clearing_windows (
    id BIGINT PRIMARY KEY, -- Unix timestamp as ID
    window_name VARCHAR(50) NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    cutoff_time TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'Scheduled',
    region VARCHAR(20) NOT NULL DEFAULT 'Global',

    -- Counters
    transactions_count INTEGER DEFAULT 0,
    obligations_count INTEGER DEFAULT 0,

    -- Financial metrics
    total_gross_value DECIMAL(20,2) DEFAULT 0,
    total_net_value DECIMAL(20,2) DEFAULT 0,
    saved_amount DECIMAL(20,2) DEFAULT 0,
    netting_efficiency DECIMAL(5,2) DEFAULT 0,

    -- Settlement instructions
    settlement_instructions UUID[],

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at TIMESTAMPTZ,
    processed_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,

    -- Grace period tracking
    grace_period_seconds INTEGER DEFAULT 30,
    grace_period_started TIMESTAMPTZ,

    CONSTRAINT valid_window_times CHECK (end_time > start_time),
    CONSTRAINT valid_cutoff CHECK (cutoff_time <= end_time)
);

CREATE INDEX idx_clearing_windows_status ON clearing_windows(status);
CREATE INDEX idx_clearing_windows_region ON clearing_windows(region);
CREATE INDEX idx_clearing_windows_start ON clearing_windows(start_time DESC);
CREATE INDEX idx_clearing_windows_created ON clearing_windows(created_at DESC);

-- Window status history for audit trail
CREATE TABLE IF NOT EXISTS clearing_window_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    event_type VARCHAR(50) NOT NULL,
    old_status VARCHAR(20),
    new_status VARCHAR(20),
    event_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    triggered_by VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_window_events_window (window_id, created_at DESC),
    INDEX idx_window_events_type (event_type)
);

-- Atomic operations log for clearing operations
CREATE TABLE IF NOT EXISTS clearing_atomic_operations (
    operation_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    operation_type VARCHAR(50) NOT NULL,
    state VARCHAR(20) NOT NULL DEFAULT 'Pending',
    parent_operation_id UUID REFERENCES clearing_atomic_operations(operation_id),

    -- Checkpoints
    checkpoints JSONB DEFAULT '[]'::jsonb,

    -- Timestamps
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    rolled_back_at TIMESTAMPTZ,

    -- Error tracking
    error_message TEXT,
    error_code VARCHAR(50),
    rollback_data JSONB,
    rollback_reason TEXT,

    INDEX idx_atomic_window (window_id),
    INDEX idx_atomic_state (state),
    INDEX idx_atomic_type (operation_type)
);

-- Operation checkpoints for granular rollback
CREATE TABLE IF NOT EXISTS clearing_operation_checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    operation_id UUID NOT NULL REFERENCES clearing_atomic_operations(operation_id) ON DELETE CASCADE,
    checkpoint_name VARCHAR(100) NOT NULL,
    checkpoint_order INTEGER NOT NULL,
    checkpoint_data JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(operation_id, checkpoint_order),
    INDEX idx_checkpoints_operation (operation_id, checkpoint_order)
);

-- Window locks to prevent concurrent processing
CREATE TABLE IF NOT EXISTS clearing_window_locks (
    window_id BIGINT PRIMARY KEY REFERENCES clearing_windows(id),
    locked_by VARCHAR(100) NOT NULL,
    locked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    lock_token UUID NOT NULL DEFAULT gen_random_uuid(),

    CONSTRAINT valid_lock_expiry CHECK (expires_at > locked_at)
);

CREATE INDEX idx_window_locks_expires ON clearing_window_locks(expires_at);

-- Net positions calculated during clearing
CREATE TABLE IF NOT EXISTS clearing_net_positions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    bank_pair_hash VARCHAR(64) NOT NULL, -- SHA256(bank_a_id || bank_b_id)
    bank_a_id UUID NOT NULL REFERENCES banks(id),
    bank_b_id UUID NOT NULL REFERENCES banks(id),
    currency VARCHAR(3) NOT NULL,

    -- Gross amounts
    gross_debit_a_to_b DECIMAL(20,2) NOT NULL DEFAULT 0,
    gross_credit_b_to_a DECIMAL(20,2) NOT NULL DEFAULT 0,

    -- Net calculation
    net_amount DECIMAL(20,2) NOT NULL,
    net_direction VARCHAR(10) NOT NULL, -- Debit, Credit, Neutral
    net_payer_id UUID REFERENCES banks(id),
    net_receiver_id UUID REFERENCES banks(id),

    -- Statistics
    obligations_netted INTEGER NOT NULL DEFAULT 0,
    netting_ratio DECIMAL(5,2) NOT NULL DEFAULT 0,
    amount_saved DECIMAL(20,2) NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(window_id, bank_pair_hash),
    INDEX idx_net_positions_window (window_id),
    INDEX idx_net_positions_banks (bank_a_id, bank_b_id),
    INDEX idx_net_positions_currency (currency)
);

-- Settlement instructions generated by clearing
CREATE TABLE IF NOT EXISTS clearing_settlement_instructions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    net_position_id UUID REFERENCES clearing_net_positions(id),

    -- Parties
    payer_bank_id UUID NOT NULL REFERENCES banks(id),
    payee_bank_id UUID NOT NULL REFERENCES banks(id),

    -- Amount
    amount DECIMAL(20,2) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Instruction details
    instruction_type VARCHAR(30) NOT NULL DEFAULT 'NetSettlement',
    priority INTEGER DEFAULT 5, -- 1-10 scale
    deadline TIMESTAMPTZ NOT NULL,

    -- Status tracking
    status VARCHAR(20) NOT NULL DEFAULT 'Generated',
    sent_to_settlement_at TIMESTAMPTZ,
    settlement_id UUID, -- Reference to settlement engine

    -- Metadata
    instruction_data JSONB NOT NULL DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT positive_amount CHECK (amount > 0),
    CONSTRAINT valid_priority CHECK (priority BETWEEN 1 AND 10),
    INDEX idx_settlement_instructions_window (window_id),
    INDEX idx_settlement_instructions_status (status),
    INDEX idx_settlement_instructions_deadline (deadline)
);

-- Clearing metrics for monitoring
CREATE TABLE IF NOT EXISTS clearing_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),

    -- Processing metrics
    processing_started_at TIMESTAMPTZ NOT NULL,
    processing_completed_at TIMESTAMPTZ,
    processing_duration_ms BIGINT,

    -- Netting metrics
    obligations_collected INTEGER NOT NULL DEFAULT 0,
    obligations_netted INTEGER NOT NULL DEFAULT 0,
    net_positions_calculated INTEGER NOT NULL DEFAULT 0,

    -- Financial metrics
    gross_value DECIMAL(20,2) NOT NULL DEFAULT 0,
    net_value DECIMAL(20,2) NOT NULL DEFAULT 0,
    efficiency_percent DECIMAL(5,2) NOT NULL DEFAULT 0,
    total_saved DECIMAL(20,2) NOT NULL DEFAULT 0,

    -- Instruction metrics
    instructions_generated INTEGER NOT NULL DEFAULT 0,
    instructions_sent INTEGER NOT NULL DEFAULT 0,

    -- Error metrics
    errors_count INTEGER NOT NULL DEFAULT 0,
    warnings_count INTEGER NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(window_id)
);

-- Functions for automatic updates
CREATE OR REPLACE FUNCTION update_clearing_window_metrics()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'Completed' THEN
        UPDATE clearing_windows
        SET
            netting_efficiency = (
                CASE
                    WHEN NEW.total_gross_value > 0
                    THEN ((NEW.total_gross_value - NEW.total_net_value) / NEW.total_gross_value * 100)::DECIMAL(5,2)
                    ELSE 0
                END
            ),
            saved_amount = NEW.total_gross_value - NEW.total_net_value
        WHERE id = NEW.id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_clearing_metrics
    AFTER UPDATE OF status ON clearing_windows
    FOR EACH ROW
    WHEN (NEW.status = 'Completed')
    EXECUTE FUNCTION update_clearing_window_metrics();

-- Function to automatically create window events
CREATE OR REPLACE FUNCTION log_clearing_window_status_change()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status IS DISTINCT FROM OLD.status THEN
        INSERT INTO clearing_window_events (
            window_id,
            event_type,
            old_status,
            new_status,
            event_data,
            triggered_by
        ) VALUES (
            NEW.id,
            'status_changed',
            OLD.status,
            NEW.status,
            jsonb_build_object(
                'window_name', NEW.window_name,
                'region', NEW.region,
                'transactions_count', NEW.transactions_count
            ),
            current_user
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_log_clearing_status_changes
    AFTER UPDATE OF status ON clearing_windows
    FOR EACH ROW
    EXECUTE FUNCTION log_clearing_window_status_change();

-- Views for monitoring

-- Current window status
CREATE OR REPLACE VIEW v_clearing_current_windows AS
SELECT
    cw.id,
    cw.window_name,
    cw.status,
    cw.region,
    cw.start_time,
    cw.end_time,
    cw.cutoff_time,
    cw.transactions_count,
    cw.obligations_count,
    cw.total_gross_value,
    cw.total_net_value,
    cw.netting_efficiency,
    cw.saved_amount,
    ARRAY_LENGTH(cw.settlement_instructions, 1) as instructions_count,
    cw.created_at,
    NOW() - cw.start_time as window_age
FROM clearing_windows cw
WHERE cw.status IN ('Open', 'Closing', 'Processing', 'Settling')
ORDER BY cw.start_time DESC;

-- Clearing efficiency report
CREATE OR REPLACE VIEW v_clearing_efficiency_report AS
SELECT
    DATE_TRUNC('day', cw.start_time) as date,
    cw.region,
    COUNT(*) as windows_completed,
    SUM(cw.obligations_count) as total_obligations,
    SUM(cw.total_gross_value) as total_gross_value,
    SUM(cw.total_net_value) as total_net_value,
    SUM(cw.saved_amount) as total_saved,
    AVG(cw.netting_efficiency) as avg_efficiency,
    MAX(cw.netting_efficiency) as max_efficiency,
    MIN(cw.netting_efficiency) as min_efficiency
FROM clearing_windows cw
WHERE cw.status = 'Completed'
    AND cw.start_time > NOW() - INTERVAL '30 days'
GROUP BY DATE_TRUNC('day', cw.start_time), cw.region
ORDER BY date DESC, region;

-- Atomic operations summary
CREATE OR REPLACE VIEW v_clearing_atomic_operations_summary AS
SELECT
    cao.operation_type,
    cao.state,
    COUNT(*) as count,
    AVG(EXTRACT(EPOCH FROM (COALESCE(cao.completed_at, NOW()) - cao.started_at))) as avg_duration_seconds,
    COUNT(CASE WHEN cao.state = 'RolledBack' THEN 1 END) as rollback_count
FROM clearing_atomic_operations cao
WHERE cao.started_at > NOW() - INTERVAL '24 hours'
GROUP BY cao.operation_type, cao.state
ORDER BY cao.operation_type, cao.state;

-- Grant permissions
GRANT SELECT, INSERT, UPDATE, DELETE ON clearing_windows TO deltran;
GRANT SELECT, INSERT ON clearing_window_events TO deltran;
GRANT SELECT, INSERT, UPDATE ON clearing_atomic_operations TO deltran;
GRANT SELECT, INSERT ON clearing_operation_checkpoints TO deltran;
GRANT SELECT, INSERT, DELETE ON clearing_window_locks TO deltran;
GRANT SELECT, INSERT, UPDATE ON clearing_net_positions TO deltran;
GRANT SELECT, INSERT, UPDATE ON clearing_settlement_instructions TO deltran;
GRANT SELECT, INSERT ON clearing_metrics TO deltran;

-- Audit log entry
INSERT INTO audit_log (entity_type, action, changes)
VALUES ('database', 'MIGRATION_005_CLEARING_ENGINE', '{"message": "Clearing engine schema created successfully"}'::jsonb);

COMMIT;
