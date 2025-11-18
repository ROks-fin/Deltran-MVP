-- DelTran MVP Database Schema
-- Initial schema with all core tables

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ===== BANKS AND PARTICIPANTS =====

CREATE TABLE IF NOT EXISTS banks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bank_code VARCHAR(20) UNIQUE NOT NULL,
    bank_name VARCHAR(255) NOT NULL,
    swift_bic VARCHAR(11),
    country_code VARCHAR(3) NOT NULL,
    region VARCHAR(50),
    status VARCHAR(20) DEFAULT 'ACTIVE',
    onboarded_at TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_banks_code ON banks(bank_code);
CREATE INDEX idx_banks_country ON banks(country_code);

-- ===== CLEARING WINDOWS =====

CREATE TABLE IF NOT EXISTS clearing_windows (
    id BIGSERIAL PRIMARY KEY,
    window_name VARCHAR(100) UNIQUE NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    cutoff_time TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) DEFAULT 'OPEN',
    region VARCHAR(50) DEFAULT 'Global',
    transactions_count INTEGER DEFAULT 0,
    obligations_count INTEGER DEFAULT 0,
    total_gross_value NUMERIC(26,8) DEFAULT 0,
    total_net_value NUMERIC(26,8) DEFAULT 0,
    saved_amount NUMERIC(26,8) DEFAULT 0,
    netting_efficiency NUMERIC(5,2) DEFAULT 0,
    settlement_instructions UUID[],
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    closed_at TIMESTAMPTZ,
    processed_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    grace_period_seconds INTEGER DEFAULT 1800,
    grace_period_started TIMESTAMPTZ
);

CREATE INDEX idx_windows_status ON clearing_windows(status);
CREATE INDEX idx_windows_region ON clearing_windows(region);
CREATE INDEX idx_windows_created ON clearing_windows(created_at DESC);

-- ===== WINDOW EVENTS =====

CREATE TABLE IF NOT EXISTS window_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    event_type VARCHAR(50) NOT NULL,
    old_status VARCHAR(20),
    new_status VARCHAR(20),
    event_data JSONB DEFAULT '{}',
    triggered_by VARCHAR(100),
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_window_events_window ON window_events(window_id);
CREATE INDEX idx_window_events_type ON window_events(event_type);

-- ===== ATOMIC OPERATIONS =====

CREATE TABLE IF NOT EXISTS atomic_operations (
    operation_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    operation_type VARCHAR(50) NOT NULL,
    state VARCHAR(20) DEFAULT 'Pending',
    parent_operation_id UUID REFERENCES atomic_operations(operation_id),
    checkpoints JSONB DEFAULT '{}',
    started_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    rolled_back_at TIMESTAMPTZ,
    error_message TEXT,
    error_code VARCHAR(50),
    rollback_data JSONB,
    rollback_reason TEXT
);

CREATE INDEX idx_atomic_ops_window ON atomic_operations(window_id);
CREATE INDEX idx_atomic_ops_state ON atomic_operations(state);

-- ===== OPERATION CHECKPOINTS =====

CREATE TABLE IF NOT EXISTS operation_checkpoints (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    operation_id UUID NOT NULL REFERENCES atomic_operations(operation_id),
    checkpoint_name VARCHAR(100) NOT NULL,
    checkpoint_order INTEGER NOT NULL,
    checkpoint_data JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_checkpoints_operation ON operation_checkpoints(operation_id);

-- ===== OBLIGATIONS =====

CREATE TABLE IF NOT EXISTS obligations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    window_id BIGINT REFERENCES clearing_windows(id),
    transaction_id UUID,
    payer_id UUID NOT NULL REFERENCES banks(id),
    payee_id UUID NOT NULL REFERENCES banks(id),
    amount NUMERIC(26,8) NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL,
    status VARCHAR(20) DEFAULT 'PENDING',
    obligation_type VARCHAR(50) DEFAULT 'TRANSACTION',
    priority INTEGER DEFAULT 1,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    processed_at TIMESTAMPTZ
);

CREATE INDEX idx_obligations_window ON obligations(window_id);
CREATE INDEX idx_obligations_status ON obligations(status);
CREATE INDEX idx_obligations_payer ON obligations(payer_id);
CREATE INDEX idx_obligations_payee ON obligations(payee_id);
CREATE INDEX idx_obligations_currency ON obligations(currency);

-- ===== NET POSITIONS =====

CREATE TABLE IF NOT EXISTS net_positions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    bank_pair_hash VARCHAR(100) NOT NULL,
    bank_a_id UUID NOT NULL REFERENCES banks(id),
    bank_b_id UUID NOT NULL REFERENCES banks(id),
    currency VARCHAR(3) NOT NULL,
    gross_debit_a_to_b NUMERIC(26,8) DEFAULT 0,
    gross_credit_b_to_a NUMERIC(26,8) DEFAULT 0,
    net_amount NUMERIC(26,8) DEFAULT 0,
    net_direction VARCHAR(20),
    net_payer_id UUID REFERENCES banks(id),
    net_receiver_id UUID REFERENCES banks(id),
    obligations_netted INTEGER DEFAULT 0,
    netting_ratio NUMERIC(5,4) DEFAULT 0,
    amount_saved NUMERIC(26,8) DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_net_pos_window ON net_positions(window_id);
CREATE INDEX idx_net_pos_hash ON net_positions(bank_pair_hash);
CREATE INDEX idx_net_pos_currency ON net_positions(currency);

-- ===== SETTLEMENT INSTRUCTIONS =====

CREATE TABLE IF NOT EXISTS settlement_instructions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    net_position_id UUID REFERENCES net_positions(id),
    payer_bank_id UUID NOT NULL REFERENCES banks(id),
    payee_bank_id UUID NOT NULL REFERENCES banks(id),
    amount NUMERIC(26,8) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    instruction_type VARCHAR(50) DEFAULT 'NET_SETTLEMENT',
    priority INTEGER DEFAULT 1,
    deadline TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) DEFAULT 'PENDING',
    sent_to_settlement_at TIMESTAMPTZ,
    settlement_id UUID,
    instruction_data JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_settlement_instr_window ON settlement_instructions(window_id);
CREATE INDEX idx_settlement_instr_status ON settlement_instructions(status);
CREATE INDEX idx_settlement_instr_deadline ON settlement_instructions(deadline);

-- ===== CLEARING METRICS =====

CREATE TABLE IF NOT EXISTS clearing_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    window_id BIGINT UNIQUE NOT NULL REFERENCES clearing_windows(id),
    processing_started_at TIMESTAMPTZ NOT NULL,
    processing_completed_at TIMESTAMPTZ,
    processing_duration_ms BIGINT,
    obligations_collected INTEGER DEFAULT 0,
    obligations_netted INTEGER DEFAULT 0,
    net_positions_calculated INTEGER DEFAULT 0,
    gross_value NUMERIC(26,8) DEFAULT 0,
    net_value NUMERIC(26,8) DEFAULT 0,
    efficiency_percent NUMERIC(5,2) DEFAULT 0,
    total_saved NUMERIC(26,8) DEFAULT 0,
    instructions_generated INTEGER DEFAULT 0,
    instructions_sent INTEGER DEFAULT 0,
    errors_count INTEGER DEFAULT 0,
    warnings_count INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_metrics_window ON clearing_metrics(window_id);

-- ===== WINDOW LOCKS =====

CREATE TABLE IF NOT EXISTS window_locks (
    window_id BIGINT PRIMARY KEY REFERENCES clearing_windows(id),
    locked_by VARCHAR(100) NOT NULL,
    locked_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    lock_token UUID NOT NULL DEFAULT uuid_generate_v4()
);

CREATE INDEX idx_window_locks_expires ON window_locks(expires_at);
