-- Gateway Service - Payments Table
-- Stores all incoming payments in canonical format

CREATE TABLE IF NOT EXISTS payments (
    -- Core identifiers
    deltran_tx_id UUID PRIMARY KEY,
    obligation_id UUID,
    uetr UUID,  -- Universal End-to-End Transaction Reference (ISO 20022)

    -- ISO 20022 identifiers
    end_to_end_id VARCHAR(35) NOT NULL,
    instruction_id VARCHAR(35) NOT NULL,
    message_id VARCHAR(35),

    -- Amounts
    instructed_amount DECIMAL(18, 5) NOT NULL,
    settlement_amount DECIMAL(18, 5) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Parties (simplified for now)
    debtor_name VARCHAR(140),
    debtor_iban VARCHAR(34),
    debtor_account VARCHAR(34),

    creditor_name VARCHAR(140),
    creditor_iban VARCHAR(34),
    creditor_account VARCHAR(34),

    -- Financial institutions
    debtor_agent_bic VARCHAR(11),
    debtor_agent_name VARCHAR(140),

    creditor_agent_bic VARCHAR(11),
    creditor_agent_name VARCHAR(140),

    -- Payment details
    payment_purpose VARCHAR(4),
    remittance_info TEXT,

    -- Status tracking
    status VARCHAR(50) NOT NULL DEFAULT 'Received',

    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    funded_at TIMESTAMP,
    cleared_at TIMESTAMP,
    settled_at TIMESTAMP,
    completed_at TIMESTAMP,

    -- Raw data
    raw_iso_message TEXT,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Indexes for performance
CREATE INDEX idx_payments_status ON payments(status);
CREATE INDEX idx_payments_created_at ON payments(created_at DESC);
CREATE INDEX idx_payments_end_to_end_id ON payments(end_to_end_id);
CREATE INDEX idx_payments_obligation_id ON payments(obligation_id) WHERE obligation_id IS NOT NULL;
CREATE INDEX idx_payments_uetr ON payments(uetr) WHERE uetr IS NOT NULL;
CREATE INDEX idx_payments_debtor_agent ON payments(debtor_agent_bic);
CREATE INDEX idx_payments_creditor_agent ON payments(creditor_agent_bic);
CREATE INDEX idx_payments_currency ON payments(currency);

-- Pending funding monitoring
CREATE INDEX idx_payments_pending_funding ON payments(created_at) WHERE status = 'PendingFunding';

-- Payment events table for audit trail
CREATE TABLE IF NOT EXISTS payment_events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    deltran_tx_id UUID NOT NULL REFERENCES payments(deltran_tx_id),
    event_type VARCHAR(50) NOT NULL,
    event_status VARCHAR(20) NOT NULL,
    event_data JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),

    CONSTRAINT fk_payment FOREIGN KEY (deltran_tx_id) REFERENCES payments(deltran_tx_id) ON DELETE CASCADE
);

CREATE INDEX idx_payment_events_tx_id ON payment_events(deltran_tx_id, created_at DESC);
CREATE INDEX idx_payment_events_type ON payment_events(event_type);
CREATE INDEX idx_payment_events_created_at ON payment_events(created_at DESC);

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_payments_updated_at
    BEFORE UPDATE ON payments
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Comments
COMMENT ON TABLE payments IS 'Canonical payment storage for all ISO 20022 messages';
COMMENT ON COLUMN payments.deltran_tx_id IS 'DelTran internal transaction ID';
COMMENT ON COLUMN payments.uetr IS 'ISO 20022 Universal End-to-End Transaction Reference';
COMMENT ON COLUMN payments.end_to_end_id IS 'ISO 20022 End-to-End Identification';
COMMENT ON COLUMN payments.status IS 'Payment lifecycle status: Received, Validated, PendingFunding, Funded, etc.';
COMMENT ON TABLE payment_events IS 'Audit trail of all payment state changes';
