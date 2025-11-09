-- ============================================================================
-- REPORTING ENGINE SCHEMA
-- Version: 1.0
-- Description: Tables for enterprise reporting, analytics, and Big 4 audit reports
-- ============================================================================

-- Report definitions and templates
CREATE TABLE IF NOT EXISTS report_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_code VARCHAR(100) UNIQUE NOT NULL,
    report_name VARCHAR(255) NOT NULL,
    report_type VARCHAR(50) NOT NULL, -- AML, Settlement, Reconciliation, Operational, Compliance

    -- Description
    description TEXT,
    category VARCHAR(50), -- Daily, Weekly, Monthly, Quarterly, Annual, AdHoc

    -- Schedule
    schedule_cron VARCHAR(100), -- Cron expression for automated reports
    schedule_enabled BOOLEAN DEFAULT false,
    schedule_timezone VARCHAR(50) DEFAULT 'UTC',

    -- Query and generation
    query_template TEXT, -- SQL template for data extraction
    generation_logic TEXT, -- Custom logic or stored procedure name

    -- Output format
    output_formats VARCHAR(20)[] DEFAULT ARRAY['XLSX', 'CSV'], -- XLSX, CSV, PDF, JSON
    default_format VARCHAR(20) DEFAULT 'XLSX',

    -- Big 4 audit formatting
    audit_compliant BOOLEAN DEFAULT false,
    audit_standards TEXT[], -- PwC, Deloitte, EY, KPMG standards

    -- Recipients
    default_recipients TEXT[], -- Email addresses
    recipient_roles TEXT[], -- Roles that should receive this report

    -- Retention
    retention_days INTEGER DEFAULT 90,
    archive_after_days INTEGER DEFAULT 365,

    -- Status
    is_active BOOLEAN DEFAULT true,
    is_public BOOLEAN DEFAULT false,

    -- Metadata
    tags TEXT[],
    metadata JSONB DEFAULT '{}'::jsonb,

    -- Version control
    version INTEGER DEFAULT 1,
    previous_version_id UUID REFERENCES report_definitions(id),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by VARCHAR(100),

    INDEX idx_report_defs_type (report_type),
    INDEX idx_report_defs_code (report_code),
    INDEX idx_report_defs_category (category),
    INDEX idx_report_defs_active (is_active),
    INDEX idx_report_defs_schedule (schedule_enabled, schedule_cron)
);

-- Generated reports
CREATE TABLE IF NOT EXISTS generated_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_definition_id UUID REFERENCES report_definitions(id),
    report_code VARCHAR(100) NOT NULL,
    report_name VARCHAR(255) NOT NULL,

    -- Period
    report_period_start TIMESTAMPTZ NOT NULL,
    report_period_end TIMESTAMPTZ NOT NULL,
    period_type VARCHAR(30), -- Daily, Weekly, Monthly, Quarterly, Annual, Custom

    -- File details
    file_path TEXT,
    file_name VARCHAR(500),
    file_format VARCHAR(20) NOT NULL, -- XLSX, CSV, PDF, JSON
    file_size BIGINT,
    file_hash VARCHAR(64), -- SHA256 hash for integrity

    -- Generation details
    row_count INTEGER,
    data_points INTEGER,
    generation_time_ms BIGINT,
    generated_by VARCHAR(100),

    -- Storage
    storage_type VARCHAR(30) DEFAULT 'local', -- local, s3, azure, gcs
    storage_url TEXT,
    presigned_url TEXT,
    presigned_url_expires TIMESTAMPTZ,

    -- Status
    status VARCHAR(20) NOT NULL DEFAULT 'Generating',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,

    -- Error tracking
    error_message TEXT,
    error_code VARCHAR(50),
    retry_count INTEGER DEFAULT 0,

    -- Distribution
    sent_to TEXT[], -- Email addresses that received this report
    sent_at TIMESTAMPTZ,
    download_count INTEGER DEFAULT 0,
    last_downloaded_at TIMESTAMPTZ,

    -- Retention
    expires_at TIMESTAMPTZ,
    archived_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    CONSTRAINT valid_status CHECK (status IN ('Generating', 'Completed', 'Failed', 'Archived', 'Deleted')),
    INDEX idx_generated_reports_def (report_definition_id),
    INDEX idx_generated_reports_code (report_code),
    INDEX idx_generated_reports_period (report_period_start, report_period_end),
    INDEX idx_generated_reports_status (status),
    INDEX idx_generated_reports_generated (started_at DESC),
    INDEX idx_generated_reports_expires (expires_at)
);

-- Report schedules and execution history
CREATE TABLE IF NOT EXISTS report_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_definition_id UUID NOT NULL REFERENCES report_definitions(id),

    -- Schedule details
    schedule_name VARCHAR(255) NOT NULL,
    cron_expression VARCHAR(100) NOT NULL,
    timezone VARCHAR(50) DEFAULT 'UTC',

    -- Period configuration
    period_type VARCHAR(30) NOT NULL, -- Daily, Weekly, Monthly, Quarterly, Annual
    lookback_days INTEGER, -- For rolling periods

    -- Recipients
    recipients TEXT[] NOT NULL,
    recipient_groups TEXT[],

    -- Status
    is_active BOOLEAN DEFAULT true,
    next_run_at TIMESTAMPTZ,
    last_run_at TIMESTAMPTZ,
    last_run_status VARCHAR(20),

    -- Statistics
    successful_runs INTEGER DEFAULT 0,
    failed_runs INTEGER DEFAULT 0,
    total_runs INTEGER DEFAULT 0,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_report_schedules_def (report_definition_id),
    INDEX idx_report_schedules_active (is_active, next_run_at),
    INDEX idx_report_schedules_next_run (next_run_at)
);

-- Report execution log
CREATE TABLE IF NOT EXISTS report_execution_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_definition_id UUID REFERENCES report_definitions(id),
    generated_report_id UUID REFERENCES generated_reports(id),
    schedule_id UUID REFERENCES report_schedules(id),

    -- Execution details
    execution_type VARCHAR(20) NOT NULL, -- Scheduled, Manual, API
    triggered_by VARCHAR(100),

    -- Status
    status VARCHAR(20) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,

    -- Performance
    data_fetch_time_ms BIGINT,
    processing_time_ms BIGINT,
    file_generation_time_ms BIGINT,
    total_time_ms BIGINT,

    -- Resource usage
    memory_used_mb INTEGER,
    rows_processed INTEGER,
    queries_executed INTEGER,

    -- Error tracking
    error_message TEXT,
    error_stack TEXT,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,

    INDEX idx_execution_log_report (report_definition_id, started_at DESC),
    INDEX idx_execution_log_generated (generated_report_id),
    INDEX idx_execution_log_status (status),
    INDEX idx_execution_log_started (started_at DESC)
);

-- Materialized views for reporting (TimescaleDB hypertables recommended)
CREATE TABLE IF NOT EXISTS reporting_mv_transactions_daily (
    date DATE NOT NULL,
    bank_id UUID NOT NULL REFERENCES banks(id),
    currency VARCHAR(3) NOT NULL,

    -- Transaction metrics
    transaction_count INTEGER NOT NULL DEFAULT 0,
    successful_count INTEGER NOT NULL DEFAULT 0,
    failed_count INTEGER NOT NULL DEFAULT 0,
    pending_count INTEGER NOT NULL DEFAULT 0,

    -- Financial metrics
    total_volume DECIMAL(20,2) NOT NULL DEFAULT 0,
    avg_transaction_amount DECIMAL(20,2),
    max_transaction_amount DECIMAL(20,2),
    min_transaction_amount DECIMAL(20,2),

    -- Settlement metrics
    instant_settlement_count INTEGER DEFAULT 0,
    deferred_settlement_count INTEGER DEFAULT 0,
    netted_settlement_count INTEGER DEFAULT 0,

    -- Compliance metrics
    compliance_checks_count INTEGER DEFAULT 0,
    compliance_holds_count INTEGER DEFAULT 0,
    compliance_rejections_count INTEGER DEFAULT 0,

    -- Risk metrics
    high_risk_count INTEGER DEFAULT 0,
    risk_rejections_count INTEGER DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (date, bank_id, currency),
    INDEX idx_mv_trans_daily_date (date DESC),
    INDEX idx_mv_trans_daily_bank (bank_id)
);

-- Settlement analytics
CREATE TABLE IF NOT EXISTS reporting_mv_settlements_daily (
    date DATE NOT NULL,
    clearing_window_id BIGINT,
    currency VARCHAR(3) NOT NULL,

    -- Settlement metrics
    settlement_count INTEGER NOT NULL DEFAULT 0,
    successful_settlements INTEGER DEFAULT 0,
    failed_settlements INTEGER DEFAULT 0,

    -- Financial metrics
    total_settled_amount DECIMAL(20,2) NOT NULL DEFAULT 0,
    avg_settlement_amount DECIMAL(20,2),

    -- Netting metrics
    gross_obligations DECIMAL(20,2) DEFAULT 0,
    net_obligations DECIMAL(20,2) DEFAULT 0,
    netting_efficiency DECIMAL(5,2) DEFAULT 0,
    amount_saved DECIMAL(20,2) DEFAULT 0,

    -- Performance metrics
    avg_settlement_time_seconds DECIMAL(10,2),
    max_settlement_time_seconds DECIMAL(10,2),

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (date, currency),
    INDEX idx_mv_settle_daily_date (date DESC)
);

-- Compliance analytics
CREATE TABLE IF NOT EXISTS reporting_mv_compliance_daily (
    date DATE NOT NULL,
    bank_id UUID NOT NULL REFERENCES banks(id),

    -- Compliance checks
    total_checks INTEGER NOT NULL DEFAULT 0,
    approved_checks INTEGER DEFAULT 0,
    rejected_checks INTEGER DEFAULT 0,
    review_required_checks INTEGER DEFAULT 0,

    -- Risk ratings
    high_risk_count INTEGER DEFAULT 0,
    medium_risk_count INTEGER DEFAULT 0,
    low_risk_count INTEGER DEFAULT 0,

    -- Sanctions screening
    sanctions_hits INTEGER DEFAULT 0,
    sanctions_false_positives INTEGER DEFAULT 0,

    -- PEP screening
    pep_matches INTEGER DEFAULT 0,
    pep_verified_matches INTEGER DEFAULT 0,

    -- AML alerts
    aml_alerts_generated INTEGER DEFAULT 0,
    sar_filed INTEGER DEFAULT 0,
    ctr_filed INTEGER DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (date, bank_id),
    INDEX idx_mv_compliance_daily_date (date DESC),
    INDEX idx_mv_compliance_daily_bank (bank_id)
);

-- Report downloads tracking
CREATE TABLE IF NOT EXISTS report_downloads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    generated_report_id UUID NOT NULL REFERENCES generated_reports(id),

    -- Download details
    downloaded_by VARCHAR(100),
    downloaded_by_email VARCHAR(255),
    user_id UUID,
    bank_id UUID REFERENCES banks(id),

    -- Access details
    ip_address INET,
    user_agent TEXT,
    download_method VARCHAR(30), -- Direct, Email, API

    downloaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_report_downloads_report (generated_report_id),
    INDEX idx_report_downloads_user (user_id),
    INDEX idx_report_downloads_time (downloaded_at DESC)
);

-- Report templates configuration (for Big 4 formatting)
CREATE TABLE IF NOT EXISTS report_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_code VARCHAR(100) UNIQUE NOT NULL,
    template_name VARCHAR(255) NOT NULL,

    -- Template type
    report_type VARCHAR(50) NOT NULL,
    output_format VARCHAR(20) NOT NULL,

    -- Big 4 compliance
    audit_standard VARCHAR(50), -- PwC, Deloitte, EY, KPMG, SOX, IFRS
    compliance_framework VARCHAR(50), -- IFRS, GAAP, Basel III

    -- Template structure
    structure JSONB NOT NULL, -- JSON definition of report structure
    styling JSONB, -- CSS or Excel styling definitions
    header_template TEXT,
    footer_template TEXT,

    -- Sections
    required_sections TEXT[],
    optional_sections TEXT[],
    section_order TEXT[],

    -- Status
    is_active BOOLEAN DEFAULT true,
    version INTEGER DEFAULT 1,

    -- Metadata
    description TEXT,
    metadata JSONB DEFAULT '{}'::jsonb,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_report_templates_code (template_code),
    INDEX idx_report_templates_type (report_type),
    INDEX idx_report_templates_standard (audit_standard)
);

-- Functions and triggers

-- Refresh materialized views
CREATE OR REPLACE FUNCTION refresh_reporting_materialized_views(target_date DATE)
RETURNS VOID AS $$
BEGIN
    -- Refresh transactions daily aggregates
    INSERT INTO reporting_mv_transactions_daily (
        date, bank_id, currency,
        transaction_count, successful_count, failed_count, pending_count,
        total_volume, avg_transaction_amount, max_transaction_amount, min_transaction_amount,
        instant_settlement_count, deferred_settlement_count,
        compliance_checks_count, compliance_holds_count, compliance_rejections_count,
        high_risk_count, risk_rejections_count
    )
    SELECT
        DATE(t.created_at) as date,
        t.sender_bank_id as bank_id,
        t.sent_currency as currency,
        COUNT(*) as transaction_count,
        COUNT(*) FILTER (WHERE t.status = 'Completed') as successful_count,
        COUNT(*) FILTER (WHERE t.status = 'Failed') as failed_count,
        COUNT(*) FILTER (WHERE t.status IN ('Pending', 'Processing')) as pending_count,
        SUM(t.sent_amount) as total_volume,
        AVG(t.sent_amount) as avg_transaction_amount,
        MAX(t.sent_amount) as max_transaction_amount,
        MIN(t.sent_amount) as min_transaction_amount,
        COUNT(*) FILTER (WHERE t.settlement_type = 'Instant') as instant_settlement_count,
        COUNT(*) FILTER (WHERE t.settlement_type = 'Deferred') as deferred_settlement_count,
        COUNT(cc.id) as compliance_checks_count,
        COUNT(*) FILTER (WHERE cc.overall_status = 'Hold') as compliance_holds_count,
        COUNT(*) FILTER (WHERE cc.overall_status = 'Rejected') as compliance_rejections_count,
        COUNT(*) FILTER (WHERE rs.overall_score > 75) as high_risk_count,
        COUNT(*) FILTER (WHERE rs.decision = 'Reject') as risk_rejections_count
    FROM transactions t
    LEFT JOIN compliance_checks cc ON t.id = cc.transaction_id
    LEFT JOIN risk_scores rs ON t.id = rs.transaction_id
    WHERE DATE(t.created_at) = target_date
    GROUP BY DATE(t.created_at), t.sender_bank_id, t.sent_currency
    ON CONFLICT (date, bank_id, currency) DO UPDATE SET
        transaction_count = EXCLUDED.transaction_count,
        successful_count = EXCLUDED.successful_count,
        failed_count = EXCLUDED.failed_count,
        pending_count = EXCLUDED.pending_count,
        total_volume = EXCLUDED.total_volume,
        avg_transaction_amount = EXCLUDED.avg_transaction_amount,
        max_transaction_amount = EXCLUDED.max_transaction_amount,
        min_transaction_amount = EXCLUDED.min_transaction_amount,
        updated_at = NOW();

    -- Refresh settlements daily aggregates
    INSERT INTO reporting_mv_settlements_daily (
        date, currency,
        settlement_count, successful_settlements, failed_settlements,
        total_settled_amount, avg_settlement_amount,
        gross_obligations, net_obligations, netting_efficiency, amount_saved,
        avg_settlement_time_seconds
    )
    SELECT
        DATE(cw.start_time) as date,
        nr.currency,
        COUNT(DISTINCT nr.id) as settlement_count,
        COUNT(*) FILTER (WHERE cw.status = 'Completed') as successful_settlements,
        COUNT(*) FILTER (WHERE cw.status = 'Failed') as failed_settlements,
        SUM(nr.net_amount) as total_settled_amount,
        AVG(nr.net_amount) as avg_settlement_amount,
        SUM(nr.gross_obligations_a_to_b + nr.gross_obligations_b_to_a) as gross_obligations,
        SUM(nr.net_amount) as net_obligations,
        AVG(nr.netting_efficiency) as netting_efficiency,
        SUM((nr.gross_obligations_a_to_b + nr.gross_obligations_b_to_a) - nr.net_amount) as amount_saved,
        AVG(EXTRACT(EPOCH FROM (cw.completed_at - cw.start_time))) as avg_settlement_time_seconds
    FROM clearing_windows cw
    JOIN netting_results nr ON cw.id = nr.clearing_window_id
    WHERE DATE(cw.start_time) = target_date
    GROUP BY DATE(cw.start_time), nr.currency
    ON CONFLICT (date, currency) DO UPDATE SET
        settlement_count = EXCLUDED.settlement_count,
        successful_settlements = EXCLUDED.successful_settlements,
        failed_settlements = EXCLUDED.failed_settlements,
        total_settled_amount = EXCLUDED.total_settled_amount,
        updated_at = NOW();

    -- Refresh compliance daily aggregates
    INSERT INTO reporting_mv_compliance_daily (
        date, bank_id,
        total_checks, approved_checks, rejected_checks, review_required_checks,
        high_risk_count, medium_risk_count, low_risk_count,
        sanctions_hits, pep_matches, aml_alerts_generated, sar_filed, ctr_filed
    )
    SELECT
        DATE(cc.checked_at) as date,
        t.sender_bank_id as bank_id,
        COUNT(*) as total_checks,
        COUNT(*) FILTER (WHERE cc.overall_status = 'Approved') as approved_checks,
        COUNT(*) FILTER (WHERE cc.overall_status = 'Rejected') as rejected_checks,
        COUNT(*) FILTER (WHERE cc.overall_status = 'ReviewRequired') as review_required_checks,
        COUNT(*) FILTER (WHERE cc.risk_rating = 'High' OR cc.risk_rating = 'VeryHigh') as high_risk_count,
        COUNT(*) FILTER (WHERE cc.risk_rating = 'Medium') as medium_risk_count,
        COUNT(*) FILTER (WHERE cc.risk_rating = 'Low') as low_risk_count,
        COUNT(*) FILTER (WHERE cc.sanctions_result->>'hit' = 'true') as sanctions_hits,
        COUNT(*) FILTER (WHERE cc.pep_result->>'match' = 'true') as pep_matches,
        COUNT(DISTINCT ap.id) as aml_alerts_generated,
        COUNT(DISTINCT rr.id) FILTER (WHERE rr.report_type = 'SAR') as sar_filed,
        COUNT(DISTINCT rr.id) FILTER (WHERE rr.report_type = 'CTR') as ctr_filed
    FROM compliance_checks cc
    JOIN transactions t ON cc.transaction_id = t.id
    LEFT JOIN aml_patterns ap ON DATE(ap.detected_at) = DATE(cc.checked_at)
    LEFT JOIN regulatory_reports rr ON DATE(rr.filing_date) = DATE(cc.checked_at)
    WHERE DATE(cc.checked_at) = target_date
    GROUP BY DATE(cc.checked_at), t.sender_bank_id
    ON CONFLICT (date, bank_id) DO UPDATE SET
        total_checks = EXCLUDED.total_checks,
        approved_checks = EXCLUDED.approved_checks,
        rejected_checks = EXCLUDED.rejected_checks,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- Auto-expire old reports
CREATE OR REPLACE FUNCTION expire_old_generated_reports()
RETURNS INTEGER AS $$
DECLARE
    expired_count INTEGER;
BEGIN
    UPDATE generated_reports
    SET status = 'Archived',
        archived_at = NOW()
    WHERE status = 'Completed'
        AND expires_at IS NOT NULL
        AND expires_at < NOW()
        AND archived_at IS NULL;

    GET DIAGNOSTICS expired_count = ROW_COUNT;
    RETURN expired_count;
END;
$$ LANGUAGE plpgsql;

-- Views

-- Report generation queue
CREATE OR REPLACE VIEW v_report_generation_queue AS
SELECT
    rs.id as schedule_id,
    rd.report_code,
    rd.report_name,
    rs.schedule_name,
    rs.next_run_at,
    rs.cron_expression,
    rs.period_type,
    rs.recipients,
    rs.last_run_status,
    rs.failed_runs,
    rs.successful_runs
FROM report_schedules rs
JOIN report_definitions rd ON rs.report_definition_id = rd.id
WHERE rs.is_active = true
    AND rd.is_active = true
    AND rs.next_run_at <= NOW() + INTERVAL '1 hour'
ORDER BY rs.next_run_at ASC;

-- Report generation performance
CREATE OR REPLACE VIEW v_report_performance_stats AS
SELECT
    rd.report_code,
    rd.report_name,
    rd.report_type,
    COUNT(*) as total_executions,
    COUNT(*) FILTER (WHERE rel.status = 'Completed') as successful_executions,
    COUNT(*) FILTER (WHERE rel.status = 'Failed') as failed_executions,
    AVG(rel.total_time_ms) as avg_generation_time_ms,
    MAX(rel.total_time_ms) as max_generation_time_ms,
    AVG(rel.rows_processed) as avg_rows_processed,
    AVG(rel.memory_used_mb) as avg_memory_mb
FROM report_execution_log rel
JOIN report_definitions rd ON rel.report_definition_id = rd.id
WHERE rel.started_at > NOW() - INTERVAL '30 days'
GROUP BY rd.report_code, rd.report_name, rd.report_type
ORDER BY total_executions DESC;

-- Initialize default report definitions
INSERT INTO report_definitions (
    report_code, report_name, report_type, category,
    schedule_cron, schedule_enabled, output_formats,
    audit_compliant, retention_days, is_active
) VALUES
(
    'AML_DAILY',
    'Daily AML Compliance Report',
    'AML',
    'Daily',
    '0 1 * * *',
    true,
    ARRAY['XLSX', 'CSV'],
    true,
    90,
    true
),
(
    'SETTLEMENT_DAILY',
    'Daily Settlement Summary',
    'Settlement',
    'Daily',
    '0 2 * * *',
    true,
    ARRAY['XLSX', 'CSV'],
    false,
    30,
    true
),
(
    'RECONCILIATION_DAILY',
    'Daily Reconciliation Report',
    'Reconciliation',
    'Daily',
    '0 3 * * *',
    true,
    ARRAY['XLSX'],
    true,
    90,
    true
)
ON CONFLICT DO NOTHING;

-- Grant permissions
GRANT SELECT, INSERT, UPDATE ON report_definitions TO deltran;
GRANT SELECT, INSERT, UPDATE ON generated_reports TO deltran;
GRANT SELECT, INSERT, UPDATE ON report_schedules TO deltran;
GRANT SELECT, INSERT ON report_execution_log TO deltran;
GRANT SELECT, INSERT, UPDATE ON reporting_mv_transactions_daily TO deltran;
GRANT SELECT, INSERT, UPDATE ON reporting_mv_settlements_daily TO deltran;
GRANT SELECT, INSERT, UPDATE ON reporting_mv_compliance_daily TO deltran;
GRANT SELECT, INSERT ON report_downloads TO deltran;
GRANT SELECT ON report_templates TO deltran;

-- Audit log entry
INSERT INTO audit_log (entity_type, action, changes)
VALUES ('database', 'MIGRATION_008_REPORTING_ENGINE', '{"message": "Reporting engine schema created successfully"}'::jsonb);

COMMIT;
