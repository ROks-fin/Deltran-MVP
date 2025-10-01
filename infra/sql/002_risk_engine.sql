-- Risk Engine persistence tables

-- Risk mode configuration per currency pair
CREATE TABLE IF NOT EXISTS risk_mode (
    pair TEXT PRIMARY KEY,
    mode TEXT NOT NULL CHECK (mode IN ('CONSERVATIVE', 'BALANCED', 'AGGRESSIVE')),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_by TEXT,
    reason TEXT
);

-- Risk state snapshots
CREATE TABLE IF NOT EXISTS risk_snapshot (
    pair TEXT PRIMARY KEY,
    payload JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    version INTEGER DEFAULT 1
);

-- Risk events audit log
CREATE TABLE IF NOT EXISTS risk_events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    severity TEXT CHECK (severity IN ('info', 'minor', 'major', 'critical')),
    pair TEXT,
    transaction_id TEXT,
    participant_id TEXT,
    metric_type TEXT,
    threshold_value DECIMAL(20, 8),
    actual_value DECIMAL(20, 8),
    deviation_pct DECIMAL(10, 4),
    action_taken TEXT,
    detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

-- Indexes for performance
CREATE INDEX idx_risk_mode_updated_at ON risk_mode(updated_at DESC);
CREATE INDEX idx_risk_snapshot_updated_at ON risk_snapshot(updated_at DESC);
CREATE INDEX idx_risk_events_detected_at ON risk_events(detected_at DESC);
CREATE INDEX idx_risk_events_transaction_id ON risk_events(transaction_id);
CREATE INDEX idx_risk_events_severity ON risk_events(severity);

-- Default risk modes
INSERT INTO risk_mode (pair, mode) VALUES
    ('DEFAULT', 'BALANCED'),
    ('USD/USD', 'BALANCED'),
    ('AED/INR', 'CONSERVATIVE')
ON CONFLICT (pair) DO NOTHING;

-- Function to update timestamp
CREATE OR REPLACE FUNCTION update_risk_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers for auto-updating timestamps
CREATE TRIGGER update_risk_mode_timestamp
BEFORE UPDATE ON risk_mode
FOR EACH ROW
EXECUTE FUNCTION update_risk_timestamp();

CREATE TRIGGER update_risk_snapshot_timestamp
BEFORE UPDATE ON risk_snapshot
FOR EACH ROW
EXECUTE FUNCTION update_risk_timestamp();
