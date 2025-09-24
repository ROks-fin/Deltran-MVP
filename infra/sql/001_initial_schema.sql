-- DelTran Rail MVP - Initial Database Schema
-- Version: 001
-- Description: Core tables for payments, settlement, risk, and compliance

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Schema migrations tracking
CREATE TABLE IF NOT EXISTS schema_migrations (
    version VARCHAR(255) PRIMARY KEY,
    applied_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Core payments table
CREATE TABLE payments (
    transaction_id UUID PRIMARY KEY,
    uetr UUID NOT NULL UNIQUE,
    amount DECIMAL(20,2) NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL,
    debtor_account VARCHAR(34) NOT NULL,
    creditor_account VARCHAR(34) NOT NULL,
    debtor_bic VARCHAR(11),
    creditor_bic VARCHAR(11),
    payment_purpose VARCHAR(50) DEFAULT 'TRADE',
    settlement_method VARCHAR(20) DEFAULT 'PVP',
    status VARCHAR(20) DEFAULT 'INITIATED',
    current_step VARCHAR(50),
    settlement_batch_id UUID,
    estimated_completion TIMESTAMP WITH TIME ZONE,
    trace_id VARCHAR(255),
    idempotency_key VARCHAR(255) UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_status CHECK (status IN (
        'INITIATED', 'VALIDATED', 'SCREENED', 'APPROVED',
        'REJECTED', 'SETTLED', 'COMPLETED', 'FAILED', 'CANCELLED'
    )),
    CONSTRAINT valid_settlement_method CHECK (settlement_method IN (
        'INSTANT', 'PVP', 'NETTING', 'CORRESPONDENT'
    ))
);

-- Settlement batches
CREATE TABLE settlement_batches (
    batch_id UUID PRIMARY KEY,
    window VARCHAR(20) NOT NULL,
    total_transactions INTEGER NOT NULL DEFAULT 0,
    total_amount DECIMAL(20,2) NOT NULL DEFAULT 0,
    net_positions JSONB,
    status VARCHAR(20) DEFAULT 'OPEN',
    closed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_window CHECK (window IN ('intraday', 'EOD')),
    CONSTRAINT valid_status CHECK (status IN ('OPEN', 'CLOSED', 'SETTLED'))
);

-- Settlement details
CREATE TABLE settlement_details (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID REFERENCES payments(transaction_id),
    settlement_date DATE NOT NULL,
    settlement_currency VARCHAR(3),
    exchange_rate DECIMAL(10,6),
    charges JSONB,
    netting_batch_id UUID REFERENCES settlement_batches(batch_id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Risk configurations
CREATE TABLE risk_config (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    mode VARCHAR(20) NOT NULL,
    reason TEXT,
    changed_by VARCHAR(100),
    auto_escalation BOOLEAN DEFAULT true,
    is_active BOOLEAN DEFAULT false,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_mode CHECK (mode IN ('Low', 'Medium', 'High'))
);

-- Risk assessments
CREATE TABLE risk_assessments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID REFERENCES payments(transaction_id),
    risk_score DECIMAL(5,2) NOT NULL CHECK (risk_score >= 0 AND risk_score <= 100),
    risk_factors JSONB,
    recommended_action VARCHAR(50),
    assessed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_action CHECK (recommended_action IN (
        'APPROVE', 'ENHANCED_MONITORING', 'MANUAL_REVIEW', 'REJECT'
    ))
);

-- Compliance checks
CREATE TABLE compliance_checks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID REFERENCES payments(transaction_id),
    check_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL,
    result JSONB,
    checked_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_check_type CHECK (check_type IN (
        'SANCTIONS', 'PEP', 'TRAVEL_RULE', 'KYC'
    )),
    CONSTRAINT valid_status CHECK (status IN ('PASS', 'FAIL', 'REVIEW', 'PENDING'))
);

-- Travel Rule data
CREATE TABLE travel_rule_data (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID REFERENCES payments(transaction_id),
    threshold_met BOOLEAN NOT NULL,
    threshold_amount DECIMAL(20,2),
    fields_complete BOOLEAN DEFAULT false,
    originator_info JSONB,
    beneficiary_info JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Ledger blocks
CREATE TABLE ledger_blocks (
    block_number BIGINT PRIMARY KEY,
    block_hash VARCHAR(64) UNIQUE NOT NULL,
    parent_hash VARCHAR(64),
    merkle_root VARCHAR(64),
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    validator_count INTEGER DEFAULT 0,
    transaction_count INTEGER DEFAULT 0,
    signatures JSONB,
    finalized BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Ledger transactions (events)
CREATE TABLE ledger_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID REFERENCES payments(transaction_id),
    block_number BIGINT REFERENCES ledger_blocks(block_number),
    event_type VARCHAR(50) NOT NULL,
    event_data JSONB NOT NULL,
    transaction_index INTEGER,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_event_type CHECK (event_type IN (
        'PAYMENT_INITIATED', 'PAYMENT_VALIDATED', 'PAYMENT_SETTLED',
        'BATCH_CLOSED', 'RISK_ASSESSED', 'COMPLIANCE_CHECKED'
    ))
);

-- Ledger proofs
CREATE TABLE ledger_proofs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID REFERENCES payments(transaction_id),
    block_hash VARCHAR(64),
    block_number BIGINT,
    transaction_index INTEGER,
    merkle_proof JSONB,
    validator_signatures JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Liquidity quotes (for caching and audit)
CREATE TABLE liquidity_quotes (
    quote_id UUID PRIMARY KEY,
    from_currency VARCHAR(3) NOT NULL,
    to_currency VARCHAR(3) NOT NULL,
    amount DECIMAL(20,2) NOT NULL,
    mid_rate DECIMAL(12,8) NOT NULL,
    applied_rate DECIMAL(12,8) NOT NULL,
    spread DECIMAL(8,6) NOT NULL,
    source VARCHAR(100),
    ttl_seconds INTEGER DEFAULT 30,
    expires_at TIMESTAMP WITH TIME ZONE,
    utility_score DECIMAL(3,2),
    latency_ms INTEGER,
    executed BOOLEAN DEFAULT false,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Reports storage
CREATE TABLE reports (
    report_id UUID PRIMARY KEY,
    report_type VARCHAR(50) NOT NULL,
    data JSONB NOT NULL,
    generated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    valid_until TIMESTAMP WITH TIME ZONE,

    CONSTRAINT valid_report_type CHECK (report_type IN (
        'PROOF_OF_RESERVES', 'PROOF_OF_SETTLEMENT', 'COMPLIANCE', 'TRANSACTION'
    ))
);

-- Audit log
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID NOT NULL,
    action VARCHAR(50) NOT NULL,
    actor VARCHAR(100),
    old_values JSONB,
    new_values JSONB,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    trace_id VARCHAR(255),

    CONSTRAINT valid_entity_type CHECK (entity_type IN (
        'PAYMENT', 'SETTLEMENT_BATCH', 'RISK_CONFIG', 'COMPLIANCE_CHECK'
    ))
);

-- Create indexes for performance
CREATE INDEX idx_payments_status ON payments(status);
CREATE INDEX idx_payments_created_at ON payments(created_at);
CREATE INDEX idx_payments_currency ON payments(currency);
CREATE INDEX idx_payments_debtor_account ON payments(debtor_account);
CREATE INDEX idx_payments_creditor_account ON payments(creditor_account);
CREATE INDEX idx_payments_settlement_batch ON payments(settlement_batch_id);
CREATE INDEX idx_payments_trace_id ON payments(trace_id);

CREATE INDEX idx_settlement_batches_window ON settlement_batches(window);
CREATE INDEX idx_settlement_batches_closed_at ON settlement_batches(closed_at);
CREATE INDEX idx_settlement_batches_status ON settlement_batches(status);

CREATE INDEX idx_risk_assessments_transaction_id ON risk_assessments(transaction_id);
CREATE INDEX idx_risk_assessments_score ON risk_assessments(risk_score);
CREATE INDEX idx_risk_assessments_assessed_at ON risk_assessments(assessed_at);

CREATE INDEX idx_compliance_checks_transaction_id ON compliance_checks(transaction_id);
CREATE INDEX idx_compliance_checks_type ON compliance_checks(check_type);
CREATE INDEX idx_compliance_checks_status ON compliance_checks(status);

CREATE INDEX idx_ledger_blocks_hash ON ledger_blocks(block_hash);
CREATE INDEX idx_ledger_blocks_timestamp ON ledger_blocks(timestamp);
CREATE INDEX idx_ledger_transactions_transaction_id ON ledger_transactions(transaction_id);
CREATE INDEX idx_ledger_transactions_block_number ON ledger_transactions(block_number);

CREATE INDEX idx_liquidity_quotes_currencies ON liquidity_quotes(from_currency, to_currency);
CREATE INDEX idx_liquidity_quotes_expires_at ON liquidity_quotes(expires_at);

CREATE INDEX idx_reports_type ON reports(report_type);
CREATE INDEX idx_reports_generated_at ON reports(generated_at);

CREATE INDEX idx_audit_log_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_log_trace_id ON audit_log(trace_id);

-- Insert initial data
INSERT INTO risk_config (mode, reason, changed_by, auto_escalation, is_active)
VALUES ('Medium', 'Initial system setup', 'system', true, true);

-- Insert initial schema migration record
INSERT INTO schema_migrations (version) VALUES ('001_initial_schema');