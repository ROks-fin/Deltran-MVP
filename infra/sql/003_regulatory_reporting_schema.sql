-- DelTran Rail - Regulatory Reporting Schema (Gold Layer)
-- Version: 003
-- Description: Gold layer aggregation tables for FSRA and FIU regulatory reporting

-- Event tables for Bronze layer (raw logs)
CREATE SCHEMA IF NOT EXISTS bronze;
CREATE SCHEMA IF NOT EXISTS silver;
CREATE SCHEMA IF NOT EXISTS gold;

-- Bronze layer: Raw event logs
CREATE TABLE bronze.tx_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    event_timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    event_data JSONB NOT NULL,
    source_service VARCHAR(50) NOT NULL,
    trace_id VARCHAR(255),
    raw_payload JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE bronze.settlement_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    settlement_batch_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    event_timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    event_data JSONB NOT NULL,
    net_positions JSONB,
    participant_count INTEGER,
    total_amount DECIMAL(20,2),
    currency VARCHAR(3),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE bronze.safeguarding_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    client_account VARCHAR(50) NOT NULL,
    event_type VARCHAR(50) NOT NULL, -- 'deposit', 'withdrawal', 'hold', 'release'
    event_timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    amount DECIMAL(20,2) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    balance_before DECIMAL(20,2),
    balance_after DECIMAL(20,2),
    segregated_account VARCHAR(50),
    bank_account VARCHAR(50),
    event_data JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE bronze.aml_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    transaction_id UUID,
    event_type VARCHAR(50) NOT NULL, -- 'alert', 'str_filed', 'investigation_closed'
    event_timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    alert_type VARCHAR(50),
    risk_score DECIMAL(5,2),
    investigation_id UUID,
    str_reference VARCHAR(100),
    investigator VARCHAR(100),
    status VARCHAR(50),
    event_data JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE bronze.it_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    incident_id UUID NOT NULL,
    event_type VARCHAR(50) NOT NULL, -- 'incident_created', 'escalated', 'resolved'
    event_timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    severity VARCHAR(20),
    category VARCHAR(50), -- 'cyber', 'outage', 'data_breach', 'compliance'
    service_affected VARCHAR(50),
    impact_level VARCHAR(20),
    resolution_time_minutes INTEGER,
    event_data JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE bronze.model_events (
    event_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    model_id VARCHAR(100) NOT NULL,
    model_version VARCHAR(50) NOT NULL,
    event_type VARCHAR(50) NOT NULL, -- 'deployed', 'validated', 'performance_check'
    event_timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    validation_result VARCHAR(50),
    performance_metrics JSONB,
    validator VARCHAR(100),
    approval_status VARCHAR(50),
    event_data JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Silver layer: Normalized tables
CREATE TABLE silver.transactions_normalized (
    transaction_id UUID PRIMARY KEY,
    uetr UUID NOT NULL,
    amount DECIMAL(20,2) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    usd_equivalent DECIMAL(20,2),
    exchange_rate DECIMAL(10,6),
    transaction_date DATE NOT NULL,
    settlement_date DATE,
    debtor_country VARCHAR(2),
    creditor_country VARCHAR(2),
    payment_purpose VARCHAR(50),
    risk_score DECIMAL(5,2),
    compliance_status VARCHAR(50),
    is_cross_border BOOLEAN,
    is_high_value BOOLEAN,
    processed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE silver.client_balances_normalized (
    balance_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    client_id VARCHAR(50) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    balance_date DATE NOT NULL,
    opening_balance DECIMAL(20,2) NOT NULL,
    closing_balance DECIMAL(20,2) NOT NULL,
    total_deposits DECIMAL(20,2) DEFAULT 0,
    total_withdrawals DECIMAL(20,2) DEFAULT 0,
    segregated_bank VARCHAR(100),
    account_number VARCHAR(50),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Gold layer: Aggregation views for reporting
CREATE MATERIALIZED VIEW gold.v_pru_monthly AS
SELECT
    DATE_TRUNC('month', t.transaction_date) as reporting_month,
    t.currency,

    -- Capital adequacy metrics
    SUM(CASE WHEN t.risk_score > 75 THEN t.usd_equivalent ELSE 0 END) as high_risk_exposure,
    SUM(t.usd_equivalent) as total_exposure,
    SUM(CASE WHEN t.is_cross_border THEN t.usd_equivalent ELSE 0 END) as cross_border_exposure,

    -- Liquidity metrics
    COUNT(*) as transaction_count,
    AVG(t.usd_equivalent) as avg_transaction_value,
    STDDEV(t.usd_equivalent) as value_volatility,

    -- Risk metrics
    AVG(t.risk_score) as avg_risk_score,
    COUNT(CASE WHEN t.risk_score > 85 THEN 1 END) as high_risk_count,

    -- Settlement metrics
    AVG(EXTRACT(EPOCH FROM (t.settlement_date - t.transaction_date))/3600) as avg_settlement_hours,

    -- Operational metrics
    COUNT(CASE WHEN t.compliance_status != 'COMPLIANT' THEN 1 END) as compliance_issues,

    MIN(t.transaction_date) as period_start,
    MAX(t.transaction_date) as period_end,
    NOW() as generated_at
FROM silver.transactions_normalized t
WHERE t.transaction_date >= CURRENT_DATE - INTERVAL '2 years'
GROUP BY
    DATE_TRUNC('month', t.transaction_date),
    t.currency;

CREATE MATERIALIZED VIEW gold.v_safeguarding AS
SELECT
    DATE_TRUNC('day', cb.balance_date) as reporting_date,
    cb.currency,
    cb.segregated_bank,

    -- Client money aggregates
    SUM(cb.closing_balance) as total_client_money,
    SUM(cb.total_deposits) as daily_deposits,
    SUM(cb.total_withdrawals) as daily_withdrawals,
    COUNT(DISTINCT cb.client_id) as active_clients,

    -- Reconciliation metrics
    SUM(cb.closing_balance) as book_balance,
    -- Bank balance would come from external reconciliation
    0 as bank_balance, -- Placeholder for bank reconciliation data
    0 as reconciliation_difference,

    -- Risk metrics
    MAX(cb.closing_balance) as largest_client_balance,
    STDDEV(cb.closing_balance) as balance_concentration,

    -- Safeguarding compliance
    COUNT(CASE WHEN cb.segregated_bank IS NULL THEN 1 END) as unsegregated_accounts,

    NOW() as generated_at
FROM silver.client_balances_normalized cb
WHERE cb.balance_date >= CURRENT_DATE - INTERVAL '1 year'
GROUP BY
    DATE_TRUNC('day', cb.balance_date),
    cb.currency,
    cb.segregated_bank;

CREATE MATERIALIZED VIEW gold.v_payment_stats_q AS
SELECT
    DATE_TRUNC('quarter', t.transaction_date) as reporting_quarter,
    t.currency,
    t.debtor_country,
    t.creditor_country,

    -- Volume statistics
    COUNT(*) as transaction_count,
    SUM(t.amount) as total_volume_currency,
    SUM(t.usd_equivalent) as total_volume_usd,
    AVG(t.amount) as avg_transaction_amount,

    -- Geographic distribution
    COUNT(CASE WHEN t.is_cross_border THEN 1 END) as cross_border_count,
    COUNT(CASE WHEN NOT t.is_cross_border THEN 1 END) as domestic_count,

    -- Value distribution
    COUNT(CASE WHEN t.usd_equivalent > 10000 THEN 1 END) as large_value_count,
    COUNT(CASE WHEN t.usd_equivalent BETWEEN 1000 AND 10000 THEN 1 END) as medium_value_count,
    COUNT(CASE WHEN t.usd_equivalent < 1000 THEN 1 END) as small_value_count,

    -- Purpose analysis
    COUNT(CASE WHEN t.payment_purpose = 'TRADE' THEN 1 END) as trade_payments,
    COUNT(CASE WHEN t.payment_purpose = 'REMITTANCE' THEN 1 END) as remittance_payments,
    COUNT(CASE WHEN t.payment_purpose = 'TREASURY' THEN 1 END) as treasury_payments,

    -- Timing metrics
    MIN(t.transaction_date) as quarter_start,
    MAX(t.transaction_date) as quarter_end,
    NOW() as generated_at
FROM silver.transactions_normalized t
WHERE t.transaction_date >= CURRENT_DATE - INTERVAL '5 years'
GROUP BY
    DATE_TRUNC('quarter', t.transaction_date),
    t.currency,
    t.debtor_country,
    t.creditor_country;

CREATE MATERIALIZED VIEW gold.v_aml_kpis_y AS
SELECT
    DATE_TRUNC('year', ae.event_timestamp) as reporting_year,

    -- Alert metrics
    COUNT(CASE WHEN ae.event_type = 'alert' THEN 1 END) as total_alerts,
    COUNT(CASE WHEN ae.event_type = 'alert' AND ae.risk_score > 75 THEN 1 END) as high_risk_alerts,
    COUNT(CASE WHEN ae.event_type = 'alert' AND ae.risk_score > 90 THEN 1 END) as critical_alerts,

    -- STR metrics
    COUNT(CASE WHEN ae.event_type = 'str_filed' THEN 1 END) as strs_filed,
    COUNT(DISTINCT ae.str_reference) as unique_str_cases,

    -- Investigation metrics
    COUNT(CASE WHEN ae.event_type = 'investigation_closed' THEN 1 END) as investigations_closed,
    AVG(CASE WHEN ae.event_type = 'investigation_closed' THEN
        EXTRACT(EPOCH FROM (ae.event_timestamp - ae.created_at))/86400
    END) as avg_investigation_days,

    -- Risk distribution
    AVG(ae.risk_score) as avg_risk_score,
    STDDEV(ae.risk_score) as risk_score_variance,

    -- Investigator workload
    COUNT(DISTINCT ae.investigator) as active_investigators,
    COUNT(*) / NULLIF(COUNT(DISTINCT ae.investigator), 0) as avg_cases_per_investigator,

    -- Jurisdiction analysis
    COUNT(DISTINCT (ae.event_data->>'jurisdiction')) as jurisdictions_involved,

    MIN(ae.event_timestamp::DATE) as year_start,
    MAX(ae.event_timestamp::DATE) as year_end,
    NOW() as generated_at
FROM bronze.aml_events ae
WHERE ae.event_timestamp >= CURRENT_DATE - INTERVAL '7 years'
GROUP BY DATE_TRUNC('year', ae.event_timestamp);

-- IT and Cyber Risk reporting view
CREATE MATERIALIZED VIEW gold.v_tech_risk_q AS
SELECT
    DATE_TRUNC('quarter', ie.event_timestamp) as reporting_quarter,
    ie.category,
    ie.severity,

    -- Incident counts
    COUNT(*) as total_incidents,
    COUNT(CASE WHEN ie.severity = 'CRITICAL' THEN 1 END) as critical_incidents,
    COUNT(CASE WHEN ie.severity = 'HIGH' THEN 1 END) as high_incidents,
    COUNT(CASE WHEN ie.category = 'cyber' THEN 1 END) as cyber_incidents,
    COUNT(CASE WHEN ie.category = 'outage' THEN 1 END) as outage_incidents,

    -- Resolution metrics
    AVG(ie.resolution_time_minutes) as avg_resolution_minutes,
    MAX(ie.resolution_time_minutes) as max_resolution_minutes,
    COUNT(CASE WHEN ie.resolution_time_minutes > 240 THEN 1 END) as incidents_over_4h,

    -- Service impact
    COUNT(DISTINCT ie.service_affected) as services_affected,
    COUNT(CASE WHEN ie.impact_level = 'HIGH' THEN 1 END) as high_impact_incidents,

    -- Compliance metrics (cyber incidents reported within 24h)
    COUNT(CASE WHEN ie.category = 'cyber' AND
        EXTRACT(EPOCH FROM (ie.created_at - ie.event_timestamp))/3600 <= 24
        THEN 1 END) as cyber_reported_within_24h,

    MIN(ie.event_timestamp::DATE) as quarter_start,
    MAX(ie.event_timestamp::DATE) as quarter_end,
    NOW() as generated_at
FROM bronze.it_events ie
WHERE ie.event_timestamp >= CURRENT_DATE - INTERVAL '3 years'
GROUP BY
    DATE_TRUNC('quarter', ie.event_timestamp),
    ie.category,
    ie.severity;

-- Model validation and stress test reporting
CREATE MATERIALIZED VIEW gold.v_model_validation_y AS
SELECT
    DATE_TRUNC('year', me.event_timestamp) as reporting_year,
    me.model_id,
    me.model_version,

    -- Validation metrics
    COUNT(CASE WHEN me.event_type = 'validated' THEN 1 END) as validations_performed,
    COUNT(CASE WHEN me.event_type = 'validated' AND me.validation_result = 'PASS' THEN 1 END) as validations_passed,
    COUNT(CASE WHEN me.event_type = 'deployed' THEN 1 END) as deployments,

    -- Performance tracking
    COUNT(CASE WHEN me.event_type = 'performance_check' THEN 1 END) as performance_checks,
    AVG((me.performance_metrics->>'accuracy')::DECIMAL) as avg_model_accuracy,
    AVG((me.performance_metrics->>'precision')::DECIMAL) as avg_model_precision,

    -- Approval workflow
    COUNT(CASE WHEN me.approval_status = 'APPROVED' THEN 1 END) as approvals,
    COUNT(CASE WHEN me.approval_status = 'REJECTED' THEN 1 END) as rejections,
    COUNT(DISTINCT me.validator) as validators_involved,

    -- Latest validation
    MAX(me.event_timestamp) as last_validation_date,
    MAX(CASE WHEN me.event_type = 'validated' THEN me.validation_result END) as last_validation_result,

    MIN(me.event_timestamp::DATE) as year_start,
    MAX(me.event_timestamp::DATE) as year_end,
    NOW() as generated_at
FROM bronze.model_events me
WHERE me.event_timestamp >= CURRENT_DATE - INTERVAL '5 years'
GROUP BY
    DATE_TRUNC('year', me.event_timestamp),
    me.model_id,
    me.model_version;

-- Audit trail for all Gold layer aggregations
CREATE TABLE gold.aggregation_audit (
    audit_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    view_name VARCHAR(100) NOT NULL,
    aggregation_period VARCHAR(50) NOT NULL,
    data_version_hash VARCHAR(64) NOT NULL,
    rules_version VARCHAR(50) NOT NULL,
    source_events_count BIGINT NOT NULL,
    aggregated_records_count BIGINT NOT NULL,
    validation_status VARCHAR(50) NOT NULL,
    validated_by VARCHAR(100),
    validation_timestamp TIMESTAMP WITH TIME ZONE,
    export_ready BOOLEAN DEFAULT FALSE,
    retention_until DATE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Regulatory deadline tracking
CREATE TABLE gold.regulatory_deadlines (
    deadline_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    regulation_type VARCHAR(50) NOT NULL, -- 'PRU', 'SAFEGUARDING', 'AML_ANNUAL', etc.
    report_period VARCHAR(50) NOT NULL,
    deadline_date DATE NOT NULL,
    submission_date DATE,
    status VARCHAR(50) DEFAULT 'PENDING', -- 'PENDING', 'PREPARED', 'SUBMITTED', 'ACCEPTED'
    file_references JSONB,
    validator VARCHAR(100),
    submission_reference VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX idx_bronze_tx_events_timestamp ON bronze.tx_events(event_timestamp);
CREATE INDEX idx_bronze_tx_events_transaction_id ON bronze.tx_events(transaction_id);
CREATE INDEX idx_bronze_settlement_events_timestamp ON bronze.settlement_events(event_timestamp);
CREATE INDEX idx_bronze_safeguarding_events_timestamp ON bronze.safeguarding_events(event_timestamp);
CREATE INDEX idx_bronze_safeguarding_events_client ON bronze.safeguarding_events(client_account);
CREATE INDEX idx_bronze_aml_events_timestamp ON bronze.aml_events(event_timestamp);
CREATE INDEX idx_bronze_aml_events_transaction ON bronze.aml_events(transaction_id);
CREATE INDEX idx_bronze_it_events_timestamp ON bronze.it_events(event_timestamp);
CREATE INDEX idx_bronze_it_events_category ON bronze.it_events(category);
CREATE INDEX idx_bronze_model_events_timestamp ON bronze.model_events(event_timestamp);
CREATE INDEX idx_bronze_model_events_model_id ON bronze.model_events(model_id);

CREATE INDEX idx_silver_transactions_date ON silver.transactions_normalized(transaction_date);
CREATE INDEX idx_silver_transactions_currency ON silver.transactions_normalized(currency);
CREATE INDEX idx_silver_client_balances_date ON silver.client_balances_normalized(balance_date);
CREATE INDEX idx_silver_client_balances_client ON silver.client_balances_normalized(client_id);

CREATE INDEX idx_gold_audit_view_name ON gold.aggregation_audit(view_name);
CREATE INDEX idx_gold_audit_created_at ON gold.aggregation_audit(created_at);
CREATE INDEX idx_gold_deadlines_type ON gold.regulatory_deadlines(regulation_type);
CREATE INDEX idx_gold_deadlines_deadline ON gold.regulatory_deadlines(deadline_date);

-- Initial regulatory deadlines for 2024
INSERT INTO gold.regulatory_deadlines (regulation_type, report_period, deadline_date) VALUES
('PRU_MONTHLY', '2024-01', '2024-02-15'),
('PRU_MONTHLY', '2024-02', '2024-03-15'),
('PRU_MONTHLY', '2024-03', '2024-04-15'),
('PRU_QUARTERLY', '2024-Q1', '2024-04-30'),
('SAFEGUARDING_MONTHLY', '2024-01', '2024-02-10'),
('SAFEGUARDING_MONTHLY', '2024-02', '2024-03-10'),
('PAYMENT_STATS_QUARTERLY', '2024-Q1', '2024-04-30'),
('AML_ANNUAL', '2023', '2024-03-31'),
('TECH_RISK_QUARTERLY', '2024-Q1', '2024-04-30'),
('MODEL_VALIDATION_ANNUAL', '2023', '2024-02-28');

-- Insert initial schema migration record
INSERT INTO schema_migrations (version) VALUES ('003_regulatory_reporting_schema');