-- EMI Accounts Schema for 1:1 Backing
-- Supports multi-bank, multi-currency segregated accounts

-- ===== EMI ACCOUNTS =====

CREATE TABLE IF NOT EXISTS emi_accounts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    bank_id UUID NOT NULL REFERENCES banks(id),
    account_number VARCHAR(50) NOT NULL,
    iban VARCHAR(34),
    swift_bic VARCHAR(11),
    currency VARCHAR(3) NOT NULL,
    country_code VARCHAR(3) NOT NULL,

    -- Account type: client_funds, settlement, fee, reserve_buffer
    account_type VARCHAR(20) NOT NULL DEFAULT 'client_funds',

    -- Balances (all in account currency)
    ledger_balance NUMERIC(26,8) DEFAULT 0 CHECK (ledger_balance >= 0),
    bank_reported_balance NUMERIC(26,8) DEFAULT 0,
    reserved_balance NUMERIC(26,8) DEFAULT 0 CHECK (reserved_balance >= 0),
    available_balance NUMERIC(26,8) GENERATED ALWAYS AS (ledger_balance - reserved_balance) STORED,

    -- Reconciliation tracking
    last_reconciliation_at TIMESTAMPTZ,
    reconciliation_status VARCHAR(20) DEFAULT 'PENDING',
    reconciliation_source VARCHAR(50), -- 'camt.053', 'camt.054', 'api_polling'
    reconciliation_difference NUMERIC(26,8) DEFAULT 0,

    -- Metadata
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(bank_id, account_number, currency)
);

CREATE INDEX idx_emi_accounts_bank ON emi_accounts(bank_id);
CREATE INDEX idx_emi_accounts_type ON emi_accounts(account_type);
CREATE INDEX idx_emi_accounts_currency ON emi_accounts(currency);
CREATE INDEX idx_emi_accounts_country ON emi_accounts(country_code);
CREATE INDEX idx_emi_accounts_recon_status ON emi_accounts(reconciliation_status);

-- ===== EMI ACCOUNT SNAPSHOTS =====
-- EOD snapshots for regulatory compliance

CREATE TABLE IF NOT EXISTS emi_account_snapshots (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_id UUID NOT NULL REFERENCES emi_accounts(id),
    snapshot_date DATE NOT NULL,
    snapshot_time TIMESTAMPTZ NOT NULL,

    ledger_balance NUMERIC(26,8) NOT NULL,
    bank_reported_balance NUMERIC(26,8) NOT NULL,
    reserved_balance NUMERIC(26,8) NOT NULL,
    available_balance NUMERIC(26,8) NOT NULL,

    difference NUMERIC(26,8) DEFAULT 0,
    reconciled BOOLEAN DEFAULT FALSE,

    statement_reference VARCHAR(100), -- camt.053 reference
    snapshot_data JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(account_id, snapshot_date)
);

CREATE INDEX idx_snapshots_account ON emi_account_snapshots(account_id);
CREATE INDEX idx_snapshots_date ON emi_account_snapshots(snapshot_date DESC);
CREATE INDEX idx_snapshots_reconciled ON emi_account_snapshots(reconciled);

-- ===== EMI TRANSACTIONS =====
-- All movements in/out of EMI accounts

CREATE TABLE IF NOT EXISTS emi_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_id UUID NOT NULL REFERENCES emi_accounts(id),

    transaction_type VARCHAR(30) NOT NULL, -- 'funding', 'settlement', 'fee', 'reversal'
    direction VARCHAR(10) NOT NULL, -- 'CREDIT', 'DEBIT'
    amount NUMERIC(26,8) NOT NULL CHECK (amount > 0),

    -- Balances after this transaction
    balance_before NUMERIC(26,8) NOT NULL,
    balance_after NUMERIC(26,8) NOT NULL,

    -- References
    related_transaction_id UUID, -- Original DelTran transaction
    related_settlement_id UUID,  -- Settlement instruction
    bank_reference VARCHAR(100), -- Bank's transaction reference
    uetr VARCHAR(36),           -- ISO 20022 UETR

    -- ISO 20022 message tracking
    iso_message_type VARCHAR(20), -- 'pacs.008', 'camt.054', etc.
    iso_message_id VARCHAR(35),

    -- Status
    status VARCHAR(20) DEFAULT 'PENDING', -- 'PENDING', 'CONFIRMED', 'FAILED'
    confirmed_at TIMESTAMPTZ,

    -- Metadata
    description TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_emi_txn_account ON emi_transactions(account_id);
CREATE INDEX idx_emi_txn_type ON emi_transactions(transaction_type);
CREATE INDEX idx_emi_txn_status ON emi_transactions(status);
CREATE INDEX idx_emi_txn_related ON emi_transactions(related_transaction_id);
CREATE INDEX idx_emi_txn_uetr ON emi_transactions(uetr);
CREATE INDEX idx_emi_txn_created ON emi_transactions(created_at DESC);

-- ===== RECONCILIATION DISCREPANCIES =====
-- Track all reconciliation issues

CREATE TABLE IF NOT EXISTS reconciliation_discrepancies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_id UUID NOT NULL REFERENCES emi_accounts(id),

    discrepancy_type VARCHAR(30) NOT NULL, -- 'BALANCE_MISMATCH', 'MISSING_TXN', 'DUPLICATE_TXN'
    detected_at TIMESTAMPTZ DEFAULT NOW(),

    expected_value NUMERIC(26,8),
    actual_value NUMERIC(26,8),
    difference NUMERIC(26,8),

    -- Threshold information
    threshold_type VARCHAR(20), -- 'PERCENTAGE', 'ABSOLUTE'
    threshold_value NUMERIC(10,4),
    threshold_exceeded BOOLEAN DEFAULT FALSE,

    -- Resolution
    status VARCHAR(20) DEFAULT 'OPEN', -- 'OPEN', 'INVESTIGATING', 'RESOLVED', 'ESCALATED'
    resolved_at TIMESTAMPTZ,
    resolution_notes TEXT,

    -- Source information
    source_system VARCHAR(50), -- 'CAMT_053', 'CAMT_054', 'API_POLL', 'MANUAL'
    source_reference VARCHAR(100),

    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_discrepancies_account ON reconciliation_discrepancies(account_id);
CREATE INDEX idx_discrepancies_status ON reconciliation_discrepancies(status);
CREATE INDEX idx_discrepancies_detected ON reconciliation_discrepancies(detected_at DESC);
CREATE INDEX idx_discrepancies_threshold ON reconciliation_discrepancies(threshold_exceeded);

-- ===== RESERVE BUFFER CALCULATIONS =====
-- Track reserve buffer requirements

CREATE TABLE IF NOT EXISTS reserve_buffer_calculations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    account_id UUID NOT NULL REFERENCES emi_accounts(id),
    calculation_date DATE NOT NULL,

    -- FX volatility metrics
    fx_volatility_30d NUMERIC(10,6),
    fx_volatility_90d NUMERIC(10,6),

    -- Volume metrics
    avg_daily_volume NUMERIC(26,8),
    max_daily_volume NUMERIC(26,8),

    -- Buffer calculation
    required_buffer NUMERIC(26,8) NOT NULL,
    current_buffer NUMERIC(26,8) NOT NULL,
    buffer_adequacy NUMERIC(5,2), -- percentage

    -- Replenishment
    replenishment_needed BOOLEAN DEFAULT FALSE,
    replenishment_amount NUMERIC(26,8) DEFAULT 0,
    replenished_at TIMESTAMPTZ,

    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),

    UNIQUE(account_id, calculation_date)
);

CREATE INDEX idx_buffer_calc_account ON reserve_buffer_calculations(account_id);
CREATE INDEX idx_buffer_calc_date ON reserve_buffer_calculations(calculation_date DESC);
CREATE INDEX idx_buffer_calc_replenishment ON reserve_buffer_calculations(replenishment_needed);

-- ===== FUNCTIONS FOR RECONCILIATION =====

-- Function to update EMI account balance
CREATE OR REPLACE FUNCTION update_emi_account_balance(
    p_account_id UUID,
    p_amount NUMERIC(26,8),
    p_transaction_type VARCHAR(30)
)
RETURNS VOID AS $$
BEGIN
    UPDATE emi_accounts
    SET
        ledger_balance = ledger_balance + p_amount,
        updated_at = NOW()
    WHERE id = p_account_id;
END;
$$ LANGUAGE plpgsql;

-- Function to check reconciliation threshold
CREATE OR REPLACE FUNCTION check_reconciliation_threshold(
    p_account_id UUID,
    p_expected NUMERIC(26,8),
    p_actual NUMERIC(26,8)
)
RETURNS BOOLEAN AS $$
DECLARE
    v_difference NUMERIC(26,8);
    v_percentage NUMERIC(10,4);
BEGIN
    v_difference := ABS(p_expected - p_actual);

    -- Threshold: 0.01% or absolute 0.01
    v_percentage := (v_difference / NULLIF(p_expected, 0)) * 100;

    RETURN v_difference > 0.01 OR v_percentage > 0.01;
END;
$$ LANGUAGE plpgsql;
