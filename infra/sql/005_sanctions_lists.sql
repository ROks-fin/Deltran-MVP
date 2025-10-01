-- 005_sanctions_lists.sql
-- Sanctions Lists Database Schema (OFAC, UN, EU)

-- Sanctions Lists Table
CREATE TABLE IF NOT EXISTS sanctions_lists (
    list_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    list_source VARCHAR(50) NOT NULL, -- OFAC, UN, EU, UK, etc.
    list_name VARCHAR(100) NOT NULL,  -- SDN, Consolidated, etc.
    version VARCHAR(50) NOT NULL,
    effective_date TIMESTAMP NOT NULL,
    last_updated TIMESTAMP NOT NULL DEFAULT NOW(),
    entries_count INTEGER DEFAULT 0,
    metadata JSONB,
    UNIQUE(list_source, list_name, version)
);

CREATE INDEX idx_sanctions_lists_source ON sanctions_lists(list_source);
CREATE INDEX idx_sanctions_lists_effective ON sanctions_lists(effective_date DESC);

-- Sanctions Entries Table (Individuals, Entities, Vessels, Aircraft)
CREATE TABLE IF NOT EXISTS sanctions_entries (
    entry_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    list_id UUID NOT NULL REFERENCES sanctions_lists(list_id) ON DELETE CASCADE,

    -- Source system ID
    source_entry_id VARCHAR(100) NOT NULL, -- UID from OFAC, UN, etc.

    -- Entity information
    entity_name VARCHAR(500) NOT NULL,
    entity_type VARCHAR(50) NOT NULL, -- individual, entity, vessel, aircraft

    -- Program/Regime
    program VARCHAR(100), -- SDGT, UKRAINE, IRAN, SYRIA, etc.
    list_date DATE, -- When added to list
    remarks TEXT,

    -- Status
    is_active BOOLEAN DEFAULT TRUE,
    action VARCHAR(20) DEFAULT 'ADD', -- ADD, UPDATE, DELETE

    -- Metadata
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    metadata JSONB,

    UNIQUE(list_id, source_entry_id)
);

CREATE INDEX idx_sanctions_entries_list ON sanctions_entries(list_id);
CREATE INDEX idx_sanctions_entries_name ON sanctions_entries USING gin(to_tsvector('english', entity_name));
CREATE INDEX idx_sanctions_entries_type ON sanctions_entries(entity_type);
CREATE INDEX idx_sanctions_entries_active ON sanctions_entries(is_active);
CREATE INDEX idx_sanctions_entries_program ON sanctions_entries(program);

-- AKA Names (Also Known As)
CREATE TABLE IF NOT EXISTS sanctions_aka_names (
    aka_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_id UUID NOT NULL REFERENCES sanctions_entries(entry_id) ON DELETE CASCADE,
    aka_name VARCHAR(500) NOT NULL,
    aka_type VARCHAR(50), -- weak, strong, etc.
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sanctions_aka_entry ON sanctions_aka_names(entry_id);
CREATE INDEX idx_sanctions_aka_name ON sanctions_aka_names USING gin(to_tsvector('english', aka_name));

-- Addresses
CREATE TABLE IF NOT EXISTS sanctions_addresses (
    address_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_id UUID NOT NULL REFERENCES sanctions_entries(entry_id) ON DELETE CASCADE,
    address_line_1 VARCHAR(200),
    address_line_2 VARCHAR(200),
    city VARCHAR(100),
    postal_code VARCHAR(50),
    country_code CHAR(2), -- ISO 3166-1 alpha-2
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sanctions_addresses_entry ON sanctions_addresses(entry_id);
CREATE INDEX idx_sanctions_addresses_country ON sanctions_addresses(country_code);

-- Nationalities
CREATE TABLE IF NOT EXISTS sanctions_nationalities (
    nationality_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_id UUID NOT NULL REFERENCES sanctions_entries(entry_id) ON DELETE CASCADE,
    country_code CHAR(2) NOT NULL, -- ISO 3166-1 alpha-2
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sanctions_nationalities_entry ON sanctions_nationalities(entry_id);
CREATE INDEX idx_sanctions_nationalities_country ON sanctions_nationalities(country_code);

-- Dates of Birth
CREATE TABLE IF NOT EXISTS sanctions_dates_of_birth (
    dob_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_id UUID NOT NULL REFERENCES sanctions_entries(entry_id) ON DELETE CASCADE,
    date_of_birth DATE,
    date_of_birth_text VARCHAR(100), -- For approximate dates like "circa 1970"
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sanctions_dob_entry ON sanctions_dates_of_birth(entry_id);
CREATE INDEX idx_sanctions_dob_date ON sanctions_dates_of_birth(date_of_birth);

-- Identification Documents (Passports, National IDs, etc.)
CREATE TABLE IF NOT EXISTS sanctions_identifications (
    identification_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entry_id UUID NOT NULL REFERENCES sanctions_entries(entry_id) ON DELETE CASCADE,
    id_type VARCHAR(50), -- passport, national_id, tax_id, etc.
    id_number VARCHAR(100),
    issuing_country CHAR(2), -- ISO 3166-1 alpha-2
    issue_date DATE,
    expiry_date DATE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sanctions_ids_entry ON sanctions_identifications(entry_id);
CREATE INDEX idx_sanctions_ids_number ON sanctions_identifications(id_number);
CREATE INDEX idx_sanctions_ids_country ON sanctions_identifications(issuing_country);

-- Screening Results Cache
CREATE TABLE IF NOT EXISTS compliance_screenings (
    screening_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_id UUID NOT NULL,

    -- Screening details
    screened_at TIMESTAMP NOT NULL DEFAULT NOW(),
    screening_result VARCHAR(50) NOT NULL, -- CLEAR, HIT, FALSE_POSITIVE, BLOCKED, PENDING
    screening_status VARCHAR(50) NOT NULL, -- PENDING, COMPLETED, FAILED, TIMEOUT

    -- Entity screened
    entity_name VARCHAR(500) NOT NULL,
    entity_type VARCHAR(50), -- debtor, creditor
    entity_country CHAR(2),

    -- Performance
    processing_duration_ms INTEGER,

    -- Lists screened
    lists_checked TEXT[], -- ["OFAC_SDN", "UN_CONSOLIDATED", "EU_SANCTIONS"]

    -- Metadata
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_compliance_screenings_payment ON compliance_screenings(payment_id);
CREATE INDEX idx_compliance_screenings_result ON compliance_screenings(screening_result);
CREATE INDEX idx_compliance_screenings_status ON compliance_screenings(screening_status);
CREATE INDEX idx_compliance_screenings_created ON compliance_screenings(created_at DESC);

-- Screening Hits
CREATE TABLE IF NOT EXISTS compliance_screening_hits (
    hit_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    screening_id UUID NOT NULL REFERENCES compliance_screenings(screening_id) ON DELETE CASCADE,
    entry_id UUID NOT NULL REFERENCES sanctions_entries(entry_id),

    -- Match details
    match_score DECIMAL(5,4) NOT NULL, -- 0.0000 to 1.0000
    match_reason TEXT,

    -- Hit classification
    hit_type VARCHAR(50), -- SANCTIONS, PEP, ADVERSE_MEDIA, HIGH_RISK

    -- List details
    list_source VARCHAR(50),
    list_name VARCHAR(100),
    entity_name VARCHAR(500),
    program VARCHAR(100),

    -- Review
    reviewed BOOLEAN DEFAULT FALSE,
    reviewed_by VARCHAR(100),
    reviewed_at TIMESTAMP,
    review_decision VARCHAR(50), -- FALSE_POSITIVE, TRUE_POSITIVE, ESCALATED
    review_notes TEXT,

    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_screening_hits_screening ON compliance_screening_hits(screening_id);
CREATE INDEX idx_screening_hits_entry ON compliance_screening_hits(entry_id);
CREATE INDEX idx_screening_hits_score ON compliance_screening_hits(match_score DESC);
CREATE INDEX idx_screening_hits_reviewed ON compliance_screening_hits(reviewed);
CREATE INDEX idx_screening_hits_type ON compliance_screening_hits(hit_type);

-- Suspicious Transaction Reports (STRs)
CREATE TABLE IF NOT EXISTS compliance_strs (
    str_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    filing_reference VARCHAR(100) UNIQUE NOT NULL,

    -- Related payments
    payment_ids UUID[] NOT NULL,

    -- Reporter information
    institution_name VARCHAR(200) NOT NULL,
    reporter_name VARCHAR(200) NOT NULL,
    reporter_title VARCHAR(100),

    -- Suspicious activity
    narrative TEXT NOT NULL,
    indicators TEXT[] NOT NULL, -- structuring, unusual_pattern, etc.
    amount_involved DECIMAL(20,2),
    activity_start_date TIMESTAMP,
    activity_end_date TIMESTAMP,

    -- Filing status
    status VARCHAR(50) NOT NULL DEFAULT 'DRAFT', -- DRAFT, PENDING_REVIEW, APPROVED, SUBMITTED, ACKNOWLEDGED, REJECTED
    filing_date TIMESTAMP,
    submitted_at TIMESTAMP,
    regulator_reference VARCHAR(100), -- FIU/FinCEN reference

    -- Audit
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(100) NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_by VARCHAR(100),

    metadata JSONB
);

CREATE INDEX idx_strs_status ON compliance_strs(status);
CREATE INDEX idx_strs_filing_date ON compliance_strs(filing_date DESC);
CREATE INDEX idx_strs_created ON compliance_strs(created_at DESC);
CREATE INDEX idx_strs_payments ON compliance_strs USING gin(payment_ids);

-- Regulatory Audit Trail
CREATE TABLE IF NOT EXISTS compliance_audit_trail (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Entity being audited
    entity_type VARCHAR(50) NOT NULL, -- payment, screening, str
    entity_id UUID NOT NULL,

    -- Action details
    action VARCHAR(100) NOT NULL, -- created, updated, screened, reviewed, etc.
    actor VARCHAR(200) NOT NULL, -- user_id or system
    actor_role VARCHAR(50),

    -- Changes
    old_values JSONB,
    new_values JSONB,
    changes JSONB, -- Computed diff

    -- Context
    ip_address INET,
    user_agent TEXT,
    session_id UUID,

    -- Timestamp
    timestamp TIMESTAMP NOT NULL DEFAULT NOW(),

    metadata JSONB
);

CREATE INDEX idx_audit_trail_entity ON compliance_audit_trail(entity_type, entity_id);
CREATE INDEX idx_audit_trail_timestamp ON compliance_audit_trail(timestamp DESC);
CREATE INDEX idx_audit_trail_actor ON compliance_audit_trail(actor);
CREATE INDEX idx_audit_trail_action ON compliance_audit_trail(action);

-- Sanctions List Update Log
CREATE TABLE IF NOT EXISTS sanctions_update_log (
    update_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    list_id UUID REFERENCES sanctions_lists(list_id),

    -- Update details
    update_type VARCHAR(50) NOT NULL, -- FULL_LOAD, INCREMENTAL, DELETE
    entries_added INTEGER DEFAULT 0,
    entries_updated INTEGER DEFAULT 0,
    entries_removed INTEGER DEFAULT 0,

    -- Status
    status VARCHAR(50) NOT NULL, -- STARTED, IN_PROGRESS, COMPLETED, FAILED
    error_message TEXT,

    -- Timing
    started_at TIMESTAMP NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMP,
    duration_seconds INTEGER,

    -- Metadata
    version VARCHAR(50),
    source_file VARCHAR(500),
    metadata JSONB
);

CREATE INDEX idx_sanctions_update_list ON sanctions_update_log(list_id);
CREATE INDEX idx_sanctions_update_started ON sanctions_update_log(started_at DESC);
CREATE INDEX idx_sanctions_update_status ON sanctions_update_log(status);

-- PEP (Politically Exposed Persons) - Separate from sanctions
CREATE TABLE IF NOT EXISTS compliance_pep_list (
    pep_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Person details
    full_name VARCHAR(500) NOT NULL,
    aka_names TEXT[],
    date_of_birth DATE,
    nationality CHAR(2), -- ISO 3166-1 alpha-2

    -- PEP classification
    pep_tier VARCHAR(20) NOT NULL, -- TIER_1, TIER_2, TIER_3
    pep_role VARCHAR(200) NOT NULL, -- Head of State, Minister, etc.
    pep_country CHAR(2), -- Country where PEP role held

    -- Status
    is_active BOOLEAN DEFAULT TRUE,
    start_date DATE,
    end_date DATE,

    -- Source
    source VARCHAR(100), -- Internal, WorldCheck, Dow Jones, etc.

    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_pep_name ON compliance_pep_list USING gin(to_tsvector('english', full_name));
CREATE INDEX idx_pep_active ON compliance_pep_list(is_active);
CREATE INDEX idx_pep_tier ON compliance_pep_list(pep_tier);
CREATE INDEX idx_pep_country ON compliance_pep_list(pep_country);

-- High-Risk Jurisdictions
CREATE TABLE IF NOT EXISTS compliance_high_risk_countries (
    country_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    country_code CHAR(2) NOT NULL UNIQUE, -- ISO 3166-1 alpha-2
    country_name VARCHAR(100) NOT NULL,

    -- Risk classification
    risk_level VARCHAR(20) NOT NULL, -- HIGH, MEDIUM, LOW

    -- Reasons
    fatf_greylisted BOOLEAN DEFAULT FALSE,
    fatf_blacklisted BOOLEAN DEFAULT FALSE,
    ctf_concerns BOOLEAN DEFAULT FALSE, -- Counter Terrorism Financing
    aml_deficiencies BOOLEAN DEFAULT FALSE,
    corruption_index INTEGER, -- CPI score

    -- Source and dates
    source VARCHAR(100), -- FATF, FSRA, Internal
    effective_date DATE NOT NULL,
    review_date DATE,

    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_high_risk_countries_code ON compliance_high_risk_countries(country_code);
CREATE INDEX idx_high_risk_countries_level ON compliance_high_risk_countries(risk_level);

-- Travel Rule Compliance Checks
CREATE TABLE IF NOT EXISTS compliance_travel_rule_checks (
    check_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_id UUID NOT NULL,

    -- Check result
    compliant BOOLEAN NOT NULL,
    threshold_applicable DECIMAL(20,2), -- e.g., 1000.00 USD
    missing_fields TEXT[],

    -- Originator details
    originator_name VARCHAR(500),
    originator_account VARCHAR(100),
    originator_country CHAR(2),
    originator_identification_type VARCHAR(50),
    originator_identification_number VARCHAR(100),

    -- Beneficiary details
    beneficiary_name VARCHAR(500),
    beneficiary_account VARCHAR(100),
    beneficiary_country CHAR(2),
    beneficiary_identification_type VARCHAR(50),
    beneficiary_identification_number VARCHAR(100),

    checked_at TIMESTAMP NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_travel_rule_payment ON compliance_travel_rule_checks(payment_id);
CREATE INDEX idx_travel_rule_compliant ON compliance_travel_rule_checks(compliant);
CREATE INDEX idx_travel_rule_checked ON compliance_travel_rule_checks(checked_at DESC);

-- Compliance Statistics (Materialized View for Performance)
CREATE MATERIALIZED VIEW compliance_stats_daily AS
SELECT
    DATE(s.created_at) AS stat_date,
    COUNT(DISTINCT s.screening_id) AS total_screenings,
    COUNT(DISTINCT CASE WHEN s.screening_result = 'HIT' THEN s.screening_id END) AS screening_hits,
    COUNT(DISTINCT CASE WHEN s.screening_result = 'BLOCKED' THEN s.screening_id END) AS screening_blocks,
    COUNT(DISTINCT CASE WHEN s.screening_result = 'FALSE_POSITIVE' THEN s.screening_id END) AS false_positives,
    COUNT(DISTINCT h.hit_id) AS total_hits,
    AVG(s.processing_duration_ms)::INTEGER AS avg_processing_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY s.processing_duration_ms)::INTEGER AS p95_processing_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY s.processing_duration_ms)::INTEGER AS p99_processing_ms
FROM compliance_screenings s
LEFT JOIN compliance_screening_hits h ON s.screening_id = h.screening_id
GROUP BY DATE(s.created_at)
ORDER BY stat_date DESC;

CREATE UNIQUE INDEX idx_compliance_stats_daily_date ON compliance_stats_daily(stat_date);

-- Refresh function for materialized view
CREATE OR REPLACE FUNCTION refresh_compliance_stats()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY compliance_stats_daily;
END;
$$ LANGUAGE plpgsql;

-- Comments
COMMENT ON TABLE sanctions_lists IS 'Master list of sanctions lists (OFAC, UN, EU, UK)';
COMMENT ON TABLE sanctions_entries IS 'Individual sanctioned entities (persons, organizations, vessels, aircraft)';
COMMENT ON TABLE sanctions_aka_names IS 'Alternative names for sanctioned entities';
COMMENT ON TABLE compliance_screenings IS 'Payment screening results against sanctions lists';
COMMENT ON TABLE compliance_screening_hits IS 'Matches found during screening';
COMMENT ON TABLE compliance_strs IS 'Suspicious Transaction Reports filed with regulators';
COMMENT ON TABLE compliance_audit_trail IS 'Immutable audit log for regulatory compliance';
COMMENT ON TABLE compliance_pep_list IS 'Politically Exposed Persons list';
COMMENT ON TABLE compliance_high_risk_countries IS 'FATF and regulatory high-risk jurisdictions';
COMMENT ON TABLE compliance_travel_rule_checks IS 'FATF Travel Rule compliance checks';
