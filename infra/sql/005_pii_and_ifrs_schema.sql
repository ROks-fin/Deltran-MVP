-- DelTran Rail - PII Tokenization and IFRS Reporting Schema
-- Version: 005
-- Description: Tables for PII protection and IFRS financial reporting

-- PII tokenization tables
CREATE TABLE IF NOT EXISTS compliance.pii_tokens (
    token_id UUID PRIMARY KEY,
    field_type TEXT NOT NULL,
    encrypted_value TEXT NOT NULL,
    nonce TEXT NOT NULL,
    context TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at TIMESTAMP WITH TIME ZONE,

    -- Indexes
    CONSTRAINT valid_field_type CHECK (field_type IN (
        '"full_name"', '"email"', '"phone"', '"national_id"',
        '"bank_account"', '"iban"', '"credit_card"',
        '"address"', '"ip_address"', '"date_of_birth"', '"custom"'
    ))
);

CREATE INDEX idx_pii_tokens_created_at ON compliance.pii_tokens(created_at);
CREATE INDEX idx_pii_tokens_context ON compliance.pii_tokens(context);
CREATE INDEX idx_pii_tokens_expires_at ON compliance.pii_tokens(expires_at) WHERE expires_at IS NOT NULL;

-- PII access audit log
CREATE TABLE IF NOT EXISTS compliance.pii_access_log (
    access_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    token_id UUID NOT NULL REFERENCES compliance.pii_tokens(token_id),
    requester VARCHAR(100) NOT NULL,
    purpose TEXT NOT NULL,
    accessed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    ip_address INET,
    user_agent TEXT,

    -- Audit metadata
    access_granted BOOLEAN DEFAULT TRUE,
    denial_reason TEXT
);

CREATE INDEX idx_pii_access_log_token_id ON compliance.pii_access_log(token_id);
CREATE INDEX idx_pii_access_log_accessed_at ON compliance.pii_access_log(accessed_at);
CREATE INDEX idx_pii_access_log_requester ON compliance.pii_access_log(requester);

-- IFRS reports table
CREATE TABLE IF NOT EXISTS compliance.ifrs_reports (
    report_id UUID PRIMARY KEY,
    entity_name VARCHAR(200) NOT NULL,
    reporting_period VARCHAR(100) NOT NULL,
    period_start TIMESTAMP WITH TIME ZONE NOT NULL,
    period_end TIMESTAMP WITH TIME ZONE NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Line items (JSONB array)
    line_items JSONB NOT NULL,

    -- Metadata
    generated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    prepared_by VARCHAR(100) NOT NULL,
    approved_by VARCHAR(100),
    approved_at TIMESTAMP WITH TIME ZONE,

    -- Status
    status VARCHAR(50) DEFAULT 'draft',

    -- File references
    file_path TEXT,
    file_hash VARCHAR(64),

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_ifrs_status CHECK (status IN (
        'draft', 'pending_approval', 'approved', 'published', 'archived'
    ))
);

CREATE INDEX idx_ifrs_reports_period ON compliance.ifrs_reports(period_start, period_end);
CREATE INDEX idx_ifrs_reports_entity ON compliance.ifrs_reports(entity_name);
CREATE INDEX idx_ifrs_reports_status ON compliance.ifrs_reports(status);
CREATE INDEX idx_ifrs_reports_generated_at ON compliance.ifrs_reports(generated_at);

-- IFRS line items materialized view (for reporting)
CREATE MATERIALIZED VIEW IF NOT EXISTS compliance.v_ifrs_line_items AS
SELECT
    r.report_id,
    r.entity_name,
    r.reporting_period,
    r.period_start,
    r.period_end,
    r.currency,
    (item->>'ifrs_account')::TEXT as ifrs_account,
    (item->>'statement_type')::TEXT as statement_type,
    (item->>'amount')::DECIMAL as amount,
    (item->>'notes')::TEXT as notes,
    r.generated_at
FROM compliance.ifrs_reports r,
     jsonb_array_elements(r.line_items) as item
WHERE r.status = 'approved';

CREATE INDEX idx_v_ifrs_line_items_report_id ON compliance.v_ifrs_line_items(report_id);
CREATE INDEX idx_v_ifrs_line_items_account ON compliance.v_ifrs_line_items(ifrs_account);
CREATE INDEX idx_v_ifrs_line_items_statement ON compliance.v_ifrs_line_items(statement_type);

-- IFRS Statement of Financial Position (Balance Sheet) view
CREATE MATERIALIZED VIEW IF NOT EXISTS compliance.v_ifrs_balance_sheet AS
SELECT
    report_id,
    entity_name,
    reporting_period,
    period_end as balance_sheet_date,
    currency,

    -- Assets
    SUM(CASE WHEN ifrs_account = 'CASH_AND_CASH_EQUIVALENTS' THEN amount ELSE 0 END) as cash_and_cash_equivalents,
    SUM(CASE WHEN ifrs_account = 'SEGREGATED_CLIENT_FUNDS' THEN amount ELSE 0 END) as segregated_client_funds,
    SUM(CASE WHEN ifrs_account = 'TRADE_RECEIVABLES' THEN amount ELSE 0 END) as trade_receivables,
    SUM(CASE WHEN ifrs_account = 'CONTRACT_ASSETS' THEN amount ELSE 0 END) as contract_assets,
    SUM(CASE WHEN ifrs_account = 'PREPAID_EXPENSES' THEN amount ELSE 0 END) as prepaid_expenses,
    SUM(CASE WHEN ifrs_account = 'DEFERRED_TAX_ASSETS' THEN amount ELSE 0 END) as deferred_tax_assets,

    -- Total Assets
    SUM(CASE WHEN ifrs_account IN (
        'CASH_AND_CASH_EQUIVALENTS', 'SEGREGATED_CLIENT_FUNDS', 'TRADE_RECEIVABLES',
        'CONTRACT_ASSETS', 'PREPAID_EXPENSES', 'DEFERRED_TAX_ASSETS'
    ) THEN amount ELSE 0 END) as total_assets,

    -- Liabilities
    SUM(CASE WHEN ifrs_account = 'CLIENT_MONEY_LIABILITIES' THEN amount ELSE 0 END) as client_money_liabilities,
    SUM(CASE WHEN ifrs_account = 'TRADE_PAYABLES' THEN amount ELSE 0 END) as trade_payables,
    SUM(CASE WHEN ifrs_account = 'ACCRUED_EXPENSES' THEN amount ELSE 0 END) as accrued_expenses,
    SUM(CASE WHEN ifrs_account = 'CONTRACT_LIABILITIES' THEN amount ELSE 0 END) as contract_liabilities,
    SUM(CASE WHEN ifrs_account = 'DEFERRED_REVENUE' THEN amount ELSE 0 END) as deferred_revenue,
    SUM(CASE WHEN ifrs_account = 'DEFERRED_TAX_LIABILITIES' THEN amount ELSE 0 END) as deferred_tax_liabilities,

    -- Total Liabilities
    SUM(CASE WHEN ifrs_account IN (
        'CLIENT_MONEY_LIABILITIES', 'TRADE_PAYABLES', 'ACCRUED_EXPENSES',
        'CONTRACT_LIABILITIES', 'DEFERRED_REVENUE', 'DEFERRED_TAX_LIABILITIES'
    ) THEN amount ELSE 0 END) as total_liabilities,

    -- Equity
    SUM(CASE WHEN ifrs_account = 'SHARE_CAPITAL' THEN amount ELSE 0 END) as share_capital,
    SUM(CASE WHEN ifrs_account = 'RETAINED_EARNINGS' THEN amount ELSE 0 END) as retained_earnings,
    SUM(CASE WHEN ifrs_account = 'OTHER_COMPREHENSIVE_INCOME' THEN amount ELSE 0 END) as other_comprehensive_income,

    -- Total Equity
    SUM(CASE WHEN ifrs_account IN (
        'SHARE_CAPITAL', 'RETAINED_EARNINGS', 'OTHER_COMPREHENSIVE_INCOME'
    ) THEN amount ELSE 0 END) as total_equity,

    generated_at
FROM compliance.v_ifrs_line_items
WHERE statement_type = 'statement_of_financial_position'
GROUP BY report_id, entity_name, reporting_period, period_end, currency, generated_at;

-- IFRS Statement of Comprehensive Income (P&L) view
CREATE MATERIALIZED VIEW IF NOT EXISTS compliance.v_ifrs_income_statement AS
SELECT
    report_id,
    entity_name,
    reporting_period,
    period_start,
    period_end,
    currency,

    -- Revenue
    SUM(CASE WHEN ifrs_account = 'FX_CONVERSION_REVENUE' THEN amount ELSE 0 END) as fx_conversion_revenue,
    SUM(CASE WHEN ifrs_account = 'TRANSACTION_FEE_REVENUE' THEN amount ELSE 0 END) as transaction_fee_revenue,
    SUM(CASE WHEN ifrs_account = 'INTEREST_INCOME' THEN amount ELSE 0 END) as interest_income,
    SUM(CASE WHEN ifrs_account = 'OTHER_OPERATING_INCOME' THEN amount ELSE 0 END) as other_operating_income,

    -- Total Revenue
    SUM(CASE WHEN ifrs_account IN (
        'FX_CONVERSION_REVENUE', 'TRANSACTION_FEE_REVENUE',
        'INTEREST_INCOME', 'OTHER_OPERATING_INCOME'
    ) THEN amount ELSE 0 END) as total_revenue,

    -- Expenses
    SUM(CASE WHEN ifrs_account = 'BANK_CHARGES' THEN amount ELSE 0 END) as bank_charges,
    SUM(CASE WHEN ifrs_account = 'SETTLEMENT_COSTS' THEN amount ELSE 0 END) as settlement_costs,
    SUM(CASE WHEN ifrs_account = 'TECHNOLOGY_EXPENSES' THEN amount ELSE 0 END) as technology_expenses,
    SUM(CASE WHEN ifrs_account = 'COMPLIANCE_COSTS' THEN amount ELSE 0 END) as compliance_costs,
    SUM(CASE WHEN ifrs_account = 'PERSONNEL_EXPENSES' THEN amount ELSE 0 END) as personnel_expenses,
    SUM(CASE WHEN ifrs_account = 'DEPRECIATION_AMORTIZATION' THEN amount ELSE 0 END) as depreciation_amortization,
    SUM(CASE WHEN ifrs_account = 'OTHER_OPERATING_EXPENSES' THEN amount ELSE 0 END) as other_operating_expenses,

    -- Total Expenses
    SUM(CASE WHEN ifrs_account IN (
        'BANK_CHARGES', 'SETTLEMENT_COSTS', 'TECHNOLOGY_EXPENSES',
        'COMPLIANCE_COSTS', 'PERSONNEL_EXPENSES', 'DEPRECIATION_AMORTIZATION',
        'OTHER_OPERATING_EXPENSES'
    ) THEN amount ELSE 0 END) as total_expenses,

    -- Net Income
    SUM(CASE WHEN ifrs_account IN (
        'FX_CONVERSION_REVENUE', 'TRANSACTION_FEE_REVENUE',
        'INTEREST_INCOME', 'OTHER_OPERATING_INCOME'
    ) THEN amount ELSE 0 END) -
    SUM(CASE WHEN ifrs_account IN (
        'BANK_CHARGES', 'SETTLEMENT_COSTS', 'TECHNOLOGY_EXPENSES',
        'COMPLIANCE_COSTS', 'PERSONNEL_EXPENSES', 'DEPRECIATION_AMORTIZATION',
        'OTHER_OPERATING_EXPENSES'
    ) THEN amount ELSE 0 END) as net_income,

    generated_at
FROM compliance.v_ifrs_line_items
WHERE statement_type = 'statement_of_comprehensive_income'
GROUP BY report_id, entity_name, reporting_period, period_start, period_end, currency, generated_at;

-- PII retention policy enforcement function
CREATE OR REPLACE FUNCTION enforce_pii_retention_policy()
RETURNS void AS $$
BEGIN
    -- Delete expired tokens
    DELETE FROM compliance.pii_tokens
    WHERE expires_at IS NOT NULL
      AND expires_at < NOW();

    -- Delete access logs older than 7 years (regulatory requirement)
    DELETE FROM compliance.pii_access_log
    WHERE accessed_at < NOW() - INTERVAL '7 years';

    RAISE NOTICE 'PII retention policy enforced';
END;
$$ LANGUAGE plpgsql;

-- Schedule PII retention policy enforcement (called daily by cron)
-- Note: In production, use pg_cron extension or external scheduler

-- Create trigger for IFRS report updates
CREATE OR REPLACE FUNCTION update_ifrs_reports_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER ifrs_reports_updated_at
    BEFORE UPDATE ON compliance.ifrs_reports
    FOR EACH ROW
    EXECUTE FUNCTION update_ifrs_reports_updated_at();

-- Insert sample IFRS account mapping (for reference)
CREATE TABLE IF NOT EXISTS compliance.ifrs_account_mapping (
    subledger_account_code VARCHAR(20) PRIMARY KEY,
    ifrs_account VARCHAR(100) NOT NULL,
    statement_type VARCHAR(100) NOT NULL,
    description TEXT,
    is_debit_normal BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

INSERT INTO compliance.ifrs_account_mapping (subledger_account_code, ifrs_account, statement_type, description) VALUES
-- Assets
('1000', 'CASH_AND_CASH_EQUIVALENTS', 'statement_of_financial_position', 'Operational cash'),
('1010', 'SEGREGATED_CLIENT_FUNDS', 'statement_of_financial_position', 'Client money held in segregated accounts'),
('1100', 'TRADE_RECEIVABLES', 'statement_of_financial_position', 'Amounts due from customers'),
('1110', 'CONTRACT_ASSETS', 'statement_of_financial_position', 'Right to consideration (IFRS 15)'),
('1200', 'PREPAID_EXPENSES', 'statement_of_financial_position', 'Prepaid operational expenses'),
('1300', 'DEFERRED_TAX_ASSETS', 'statement_of_financial_position', 'Future tax benefits'),

-- Liabilities
('2000', 'CLIENT_MONEY_LIABILITIES', 'statement_of_financial_position', 'Obligation to return client funds'),
('2100', 'TRADE_PAYABLES', 'statement_of_financial_position', 'Amounts due to suppliers'),
('2110', 'ACCRUED_EXPENSES', 'statement_of_financial_position', 'Expenses incurred but not yet paid'),
('2200', 'CONTRACT_LIABILITIES', 'statement_of_financial_position', 'Obligation to provide services (IFRS 15)'),
('2210', 'DEFERRED_REVENUE', 'statement_of_financial_position', 'Unearned revenue'),
('2300', 'DEFERRED_TAX_LIABILITIES', 'statement_of_financial_position', 'Future tax obligations'),

-- Equity
('3000', 'SHARE_CAPITAL', 'statement_of_financial_position', 'Issued share capital'),
('3100', 'RETAINED_EARNINGS', 'statement_of_financial_position', 'Accumulated profits'),
('3200', 'OTHER_COMPREHENSIVE_INCOME', 'statement_of_financial_position', 'OCI items'),

-- Revenue
('4000', 'FX_CONVERSION_REVENUE', 'statement_of_comprehensive_income', 'FX conversion fees (IFRS 15)'),
('4100', 'TRANSACTION_FEE_REVENUE', 'statement_of_comprehensive_income', 'Payment transaction fees (IFRS 15)'),
('4200', 'INTEREST_INCOME', 'statement_of_comprehensive_income', 'Interest on segregated funds (IFRS 9)'),
('4900', 'OTHER_OPERATING_INCOME', 'statement_of_comprehensive_income', 'Other revenue'),

-- Expenses
('5000', 'BANK_CHARGES', 'statement_of_comprehensive_income', 'Banking fees'),
('5100', 'SETTLEMENT_COSTS', 'statement_of_comprehensive_income', 'Settlement network costs'),
('5200', 'TECHNOLOGY_EXPENSES', 'statement_of_comprehensive_income', 'IT infrastructure costs'),
('5300', 'COMPLIANCE_COSTS', 'statement_of_comprehensive_income', 'Regulatory compliance expenses'),
('5400', 'PERSONNEL_EXPENSES', 'statement_of_comprehensive_income', 'Salaries and benefits'),
('5500', 'DEPRECIATION_AMORTIZATION', 'statement_of_comprehensive_income', 'D&A of assets'),
('5900', 'OTHER_OPERATING_EXPENSES', 'statement_of_comprehensive_income', 'Other expenses')
ON CONFLICT (subledger_account_code) DO NOTHING;

-- Insert initial schema migration record
INSERT INTO schema_migrations (version) VALUES ('005_pii_and_ifrs_schema');
