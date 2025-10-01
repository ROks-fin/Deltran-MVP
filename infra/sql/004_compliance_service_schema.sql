-- DelTran Rail - Compliance Service Schema
-- Version: 004
-- Description: Database schema for regulatory reporting and compliance service

-- Create compliance schema
CREATE SCHEMA IF NOT EXISTS compliance;

-- Regulatory reports table
CREATE TABLE compliance.regulatory_reports (
    report_id UUID PRIMARY KEY,
    report_type VARCHAR(50) NOT NULL,
    report_format VARCHAR(10) NOT NULL,

    -- Period and timing
    reporting_period JSONB NOT NULL,
    generation_timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deadline TIMESTAMP WITH TIME ZONE,

    -- Status and validation
    status VARCHAR(20) DEFAULT 'pending',
    validation_status VARCHAR(20) DEFAULT 'pending',
    validation_results JSONB DEFAULT '[]',

    -- Content and metadata
    data_version_hash VARCHAR(64),
    rules_version VARCHAR(50) NOT NULL,
    source_events_count INTEGER DEFAULT 0,

    -- File references
    file_path TEXT,
    file_size_bytes BIGINT,
    file_hash VARCHAR(64),

    -- Submission tracking
    submission_reference VARCHAR(100),
    submitted_by VARCHAR(100),
    submitted_at TIMESTAMP WITH TIME ZONE,

    -- Audit and compliance
    generated_by VARCHAR(100) NOT NULL,
    approved_by VARCHAR(100),
    dual_control_verified BOOLEAN DEFAULT FALSE,

    -- Additional metadata
    contains_pii BOOLEAN DEFAULT TRUE,
    retention_until TIMESTAMP WITH TIME ZONE,
    export_allowed BOOLEAN DEFAULT FALSE,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_report_type CHECK (report_type IN (
        'pru_monthly', 'pru_quarterly', 'ifrs_annual', 'safeguarding_monthly',
        'payment_stats_quarterly', 'aml_annual', 'str_submission', 'sar_submission',
        'incident_report', 'tech_risk_quarterly', 'cyber_annual', 'model_validation_annual',
        'stress_test_quarterly', 'icaap_annual', 'ilaap_annual'
    )),
    CONSTRAINT valid_format CHECK (report_format IN ('xlsx', 'csv', 'pdf', 'xml', 'json')),
    CONSTRAINT valid_status CHECK (status IN (
        'pending', 'generating', 'validating', 'ready', 'submitted', 'accepted', 'rejected', 'failed'
    )),
    CONSTRAINT valid_validation_status CHECK (validation_status IN ('pending', 'pass', 'fail', 'warning'))
);

-- Scheduled jobs table
CREATE TABLE compliance.scheduled_jobs (
    job_id VARCHAR(100) PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    description TEXT,
    report_type VARCHAR(50) NOT NULL,
    schedule_type VARCHAR(20) NOT NULL,
    cron_expression VARCHAR(100) NOT NULL,

    -- Job configuration
    enabled BOOLEAN DEFAULT TRUE,
    format VARCHAR(10) DEFAULT 'xlsx',
    auto_submit BOOLEAN DEFAULT FALSE,
    priority INTEGER DEFAULT 1,
    max_retries INTEGER DEFAULT 3,
    timeout_minutes INTEGER DEFAULT 60,

    -- Execution tracking
    next_run TIMESTAMP WITH TIME ZONE,
    last_run TIMESTAMP WITH TIME ZONE,
    last_status VARCHAR(50),

    -- Additional metadata
    metadata JSONB DEFAULT '{}',

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_schedule_type CHECK (schedule_type IN (
        'daily', 'weekly', 'monthly', 'quarterly', 'annual', 'adhoc'
    )),
    CONSTRAINT valid_priority CHECK (priority >= 1 AND priority <= 5)
);

-- Job execution history
CREATE TABLE compliance.job_execution_history (
    execution_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_id VARCHAR(100) REFERENCES compliance.scheduled_jobs(job_id),
    report_id UUID REFERENCES compliance.regulatory_reports(report_id),

    -- Execution details
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    completed_at TIMESTAMP WITH TIME ZONE,
    status VARCHAR(50) NOT NULL,
    error_message TEXT,

    -- Performance metrics
    duration_seconds INTEGER,
    memory_used_mb INTEGER,
    cpu_time_seconds DECIMAL(10,3),

    -- Execution context
    triggered_by VARCHAR(50) NOT NULL, -- 'scheduler', 'manual', 'api'
    triggered_by_user VARCHAR(100),
    execution_metadata JSONB DEFAULT '{}',

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_execution_status CHECK (status IN (
        'started', 'running', 'completed', 'failed', 'cancelled', 'timeout'
    ))
);

-- Regulatory submission tracking
CREATE TABLE compliance.regulatory_submissions (
    submission_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    report_id UUID REFERENCES compliance.regulatory_reports(report_id),

    -- Submission details
    regulatory_authority VARCHAR(50) NOT NULL, -- 'FSRA', 'FIU', 'ADGM'
    submission_type VARCHAR(50) NOT NULL,
    submission_reference VARCHAR(100),
    submission_portal VARCHAR(100),

    -- Status tracking
    status VARCHAR(50) DEFAULT 'pending',
    submitted_at TIMESTAMP WITH TIME ZONE,
    acknowledged_at TIMESTAMP WITH TIME ZONE,
    processed_at TIMESTAMP WITH TIME ZONE,

    -- Response tracking
    authority_reference VARCHAR(100),
    response_code VARCHAR(20),
    response_message TEXT,
    response_file_path TEXT,

    -- Submission metadata
    submitted_by VARCHAR(100) NOT NULL,
    file_hash VARCHAR(64),
    file_size_bytes BIGINT,
    transmission_method VARCHAR(50), -- 'portal', 'email', 'api', 'ftp'

    -- Compliance tracking
    sla_deadline TIMESTAMP WITH TIME ZONE,
    is_overdue BOOLEAN GENERATED ALWAYS AS (
        CASE WHEN sla_deadline IS NOT NULL AND NOW() > sla_deadline AND status NOT IN ('accepted', 'processed')
        THEN TRUE ELSE FALSE END
    ) STORED,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_authority CHECK (regulatory_authority IN ('FSRA', 'FIU', 'ADGM', 'CBUAE')),
    CONSTRAINT valid_submission_status CHECK (status IN (
        'pending', 'submitted', 'acknowledged', 'processing', 'accepted', 'rejected', 'failed'
    ))
);

-- Data validation rules
CREATE TABLE compliance.validation_rules (
    rule_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    rule_name VARCHAR(200) NOT NULL,
    rule_description TEXT,
    report_type VARCHAR(50) NOT NULL,

    -- Rule definition
    rule_category VARCHAR(50) NOT NULL, -- 'completeness', 'accuracy', 'consistency', 'timeliness'
    rule_sql TEXT NOT NULL,
    rule_parameters JSONB DEFAULT '{}',

    -- Validation criteria
    severity VARCHAR(20) DEFAULT 'error', -- 'error', 'warning', 'info'
    threshold_value DECIMAL(15,6),
    comparison_operator VARCHAR(10), -- '=', '>', '<', '>=', '<=', '!='

    -- Rule status
    is_active BOOLEAN DEFAULT TRUE,
    version VARCHAR(20) DEFAULT '1.0',
    effective_from DATE NOT NULL,
    effective_to DATE,

    -- Metadata
    created_by VARCHAR(100) NOT NULL,
    approved_by VARCHAR(100),
    rule_source VARCHAR(100), -- 'FSRA', 'FIU', 'INTERNAL'

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_severity CHECK (severity IN ('error', 'warning', 'info')),
    CONSTRAINT valid_operator CHECK (comparison_operator IN ('=', '>', '<', '>=', '<=', '!=')),
    CONSTRAINT valid_rule_category CHECK (rule_category IN (
        'completeness', 'accuracy', 'consistency', 'timeliness', 'format', 'business_logic'
    ))
);

-- Validation execution results
CREATE TABLE compliance.validation_executions (
    execution_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    report_id UUID REFERENCES compliance.regulatory_reports(report_id),
    rule_id UUID REFERENCES compliance.validation_rules(rule_id),

    -- Execution details
    executed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    execution_duration_ms INTEGER,

    -- Results
    status VARCHAR(20) NOT NULL, -- 'pass', 'fail', 'warning', 'error'
    actual_value DECIMAL(15,6),
    expected_value DECIMAL(15,6),
    difference DECIMAL(15,6),

    -- Details
    result_message TEXT,
    affected_records INTEGER,
    result_details JSONB DEFAULT '{}',

    -- Context
    executed_by VARCHAR(100) DEFAULT 'system',

    CONSTRAINT valid_validation_status CHECK (status IN ('pass', 'fail', 'warning', 'error'))
);

-- Dual control approvals
CREATE TABLE compliance.dual_control_approvals (
    approval_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    report_id UUID REFERENCES compliance.regulatory_reports(report_id),

    -- Approval workflow
    approval_type VARCHAR(50) NOT NULL, -- 'generation', 'submission', 'modification'
    status VARCHAR(20) DEFAULT 'pending',

    -- First approval
    first_approver VARCHAR(100),
    first_approval_at TIMESTAMP WITH TIME ZONE,
    first_approval_comments TEXT,

    -- Second approval (dual control)
    second_approver VARCHAR(100),
    second_approval_at TIMESTAMP WITH TIME ZONE,
    second_approval_comments TEXT,

    -- Final decision
    final_decision VARCHAR(20), -- 'approved', 'rejected'
    final_decision_at TIMESTAMP WITH TIME ZONE,
    rejection_reason TEXT,

    -- Compliance requirements
    minimum_approvals INTEGER DEFAULT 2,
    same_user_approval_allowed BOOLEAN DEFAULT FALSE,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_approval_type CHECK (approval_type IN (
        'generation', 'submission', 'modification', 'deletion'
    )),
    CONSTRAINT valid_approval_status CHECK (status IN (
        'pending', 'first_approved', 'second_approved', 'approved', 'rejected'
    )),
    CONSTRAINT different_approvers CHECK (
        CASE WHEN same_user_approval_allowed = FALSE
        THEN first_approver != second_approver
        ELSE TRUE END
    )
);

-- Compliance metrics aggregates
CREATE TABLE compliance.compliance_metrics (
    metric_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    metric_date DATE NOT NULL,
    metric_type VARCHAR(50) NOT NULL,

    -- Report metrics
    reports_generated INTEGER DEFAULT 0,
    reports_submitted INTEGER DEFAULT 0,
    reports_on_time INTEGER DEFAULT 0,
    reports_overdue INTEGER DEFAULT 0,

    -- Validation metrics
    validation_pass_rate DECIMAL(5,2),
    validation_issues INTEGER DEFAULT 0,
    critical_issues INTEGER DEFAULT 0,

    -- Performance metrics
    avg_generation_time_minutes DECIMAL(8,2),
    avg_validation_time_minutes DECIMAL(8,2),
    system_availability_percent DECIMAL(5,2),

    -- Compliance metrics
    sla_compliance_rate DECIMAL(5,2),
    dual_control_compliance_rate DECIMAL(5,2),
    regulatory_deadline_adherence DECIMAL(5,2),

    -- Quality metrics
    data_quality_score DECIMAL(5,2),
    completeness_score DECIMAL(5,2),
    accuracy_score DECIMAL(5,2),

    -- Calculated fields
    total_submissions INTEGER GENERATED ALWAYS AS (
        reports_submitted + reports_overdue
    ) STORED,

    timeliness_rate DECIMAL(5,2) GENERATED ALWAYS AS (
        CASE WHEN (reports_on_time + reports_overdue) > 0
        THEN (reports_on_time * 100.0) / (reports_on_time + reports_overdue)
        ELSE 0 END
    ) STORED,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_metric_type CHECK (metric_type IN (
        'daily', 'weekly', 'monthly', 'quarterly', 'annual'
    ))
);

-- Regulatory calendar and deadlines (enhanced)
CREATE TABLE compliance.regulatory_calendar (
    calendar_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    regulation_type VARCHAR(50) NOT NULL,
    report_period VARCHAR(50) NOT NULL,

    -- Deadline information
    deadline_date DATE NOT NULL,
    submission_window_start DATE,
    submission_window_end DATE,

    -- Status tracking
    status VARCHAR(20) DEFAULT 'upcoming',
    preparation_status VARCHAR(20) DEFAULT 'not_started',

    -- Automated scheduling
    auto_generate_enabled BOOLEAN DEFAULT TRUE,
    auto_generate_date DATE,
    reminder_days_before INTEGER DEFAULT 7,

    -- Compliance tracking
    submitted_report_id UUID REFERENCES compliance.regulatory_reports(report_id),
    submission_id UUID REFERENCES compliance.regulatory_submissions(submission_id),
    actual_submission_date DATE,

    -- Risk assessment
    complexity_score INTEGER DEFAULT 1, -- 1-5 scale
    regulatory_impact VARCHAR(20) DEFAULT 'medium', -- 'low', 'medium', 'high', 'critical'
    preparation_time_required_days INTEGER DEFAULT 5,

    -- Dependencies
    prerequisite_reports JSONB DEFAULT '[]',
    dependent_reports JSONB DEFAULT '[]',

    -- Notes and comments
    special_instructions TEXT,
    preparation_notes TEXT,
    submission_notes TEXT,

    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT valid_calendar_status CHECK (status IN (
        'upcoming', 'in_progress', 'submitted', 'completed', 'overdue', 'cancelled'
    )),
    CONSTRAINT valid_preparation_status CHECK (preparation_status IN (
        'not_started', 'in_progress', 'validation_pending', 'approval_pending', 'ready', 'submitted'
    )),
    CONSTRAINT valid_impact CHECK (regulatory_impact IN ('low', 'medium', 'high', 'critical'))
);

-- Create indexes for performance
CREATE INDEX idx_regulatory_reports_type_status ON compliance.regulatory_reports(report_type, status);
CREATE INDEX idx_regulatory_reports_generation_timestamp ON compliance.regulatory_reports(generation_timestamp);
CREATE INDEX idx_regulatory_reports_deadline ON compliance.regulatory_reports(deadline);
CREATE INDEX idx_regulatory_reports_validation_status ON compliance.regulatory_reports(validation_status);

CREATE INDEX idx_scheduled_jobs_enabled_next_run ON compliance.scheduled_jobs(enabled, next_run);
CREATE INDEX idx_scheduled_jobs_schedule_type ON compliance.scheduled_jobs(schedule_type);

CREATE INDEX idx_job_execution_history_job_id ON compliance.job_execution_history(job_id);
CREATE INDEX idx_job_execution_history_started_at ON compliance.job_execution_history(started_at);
CREATE INDEX idx_job_execution_history_status ON compliance.job_execution_history(status);

CREATE INDEX idx_regulatory_submissions_authority ON compliance.regulatory_submissions(regulatory_authority);
CREATE INDEX idx_regulatory_submissions_status ON compliance.regulatory_submissions(status);
CREATE INDEX idx_regulatory_submissions_submitted_at ON compliance.regulatory_submissions(submitted_at);
CREATE INDEX idx_regulatory_submissions_overdue ON compliance.regulatory_submissions(is_overdue);

CREATE INDEX idx_validation_rules_report_type ON compliance.validation_rules(report_type);
CREATE INDEX idx_validation_rules_active ON compliance.validation_rules(is_active);
CREATE INDEX idx_validation_rules_severity ON compliance.validation_rules(severity);

CREATE INDEX idx_validation_executions_report_id ON compliance.validation_executions(report_id);
CREATE INDEX idx_validation_executions_status ON compliance.validation_executions(status);
CREATE INDEX idx_validation_executions_executed_at ON compliance.validation_executions(executed_at);

CREATE INDEX idx_dual_control_report_id ON compliance.dual_control_approvals(report_id);
CREATE INDEX idx_dual_control_status ON compliance.dual_control_approvals(status);

CREATE INDEX idx_compliance_metrics_date_type ON compliance.compliance_metrics(metric_date, metric_type);

CREATE INDEX idx_regulatory_calendar_deadline ON compliance.regulatory_calendar(deadline_date);
CREATE INDEX idx_regulatory_calendar_status ON compliance.regulatory_calendar(status);
CREATE INDEX idx_regulatory_calendar_type ON compliance.regulatory_calendar(regulation_type);

-- Create triggers for updated_at timestamps
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_regulatory_reports_updated_at BEFORE UPDATE
    ON compliance.regulatory_reports FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_scheduled_jobs_updated_at BEFORE UPDATE
    ON compliance.scheduled_jobs FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_regulatory_submissions_updated_at BEFORE UPDATE
    ON compliance.regulatory_submissions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_validation_rules_updated_at BEFORE UPDATE
    ON compliance.validation_rules FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_dual_control_approvals_updated_at BEFORE UPDATE
    ON compliance.dual_control_approvals FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_regulatory_calendar_updated_at BEFORE UPDATE
    ON compliance.regulatory_calendar FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert initial validation rules
INSERT INTO compliance.validation_rules (
    rule_name, rule_description, report_type, rule_category, rule_sql,
    severity, created_by, rule_source
) VALUES
(
    'PRU Data Completeness Check',
    'Ensure all required currencies have data for the reporting period',
    'pru_monthly',
    'completeness',
    'SELECT COUNT(DISTINCT currency) FROM gold.v_pru_monthly WHERE reporting_month = $1',
    'error',
    'system',
    'FSRA'
),
(
    'Safeguarding Reconciliation Threshold',
    'Check that reconciliation differences are within acceptable limits',
    'safeguarding_monthly',
    'accuracy',
    'SELECT MAX(ABS(reconciliation_difference)) FROM gold.v_safeguarding WHERE reporting_date BETWEEN $1 AND $2',
    'warning',
    'system',
    'FSRA'
),
(
    'AML STR Filing Rate',
    'Ensure STR filing rate is within expected range',
    'aml_annual',
    'business_logic',
    'SELECT (strs_filed * 100.0 / NULLIF(total_alerts, 0)) FROM gold.v_aml_kpis_y WHERE reporting_year = $1',
    'warning',
    'system',
    'FIU'
);

-- Insert sample regulatory calendar entries
INSERT INTO compliance.regulatory_calendar (
    regulation_type, report_period, deadline_date, auto_generate_date,
    complexity_score, regulatory_impact, preparation_time_required_days
) VALUES
('PRU_MONTHLY', '2024-01', '2024-02-15', '2024-02-10', 2, 'medium', 3),
('PRU_MONTHLY', '2024-02', '2024-03-15', '2024-03-10', 2, 'medium', 3),
('SAFEGUARDING_MONTHLY', '2024-01', '2024-02-10', '2024-02-05', 3, 'high', 5),
('AML_ANNUAL', '2023', '2024-03-31', '2024-03-01', 5, 'critical', 20),
('TECH_RISK_QUARTERLY', '2024-Q1', '2024-04-30', '2024-04-15', 3, 'medium', 10);

-- Insert initial schema migration record
INSERT INTO schema_migrations (version) VALUES ('004_compliance_service_schema');