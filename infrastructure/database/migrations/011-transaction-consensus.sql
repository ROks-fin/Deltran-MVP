-- Migration 011: Transaction Consensus & State Aggregation
-- Implements event log, decision tracking, and state aggregation

-- 1. Transaction Event Log (для аудита и отладки)
CREATE TABLE IF NOT EXISTS transaction_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID NOT NULL REFERENCES transactions(id),
    service_name VARCHAR(50) NOT NULL, -- 'compliance', 'risk', 'token', 'liquidity', 'clearing', 'settlement'
    event_type VARCHAR(50) NOT NULL, -- 'DECISION', 'STATUS_CHANGE', 'ERROR', 'RETRY'
    decision VARCHAR(20), -- 'APPROVE', 'REJECT', 'REVIEW', NULL
    payload JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_events_transaction ON transaction_events(transaction_id, occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_events_service ON transaction_events(service_name, occurred_at DESC);
CREATE INDEX IF NOT EXISTS idx_events_type ON transaction_events(event_type);

COMMENT ON TABLE transaction_events IS 'Complete audit log of all service decisions and events for each transaction';
COMMENT ON COLUMN transaction_events.service_name IS 'Name of the microservice that generated the event';
COMMENT ON COLUMN transaction_events.payload IS 'Full event data in JSON format for debugging';

-- 2. Transaction Decisions Aggregate (центральная таблица решений)
CREATE TABLE IF NOT EXISTS transaction_decisions (
    transaction_id UUID PRIMARY KEY REFERENCES transactions(id),

    -- Compliance Engine decision
    compliance_status VARCHAR(20), -- 'Approved', 'Hold', 'ReviewRequired', 'Rejected'
    compliance_risk_rating VARCHAR(20), -- 'Low', 'Medium', 'High', 'VeryHigh', 'Prohibited'
    compliance_required_actions TEXT[], -- ['EnhancedDueDiligence', 'FileSAR', etc.]
    compliance_checked_at TIMESTAMPTZ,

    -- Risk Engine decision
    risk_decision VARCHAR(20), -- 'Approve', 'Review', 'Reject'
    risk_score NUMERIC(5,2), -- 0.00 - 100.00
    risk_confidence NUMERIC(3,2), -- 0.00 - 1.00
    risk_factors JSONB,
    risk_evaluated_at TIMESTAMPTZ,

    -- Token Engine balance check
    token_balance_sufficient BOOLEAN,
    token_balance_available NUMERIC(20,2),
    token_balance_required NUMERIC(20,2),
    token_checked_at TIMESTAMPTZ,

    -- Liquidity Router prediction
    liquidity_can_instant_settle BOOLEAN,
    liquidity_recommendation VARCHAR(50), -- 'INSTANT', 'BATCH_CLEARING', 'DEFER'
    liquidity_predicted_at TIMESTAMPTZ,

    -- Clearing Engine status
    clearing_batch_id UUID,
    clearing_status VARCHAR(30), -- 'Pending', 'Queued', 'Netted', 'Completed'
    clearing_netting_amount NUMERIC(20,2),
    clearing_processed_at TIMESTAMPTZ,

    -- Settlement Engine status
    settlement_instruction_id UUID,
    settlement_status VARCHAR(30), -- 'Pending', 'InProgress', 'Settled', 'Failed'
    settlement_settled_at TIMESTAMPTZ,

    -- Aggregated final decision
    final_decision VARCHAR(40) NOT NULL DEFAULT 'PENDING',
    decision_reason TEXT,
    decided_at TIMESTAMPTZ,

    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_decisions_final ON transaction_decisions(final_decision);
CREATE INDEX IF NOT EXISTS idx_decisions_updated ON transaction_decisions(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_decisions_compliance ON transaction_decisions(compliance_status);
CREATE INDEX IF NOT EXISTS idx_decisions_risk ON transaction_decisions(risk_decision);

COMMENT ON TABLE transaction_decisions IS 'Aggregated decisions from all services for each transaction';
COMMENT ON COLUMN transaction_decisions.final_decision IS 'Computed final status: REJECTED_*, PENDING_REVIEW, APPROVED_*, SETTLED, etc.';

-- 3. Function to compute final decision
CREATE OR REPLACE FUNCTION compute_final_decision(
    p_compliance_status VARCHAR,
    p_risk_decision VARCHAR,
    p_token_sufficient BOOLEAN,
    p_settlement_status VARCHAR
) RETURNS VARCHAR AS $$
BEGIN
    -- Priority 1: Compliance rejection
    IF p_compliance_status = 'Rejected' THEN
        RETURN 'REJECTED_COMPLIANCE';
    END IF;

    -- Priority 2: Risk rejection
    IF p_risk_decision = 'Reject' THEN
        RETURN 'REJECTED_RISK';
    END IF;

    -- Priority 3: Insufficient funds
    IF p_token_sufficient = FALSE THEN
        RETURN 'REJECTED_INSUFFICIENT_FUNDS';
    END IF;

    -- Priority 4: Manual review required
    IF p_compliance_status IN ('Hold', 'ReviewRequired') OR p_risk_decision = 'Review' THEN
        RETURN 'PENDING_REVIEW';
    END IF;

    -- Priority 5: Settlement status
    IF p_settlement_status = 'Settled' THEN
        RETURN 'SETTLED';
    ELSIF p_settlement_status IN ('InProgress', 'Pending') THEN
        RETURN 'SETTLEMENT_IN_PROGRESS';
    ELSIF p_settlement_status = 'Failed' THEN
        RETURN 'SETTLEMENT_FAILED';
    END IF;

    -- Priority 6: Approved, pending settlement
    IF p_compliance_status = 'Approved' AND p_risk_decision = 'Approve' AND p_token_sufficient = TRUE THEN
        RETURN 'APPROVED_PENDING_SETTLEMENT';
    END IF;

    -- Default
    RETURN 'PROCESSING';
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- 4. Trigger to auto-update final decision
CREATE OR REPLACE FUNCTION update_transaction_decision_trigger()
RETURNS TRIGGER AS $$
BEGIN
    NEW.final_decision := compute_final_decision(
        NEW.compliance_status,
        NEW.risk_decision,
        NEW.token_balance_sufficient,
        NEW.settlement_status
    );

    NEW.decision_reason := CASE
        WHEN NEW.final_decision LIKE 'REJECTED_%' THEN
            'Transaction rejected: ' || REPLACE(NEW.final_decision, 'REJECTED_', '')
        WHEN NEW.final_decision = 'PENDING_REVIEW' THEN
            'Manual review required by ' ||
            CASE
                WHEN NEW.compliance_status IN ('Hold', 'ReviewRequired') THEN 'compliance'
                WHEN NEW.risk_decision = 'Review' THEN 'risk'
                ELSE 'unknown service'
            END
        WHEN NEW.final_decision = 'SETTLED' THEN
            'Transaction successfully settled'
        ELSE
            'Transaction in progress'
    END;

    NEW.updated_at := NOW();

    IF NEW.final_decision IN ('SETTLED', 'REJECTED_COMPLIANCE', 'REJECTED_RISK', 'REJECTED_INSUFFICIENT_FUNDS') THEN
        NEW.decided_at := NOW();
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_transaction_decision
    BEFORE INSERT OR UPDATE ON transaction_decisions
    FOR EACH ROW
    EXECUTE FUNCTION update_transaction_decision_trigger();

-- 5. Materialized View for real-time aggregation
CREATE MATERIALIZED VIEW IF NOT EXISTS transaction_state_aggregation AS
SELECT
    t.id AS transaction_id,
    t.sender_bank_id,
    t.receiver_bank_id,
    t.amount,
    t.sent_currency,
    t.received_currency,
    t.status AS transaction_status,

    -- Service decisions
    td.compliance_status,
    td.compliance_risk_rating,
    td.risk_decision,
    td.risk_score,
    td.risk_confidence,
    td.token_balance_sufficient,
    td.liquidity_can_instant_settle,
    td.clearing_status,
    td.settlement_status,

    -- Aggregated decision
    td.final_decision,
    td.decision_reason,

    -- Timestamps
    t.created_at,
    td.compliance_checked_at,
    td.risk_evaluated_at,
    td.clearing_processed_at,
    td.settlement_settled_at,
    td.updated_at AS last_updated

FROM transactions t
LEFT JOIN transaction_decisions td ON t.id = td.transaction_id;

-- Indexes for the materialized view
CREATE UNIQUE INDEX IF NOT EXISTS idx_tsa_transaction_id ON transaction_state_aggregation(transaction_id);
CREATE INDEX IF NOT EXISTS idx_tsa_final_decision ON transaction_state_aggregation(final_decision);
CREATE INDEX IF NOT EXISTS idx_tsa_updated ON transaction_state_aggregation(last_updated DESC);
CREATE INDEX IF NOT EXISTS idx_tsa_sender ON transaction_state_aggregation(sender_bank_id);
CREATE INDEX IF NOT EXISTS idx_tsa_receiver ON transaction_state_aggregation(receiver_bank_id);

COMMENT ON MATERIALIZED VIEW transaction_state_aggregation IS 'Real-time aggregated view of transaction states across all services';

-- 6. Function to refresh materialized view (call periodically)
CREATE OR REPLACE FUNCTION refresh_transaction_state_aggregation()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY transaction_state_aggregation;
END;
$$ LANGUAGE plpgsql;

-- 7. Helper function to get complete transaction state
CREATE OR REPLACE FUNCTION get_transaction_complete_state(p_transaction_id UUID)
RETURNS JSON AS $$
DECLARE
    v_result JSON;
BEGIN
    SELECT json_build_object(
        'transaction_id', tsa.transaction_id,
        'sender_bank_id', tsa.sender_bank_id,
        'receiver_bank_id', tsa.receiver_bank_id,
        'amount', tsa.amount,
        'currency', json_build_object(
            'sent', tsa.sent_currency,
            'received', tsa.received_currency
        ),
        'status', json_build_object(
            'transaction', tsa.transaction_status,
            'aggregated', tsa.final_decision,
            'reason', tsa.decision_reason
        ),
        'decisions', json_build_object(
            'compliance', json_build_object(
                'status', tsa.compliance_status,
                'risk_rating', tsa.compliance_risk_rating,
                'checked_at', tsa.compliance_checked_at
            ),
            'risk', json_build_object(
                'decision', tsa.risk_decision,
                'score', tsa.risk_score,
                'confidence', tsa.risk_confidence,
                'evaluated_at', tsa.risk_evaluated_at
            ),
            'token', json_build_object(
                'sufficient_balance', tsa.token_balance_sufficient
            ),
            'liquidity', json_build_object(
                'can_instant_settle', tsa.liquidity_can_instant_settle
            ),
            'clearing', json_build_object(
                'status', tsa.clearing_status,
                'processed_at', tsa.clearing_processed_at
            ),
            'settlement', json_build_object(
                'status', tsa.settlement_status,
                'settled_at', tsa.settlement_settled_at
            )
        ),
        'timeline', json_build_object(
            'created', tsa.created_at,
            'compliance_checked', tsa.compliance_checked_at,
            'risk_evaluated', tsa.risk_evaluated_at,
            'clearing_processed', tsa.clearing_processed_at,
            'settled', tsa.settlement_settled_at,
            'last_updated', tsa.last_updated
        )
    ) INTO v_result
    FROM transaction_state_aggregation tsa
    WHERE tsa.transaction_id = p_transaction_id;

    RETURN v_result;
END;
$$ LANGUAGE plpgsql;

COMMENT ON FUNCTION get_transaction_complete_state IS 'Returns complete transaction state with all service decisions in JSON format';

-- 8. Cleanup old events (retention policy)
CREATE OR REPLACE FUNCTION cleanup_old_transaction_events()
RETURNS VOID AS $$
BEGIN
    -- Keep only last 90 days of events
    DELETE FROM transaction_events
    WHERE occurred_at < NOW() - INTERVAL '90 days';
END;
$$ LANGUAGE plpgsql;
