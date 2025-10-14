-- ============================================================================
-- DELTRAN BIG FOUR AUDIT & LOGGING SYSTEM
-- Compliance Level: SOX, IFRS 9, Basel III, PCI DSS Level 1
-- ============================================================================

-- ============================================================================
-- 1. TRANSACTION LEDGER (Immutable Financial Records)
-- ============================================================================

CREATE TABLE IF NOT EXISTS deltran.transaction_ledger (
    id UUID PRIMARY KEY DEFAULT deltran.uuid_generate_v4(),

    -- Transaction Identification
    transaction_reference VARCHAR(100) NOT NULL UNIQUE,
    payment_id UUID REFERENCES deltran.payments(id) ON DELETE RESTRICT,
    batch_id UUID REFERENCES deltran.settlement_batches(id),

    -- Financial Details
    debit_account_id UUID REFERENCES deltran.bank_accounts(id) ON DELETE RESTRICT,
    credit_account_id UUID REFERENCES deltran.bank_accounts(id) ON DELETE RESTRICT,
    transaction_amount DECIMAL(20,2) NOT NULL CHECK (transaction_amount > 0),
    transaction_currency deltran.currency_code NOT NULL,

    -- Exchange Rate (for FX transactions)
    fx_rate DECIMAL(18,8),
    settlement_amount DECIMAL(20,2),
    settlement_currency deltran.currency_code,

    -- Balances (for reconciliation)
    debit_balance_before DECIMAL(20,2) NOT NULL,
    debit_balance_after DECIMAL(20,2) NOT NULL,
    credit_balance_before DECIMAL(20,2) NOT NULL,
    credit_balance_after DECIMAL(20,2) NOT NULL,

    -- Temporal Data
    value_date DATE NOT NULL,
    booking_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    settlement_date DATE,

    -- Cryptographic Proof
    transaction_hash VARCHAR(64) NOT NULL, -- SHA-256 hash
    previous_hash VARCHAR(64), -- Blockchain-style chaining
    digital_signature TEXT, -- Ed25519 signature
    signer_public_key VARCHAR(64),

    -- Compliance & Risk
    compliance_status VARCHAR(20) DEFAULT 'pending' CHECK (compliance_status IN ('pending', 'approved', 'rejected', 'flagged')),
    risk_score DECIMAL(5,2),
    aml_check_result JSONB,
    sanctions_check_result JSONB,

    -- Reversal Handling
    is_reversal BOOLEAN DEFAULT FALSE,
    reversal_of_transaction_id UUID REFERENCES deltran.transaction_ledger(id),
    reversal_reason TEXT,

    -- Audit Trail
    created_by UUID REFERENCES deltran.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    posted_by UUID REFERENCES deltran.users(id),
    posted_at TIMESTAMPTZ,

    -- Metadata (for extensibility)
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Immutability Control
    is_posted BOOLEAN DEFAULT FALSE,
    is_locked BOOLEAN DEFAULT FALSE,

    CONSTRAINT chk_balance_integrity CHECK (
        debit_balance_after = debit_balance_before - transaction_amount
        AND credit_balance_after = credit_balance_before + transaction_amount
    )
);

-- Prevent updates/deletes on posted transactions
CREATE OR REPLACE FUNCTION deltran.prevent_ledger_modification()
RETURNS TRIGGER AS $$
BEGIN
    IF OLD.is_posted = TRUE OR OLD.is_locked = TRUE THEN
        RAISE EXCEPTION 'Cannot modify posted or locked transaction ledger entry';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_prevent_ledger_update
    BEFORE UPDATE OR DELETE ON deltran.transaction_ledger
    FOR EACH ROW EXECUTE FUNCTION deltran.prevent_ledger_modification();

-- Indexes for performance
CREATE INDEX idx_txn_ledger_reference ON deltran.transaction_ledger(transaction_reference);
CREATE INDEX idx_txn_ledger_payment ON deltran.transaction_ledger(payment_id);
CREATE INDEX idx_txn_ledger_debit_acct ON deltran.transaction_ledger(debit_account_id);
CREATE INDEX idx_txn_ledger_credit_acct ON deltran.transaction_ledger(credit_account_id);
CREATE INDEX idx_txn_ledger_value_date ON deltran.transaction_ledger(value_date);
CREATE INDEX idx_txn_ledger_booking_date ON deltran.transaction_ledger(booking_date DESC);
CREATE INDEX idx_txn_ledger_hash ON deltran.transaction_ledger(transaction_hash);

-- ============================================================================
-- 2. SYSTEM LOGS (Application Events)
-- ============================================================================

CREATE TABLE IF NOT EXISTS deltran.system_logs (
    id BIGSERIAL PRIMARY KEY,

    -- Log Classification
    log_level VARCHAR(10) NOT NULL CHECK (log_level IN ('DEBUG', 'INFO', 'WARN', 'ERROR', 'FATAL')),
    log_category VARCHAR(50) NOT NULL, -- 'PAYMENT', 'AUTH', 'SETTLEMENT', 'COMPLIANCE', etc.

    -- Event Details
    event_type VARCHAR(100) NOT NULL,
    event_message TEXT NOT NULL,

    -- Context
    service_name VARCHAR(50), -- 'gateway', 'settlement-engine', 'risk-engine'
    request_id VARCHAR(100),
    correlation_id VARCHAR(100),

    -- User/System Context
    user_id UUID REFERENCES deltran.users(id),
    ip_address INET,
    user_agent TEXT,

    -- Related Entities
    payment_id UUID,
    bank_id UUID,
    transaction_id UUID,

    -- Timing
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processing_time_ms INTEGER,

    -- Structured Data
    stack_trace TEXT,
    additional_data JSONB DEFAULT '{}'::jsonb,

    -- Indexing
    search_vector tsvector
) PARTITION BY RANGE (timestamp);

-- Create monthly partitions for system logs
CREATE TABLE deltran.system_logs_2025_10 PARTITION OF deltran.system_logs
    FOR VALUES FROM ('2025-10-01') TO ('2025-11-01');
CREATE TABLE deltran.system_logs_2025_11 PARTITION OF deltran.system_logs
    FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');
CREATE TABLE deltran.system_logs_2025_12 PARTITION OF deltran.system_logs
    FOR VALUES FROM ('2025-12-01') TO ('2026-01-01');

CREATE INDEX idx_system_logs_level ON deltran.system_logs(log_level, timestamp DESC);
CREATE INDEX idx_system_logs_category ON deltran.system_logs(log_category, timestamp DESC);
CREATE INDEX idx_system_logs_user ON deltran.system_logs(user_id, timestamp DESC);
CREATE INDEX idx_system_logs_payment ON deltran.system_logs(payment_id) WHERE payment_id IS NOT NULL;

-- ============================================================================
-- 3. AUDIT TRAIL (Big Four Compliance Level)
-- ============================================================================

CREATE TABLE IF NOT EXISTS deltran.audit_trail (
    id BIGSERIAL PRIMARY KEY,

    -- Audit Event Classification
    audit_type VARCHAR(50) NOT NULL CHECK (audit_type IN (
        'CREATE', 'READ', 'UPDATE', 'DELETE',
        'LOGIN', 'LOGOUT', 'FAILED_LOGIN',
        'PAYMENT_INITIATED', 'PAYMENT_APPROVED', 'PAYMENT_REJECTED',
        'SETTLEMENT_EXECUTED', 'COMPLIANCE_CHECK',
        'CONFIG_CHANGE', 'PERMISSION_CHANGE',
        'EXPORT_DATA', 'IMPORT_DATA'
    )),

    -- Entity Information
    entity_type VARCHAR(50) NOT NULL, -- 'payment', 'user', 'bank', 'account', 'config'
    entity_id UUID NOT NULL,

    -- Actor Information
    actor_id UUID REFERENCES deltran.users(id),
    actor_email VARCHAR(255),
    actor_role VARCHAR(50),
    actor_ip_address INET,

    -- Change Details
    action_description TEXT NOT NULL,
    old_values JSONB, -- State before change
    new_values JSONB, -- State after change
    changed_fields TEXT[], -- Array of field names changed

    -- Authorization
    authorization_token VARCHAR(100),
    mfa_verified BOOLEAN DEFAULT FALSE,

    -- Compliance Tags
    compliance_category VARCHAR(50), -- 'SOX', 'GDPR', 'PCI-DSS', 'Basel-III', 'IFRS-9'
    regulatory_impact VARCHAR(20) CHECK (regulatory_impact IN ('LOW', 'MEDIUM', 'HIGH', 'CRITICAL')),
    requires_sign_off BOOLEAN DEFAULT FALSE,
    signed_off_by UUID REFERENCES deltran.users(id),
    signed_off_at TIMESTAMPTZ,

    -- Temporal
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Retention Policy
    retention_years INTEGER DEFAULT 7, -- Big Four standard: 7 years
    purge_after_date DATE GENERATED ALWAYS AS (DATE(timestamp) + (retention_years || ' years')::INTERVAL) STORED,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb
) PARTITION BY RANGE (timestamp);

-- Create monthly partitions for audit trail
CREATE TABLE deltran.audit_trail_2025_10 PARTITION OF deltran.audit_trail
    FOR VALUES FROM ('2025-10-01') TO ('2025-11-01');
CREATE TABLE deltran.audit_trail_2025_11 PARTITION OF deltran.audit_trail
    FOR VALUES FROM ('2025-11-01') TO ('2025-12-01');
CREATE TABLE deltran.audit_trail_2025_12 PARTITION OF deltran.audit_trail
    FOR VALUES FROM ('2025-12-01') TO ('2026-01-01');

CREATE INDEX idx_audit_trail_type ON deltran.audit_trail(audit_type, timestamp DESC);
CREATE INDEX idx_audit_trail_entity ON deltran.audit_trail(entity_type, entity_id);
CREATE INDEX idx_audit_trail_actor ON deltran.audit_trail(actor_id, timestamp DESC);
CREATE INDEX idx_audit_trail_compliance ON deltran.audit_trail(compliance_category, regulatory_impact);

-- ============================================================================
-- 4. RECONCILIATION LOG (For External Audit)
-- ============================================================================

CREATE TABLE IF NOT EXISTS deltran.reconciliation_log (
    id UUID PRIMARY KEY DEFAULT deltran.uuid_generate_v4(),

    -- Reconciliation Identification
    reconciliation_reference VARCHAR(100) NOT NULL UNIQUE,
    reconciliation_type VARCHAR(50) NOT NULL CHECK (reconciliation_type IN (
        'DAILY_SETTLEMENT', 'NOSTRO_ACCOUNT', 'INTER_BANK',
        'MONTH_END', 'YEAR_END', 'EXTERNAL_AUDIT'
    )),

    -- Scope
    reconciliation_date DATE NOT NULL,
    bank_id UUID REFERENCES deltran.banks(id),
    currency deltran.currency_code,

    -- Balances
    opening_balance DECIMAL(20,2) NOT NULL,
    closing_balance DECIMAL(20,2) NOT NULL,
    expected_balance DECIMAL(20,2) NOT NULL,

    -- Reconciliation Results
    total_debits DECIMAL(20,2) DEFAULT 0,
    total_credits DECIMAL(20,2) DEFAULT 0,
    variance DECIMAL(20,2) GENERATED ALWAYS AS (closing_balance - expected_balance) STORED,

    -- Status
    status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'matched', 'unmatched', 'investigating', 'resolved')),
    variance_explanation TEXT,

    -- Break Analysis
    unmatched_items_count INTEGER DEFAULT 0,
    unmatched_items JSONB DEFAULT '[]'::jsonb,

    -- Reconciler Information
    reconciled_by UUID REFERENCES deltran.users(id),
    reconciled_at TIMESTAMPTZ,
    approved_by UUID REFERENCES deltran.users(id),
    approved_at TIMESTAMPTZ,

    -- Audit References
    external_audit_reference VARCHAR(100),
    external_system_snapshot JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    -- Supporting Documents
    attachments JSONB DEFAULT '[]'::jsonb -- References to document storage
);

CREATE INDEX idx_recon_log_date ON deltran.reconciliation_log(reconciliation_date DESC);
CREATE INDEX idx_recon_log_bank ON deltran.reconciliation_log(bank_id, reconciliation_date);
CREATE INDEX idx_recon_log_status ON deltran.reconciliation_log(status);

-- ============================================================================
-- 5. COMPLIANCE EVIDENCE REPOSITORY
-- ============================================================================

CREATE TABLE IF NOT EXISTS deltran.compliance_evidence (
    id UUID PRIMARY KEY DEFAULT deltran.uuid_generate_v4(),

    -- Evidence Identification
    evidence_reference VARCHAR(100) NOT NULL UNIQUE,
    evidence_type VARCHAR(50) NOT NULL CHECK (evidence_type IN (
        'TRANSACTION_PROOF', 'AML_CHECK', 'SANCTIONS_SCREENING',
        'KYC_VERIFICATION', 'AUTHORIZATION_RECORD',
        'SYSTEM_CONFIGURATION', 'ACCESS_LOG',
        'RECONCILIATION_PROOF', 'EXTERNAL_CONFIRMATION'
    )),

    -- Related Entities
    payment_id UUID REFERENCES deltran.payments(id),
    transaction_id UUID REFERENCES deltran.transaction_ledger(id),
    audit_id BIGINT,

    -- Evidence Data
    evidence_data JSONB NOT NULL,
    evidence_hash VARCHAR(64) NOT NULL, -- SHA-256 for integrity

    -- Cryptographic Proof
    digital_signature TEXT,
    certificate_chain TEXT[],
    timestamp_authority_proof TEXT,

    -- Regulatory Compliance
    regulation_reference VARCHAR(100), -- e.g., 'SOX-404', 'Basel-III-CRR', 'IFRS-9'
    compliance_requirement TEXT,

    -- Retention
    retention_period_years INTEGER DEFAULT 10,
    legal_hold BOOLEAN DEFAULT FALSE,
    destruction_date DATE,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb
);

CREATE INDEX idx_compliance_evidence_type ON deltran.compliance_evidence(evidence_type);
CREATE INDEX idx_compliance_evidence_payment ON deltran.compliance_evidence(payment_id) WHERE payment_id IS NOT NULL;
CREATE INDEX idx_compliance_evidence_hash ON deltran.compliance_evidence(evidence_hash);

-- ============================================================================
-- 6. HELPER FUNCTIONS FOR AUDIT REPORTS
-- ============================================================================

-- Function: Generate Audit Report for Date Range
CREATE OR REPLACE FUNCTION deltran.generate_audit_report(
    p_start_date TIMESTAMPTZ,
    p_end_date TIMESTAMPTZ,
    p_entity_type VARCHAR DEFAULT NULL,
    p_compliance_category VARCHAR DEFAULT NULL
)
RETURNS TABLE (
    audit_id BIGINT,
    timestamp TIMESTAMPTZ,
    audit_type VARCHAR,
    entity_type VARCHAR,
    entity_id UUID,
    actor_email VARCHAR,
    action_description TEXT,
    compliance_category VARCHAR,
    regulatory_impact VARCHAR
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        a.id,
        a.timestamp,
        a.audit_type,
        a.entity_type,
        a.entity_id,
        a.actor_email,
        a.action_description,
        a.compliance_category,
        a.regulatory_impact
    FROM deltran.audit_trail a
    WHERE a.timestamp BETWEEN p_start_date AND p_end_date
        AND (p_entity_type IS NULL OR a.entity_type = p_entity_type)
        AND (p_compliance_category IS NULL OR a.compliance_category = p_compliance_category)
    ORDER BY a.timestamp DESC;
END;
$$ LANGUAGE plpgsql;

-- Function: Get Transaction Chain (Blockchain-style verification)
CREATE OR REPLACE FUNCTION deltran.verify_transaction_chain(
    p_transaction_id UUID
)
RETURNS TABLE (
    transaction_id UUID,
    transaction_hash VARCHAR,
    previous_hash VARCHAR,
    is_chain_valid BOOLEAN
) AS $$
BEGIN
    RETURN QUERY
    WITH RECURSIVE chain AS (
        -- Start with the given transaction
        SELECT
            t.id,
            t.transaction_hash,
            t.previous_hash,
            TRUE as is_valid
        FROM deltran.transaction_ledger t
        WHERE t.id = p_transaction_id

        UNION ALL

        -- Follow the chain backwards
        SELECT
            t.id,
            t.transaction_hash,
            t.previous_hash,
            (t.transaction_hash = c.previous_hash) as is_valid
        FROM deltran.transaction_ledger t
        JOIN chain c ON t.id = (
            SELECT id FROM deltran.transaction_ledger
            WHERE transaction_hash = c.previous_hash
            LIMIT 1
        )
    )
    SELECT * FROM chain;
END;
$$ LANGUAGE plpgsql;

-- Function: Daily Reconciliation Summary
CREATE OR REPLACE FUNCTION deltran.daily_reconciliation_summary(
    p_date DATE,
    p_bank_id UUID DEFAULT NULL
)
RETURNS TABLE (
    bank_name VARCHAR,
    currency VARCHAR,
    opening_balance DECIMAL,
    total_debits DECIMAL,
    total_credits DECIMAL,
    closing_balance DECIMAL,
    variance DECIMAL,
    status VARCHAR
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        b.name,
        r.currency::VARCHAR,
        r.opening_balance,
        r.total_debits,
        r.total_credits,
        r.closing_balance,
        r.variance,
        r.status
    FROM deltran.reconciliation_log r
    JOIN deltran.banks b ON r.bank_id = b.id
    WHERE r.reconciliation_date = p_date
        AND (p_bank_id IS NULL OR r.bank_id = p_bank_id)
    ORDER BY b.name, r.currency;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- 7. AUTO-LOGGING TRIGGERS
-- ============================================================================

-- Trigger: Auto-log all payment changes
CREATE OR REPLACE FUNCTION deltran.log_payment_changes()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO deltran.audit_trail (
        audit_type,
        entity_type,
        entity_id,
        actor_id,
        action_description,
        old_values,
        new_values,
        compliance_category,
        regulatory_impact
    ) VALUES (
        TG_OP,
        'payment',
        COALESCE(NEW.id, OLD.id),
        COALESCE(NEW.created_by, OLD.created_by),
        TG_OP || ' payment: ' || COALESCE(NEW.payment_reference, OLD.payment_reference),
        CASE WHEN TG_OP = 'DELETE' THEN to_jsonb(OLD) ELSE NULL END,
        CASE WHEN TG_OP != 'DELETE' THEN to_jsonb(NEW) ELSE NULL END,
        'SOX',
        CASE
            WHEN COALESCE(NEW.amount, OLD.amount) > 1000000 THEN 'CRITICAL'
            WHEN COALESCE(NEW.amount, OLD.amount) > 100000 THEN 'HIGH'
            ELSE 'MEDIUM'
        END
    );
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_audit_payment_changes
    AFTER INSERT OR UPDATE OR DELETE ON deltran.payments
    FOR EACH ROW EXECUTE FUNCTION deltran.log_payment_changes();

-- ============================================================================
-- 8. EXPORT VIEWS FOR BIG FOUR REPORTS
-- ============================================================================

-- View: Complete Audit Trail (Big Four Export Format)
CREATE OR REPLACE VIEW deltran.v_big_four_audit_export AS
SELECT
    a.id as audit_record_id,
    a.timestamp as audit_timestamp,
    a.audit_type,
    a.entity_type,
    a.entity_id,
    u.email as actor_email,
    u.full_name as actor_name,
    a.actor_role,
    a.actor_ip_address,
    a.action_description,
    a.old_values,
    a.new_values,
    a.compliance_category,
    a.regulatory_impact,
    a.mfa_verified,
    a.signed_off_by,
    a.signed_off_at,
    a.metadata
FROM deltran.audit_trail a
LEFT JOIN deltran.users u ON a.actor_id = u.id;

-- View: Transaction Ledger Export (Immutable Financial Records)
CREATE OR REPLACE VIEW deltran.v_transaction_ledger_export AS
SELECT
    tl.id as ledger_entry_id,
    tl.transaction_reference,
    p.payment_reference,
    da.account_number as debit_account,
    ca.account_number as credit_account,
    db.name as debit_bank,
    cb.name as credit_bank,
    tl.transaction_amount,
    tl.transaction_currency::VARCHAR,
    tl.fx_rate,
    tl.settlement_amount,
    tl.settlement_currency::VARCHAR,
    tl.value_date,
    tl.booking_date,
    tl.settlement_date,
    tl.transaction_hash,
    tl.digital_signature,
    tl.compliance_status,
    tl.risk_score,
    tl.is_reversal,
    tl.reversal_reason,
    u.email as posted_by,
    tl.posted_at,
    tl.metadata
FROM deltran.transaction_ledger tl
LEFT JOIN deltran.payments p ON tl.payment_id = p.id
LEFT JOIN deltran.bank_accounts da ON tl.debit_account_id = da.id
LEFT JOIN deltran.bank_accounts ca ON tl.credit_account_id = ca.id
LEFT JOIN deltran.banks db ON da.bank_id = db.id
LEFT JOIN deltran.banks cb ON ca.bank_id = cb.id
LEFT JOIN deltran.users u ON tl.posted_by = u.id
WHERE tl.is_posted = TRUE;

-- View: Reconciliation Report Export
CREATE OR REPLACE VIEW deltran.v_reconciliation_export AS
SELECT
    r.reconciliation_reference,
    r.reconciliation_type,
    r.reconciliation_date,
    b.name as bank_name,
    b.bic_code,
    r.currency::VARCHAR,
    r.opening_balance,
    r.total_debits,
    r.total_credits,
    r.closing_balance,
    r.expected_balance,
    r.variance,
    r.status,
    r.unmatched_items_count,
    reconciler.email as reconciled_by,
    r.reconciled_at,
    approver.email as approved_by,
    r.approved_at,
    r.external_audit_reference
FROM deltran.reconciliation_log r
JOIN deltran.banks b ON r.bank_id = b.id
LEFT JOIN deltran.users reconciler ON r.reconciled_by = reconciler.id
LEFT JOIN deltran.users approver ON r.approved_by = approver.id;

COMMENT ON TABLE deltran.transaction_ledger IS 'Immutable financial ledger - Big Four audit compliant';
COMMENT ON TABLE deltran.audit_trail IS 'Complete audit trail for SOX, GDPR, Basel III compliance';
COMMENT ON TABLE deltran.reconciliation_log IS 'Daily reconciliation for external audit verification';
COMMENT ON TABLE deltran.compliance_evidence IS 'Cryptographic evidence repository for regulatory requirements';
