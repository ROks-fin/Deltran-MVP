-- Reporting Engine Database Schema
-- Version: 1.0
-- Description: Tables for report generation, scheduling, and storage

-- Report metadata table
CREATE TABLE IF NOT EXISTS reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    period_start TIMESTAMP NOT NULL,
    period_end TIMESTAMP NOT NULL,
    generated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    generated_by VARCHAR(100),
    status VARCHAR(20) DEFAULT 'pending',
    storage_path TEXT,
    file_size BIGINT,
    format VARCHAR(20),
    metadata JSONB DEFAULT '{}',
    error_message TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_reports_type ON reports(type);
CREATE INDEX IF NOT EXISTS idx_reports_period ON reports(period_start, period_end);
CREATE INDEX IF NOT EXISTS idx_reports_generated_at ON reports(generated_at DESC);
CREATE INDEX IF NOT EXISTS idx_reports_status ON reports(status);

-- Report schedules table
CREATE TABLE IF NOT EXISTS report_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    report_type VARCHAR(50) NOT NULL,
    schedule_cron VARCHAR(100) NOT NULL,
    recipients TEXT[],
    formats TEXT[] DEFAULT ARRAY['excel'],
    parameters JSONB DEFAULT '{}',
    enabled BOOLEAN DEFAULT true,
    last_run TIMESTAMP,
    next_run TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_schedules_enabled ON report_schedules(enabled);
CREATE INDEX IF NOT EXISTS idx_schedules_next_run ON report_schedules(next_run);

-- Report templates table
CREATE TABLE IF NOT EXISTS report_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL,
    version INT DEFAULT 1,
    layout JSONB NOT NULL,
    styles JSONB,
    queries JSONB,
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_templates_type ON report_templates(type);
CREATE INDEX IF NOT EXISTS idx_templates_active ON report_templates(active);

-- Audit log for report access
CREATE TABLE IF NOT EXISTS report_access_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id UUID REFERENCES reports(id),
    accessed_by VARCHAR(100),
    access_type VARCHAR(20), -- view, download, share
    ip_address INET,
    user_agent TEXT,
    accessed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_access_log_report ON report_access_log(report_id);
CREATE INDEX IF NOT EXISTS idx_access_log_user ON report_access_log(accessed_by);
CREATE INDEX IF NOT EXISTS idx_access_log_timestamp ON report_access_log(accessed_at DESC);

-- Materialized view for daily transaction summary
CREATE MATERIALIZED VIEW IF NOT EXISTS daily_transaction_summary AS
SELECT
    DATE(created_at) as transaction_date,
    COUNT(*) as total_transactions,
    SUM(amount) as total_volume,
    AVG(amount) as avg_transaction_size,
    COUNT(DISTINCT sender_bank) as unique_senders,
    COUNT(DISTINCT receiver_bank) as unique_receivers,
    SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
    SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending,
    AVG(EXTRACT(EPOCH FROM (updated_at - created_at))) as avg_processing_time
FROM transactions
WHERE created_at >= CURRENT_DATE - INTERVAL '90 days'
GROUP BY DATE(created_at);

CREATE UNIQUE INDEX IF NOT EXISTS daily_summary_date_idx ON daily_transaction_summary(transaction_date);

-- Materialized view for AML metrics
CREATE MATERIALIZED VIEW IF NOT EXISTS aml_daily_metrics AS
SELECT
    DATE(created_at) as metric_date,
    COUNT(*) as total_checks,
    SUM(CASE WHEN risk_score > 70 THEN 1 ELSE 0 END) as high_risk_count,
    SUM(CASE WHEN risk_score > 85 THEN 1 ELSE 0 END) as critical_risk_count,
    SUM(CASE WHEN sanctions_hit = true THEN 1 ELSE 0 END) as sanctions_hits,
    AVG(risk_score) as avg_risk_score
FROM compliance_checks
WHERE created_at >= CURRENT_DATE - INTERVAL '90 days'
GROUP BY DATE(created_at);

CREATE UNIQUE INDEX IF NOT EXISTS aml_metrics_date_idx ON aml_daily_metrics(metric_date);

-- Materialized view for settlement efficiency
CREATE MATERIALIZED VIEW IF NOT EXISTS settlement_efficiency_view AS
SELECT
    window_id,
    window_start,
    window_end,
    COUNT(*) as total_obligations,
    SUM(gross_amount) as gross_volume,
    SUM(net_amount) as net_volume,
    (SUM(gross_amount) - SUM(net_amount)) as volume_reduction,
    ROUND(((SUM(gross_amount) - SUM(net_amount)) / NULLIF(SUM(gross_amount), 0) * 100)::numeric, 2) as efficiency_percent
FROM settlement_instructions
WHERE window_start >= CURRENT_DATE - INTERVAL '90 days'
GROUP BY window_id, window_start, window_end;

CREATE UNIQUE INDEX IF NOT EXISTS settlement_eff_window_idx ON settlement_efficiency_view(window_id);

-- Function to refresh materialized views
CREATE OR REPLACE FUNCTION refresh_reporting_views()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY daily_transaction_summary;
    REFRESH MATERIALIZED VIEW CONCURRENTLY aml_daily_metrics;
    REFRESH MATERIALIZED VIEW CONCURRENTLY settlement_efficiency_view;
END;
$$ LANGUAGE plpgsql;

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers for updated_at
CREATE TRIGGER update_reports_updated_at
    BEFORE UPDATE ON reports
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_schedules_updated_at
    BEFORE UPDATE ON report_schedules
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_templates_updated_at
    BEFORE UPDATE ON report_templates
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Insert default report templates
INSERT INTO report_templates (name, type, layout, styles, active) VALUES
('AML Compliance Report', 'aml',
 '{"sheets": ["Executive Summary", "Transaction Analysis", "Risk Indicators", "Suspicious Activities", "Sanctions Screening", "Compliance Actions"]}',
 '{"header_bg": "#2C3E50", "header_font": "#FFFFFF", "data_font": "#000000"}',
 true),
('Settlement Report', 'settlement',
 '{"sheets": ["Summary", "Gross Positions", "Netting Results", "Net Obligations", "Liquidity Analysis", "Efficiency Metrics"]}',
 '{"header_bg": "#34495E", "header_font": "#FFFFFF", "data_font": "#000000"}',
 true),
('Reconciliation Report', 'reconciliation',
 '{"sheets": ["Summary", "Matched Transactions", "Discrepancies", "Unmatched Items", "Action Items"]}',
 '{"header_bg": "#16A085", "header_font": "#FFFFFF", "data_font": "#000000"}',
 true),
('Operational Metrics', 'operational',
 '{"sheets": ["Dashboard", "Performance Metrics", "System Health", "Error Analysis"]}',
 '{"header_bg": "#2980B9", "header_font": "#FFFFFF", "data_font": "#000000"}',
 true);

-- Insert default report schedules
INSERT INTO report_schedules (name, report_type, schedule_cron, recipients, formats, enabled) VALUES
('Daily AML Report', 'aml', '0 30 0 * * *',
 ARRAY['compliance@deltran.com', 'risk@deltran.com'],
 ARRAY['excel'], true),
('Daily Settlement Report', 'settlement', '0 30 0 * * *',
 ARRAY['settlements@deltran.com', 'cfo@deltran.com'],
 ARRAY['excel', 'csv'], true),
('Weekly Reconciliation Report', 'reconciliation', '0 0 1 * * MON',
 ARRAY['operations@deltran.com'],
 ARRAY['excel'], true),
('Monthly Operational Report', 'operational', '0 0 2 1 * *',
 ARRAY['cto@deltran.com', 'operations@deltran.com'],
 ARRAY['excel'], true),
('Quarterly Compliance Report', 'aml', '0 0 3 1 1,4,7,10 *',
 ARRAY['compliance@deltran.com', 'auditors@deltran.com'],
 ARRAY['excel'], true);

-- Grant permissions
GRANT SELECT, INSERT, UPDATE ON reports TO deltran;
GRANT SELECT, INSERT, UPDATE ON report_schedules TO deltran;
GRANT SELECT ON report_templates TO deltran;
GRANT INSERT ON report_access_log TO deltran;
GRANT SELECT ON daily_transaction_summary TO deltran;
GRANT SELECT ON aml_daily_metrics TO deltran;
GRANT SELECT ON settlement_efficiency_view TO deltran;

-- Comments for documentation
COMMENT ON TABLE reports IS 'Stores metadata for all generated reports';
COMMENT ON TABLE report_schedules IS 'Configuration for scheduled report generation';
COMMENT ON TABLE report_templates IS 'Predefined templates for report layouts and formatting';
COMMENT ON TABLE report_access_log IS 'Audit trail for report access and downloads';
COMMENT ON MATERIALIZED VIEW daily_transaction_summary IS 'Aggregated daily transaction metrics for performance';
COMMENT ON MATERIALIZED VIEW aml_daily_metrics IS 'Daily AML compliance metrics';
COMMENT ON MATERIALIZED VIEW settlement_efficiency_view IS 'Settlement netting efficiency metrics';
