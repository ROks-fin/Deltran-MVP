-- Migration: Create funding_events table
-- Stores confirmed funding events (matched transactions)

CREATE TABLE IF NOT EXISTS funding_events (
    id UUID PRIMARY KEY,
    payment_id UUID NOT NULL,
    transaction_id VARCHAR(255) NOT NULL UNIQUE,
    account_id VARCHAR(100) NOT NULL,
    amount DECIMAL(20, 4) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    end_to_end_id VARCHAR(255),
    debtor_name VARCHAR(255),
    debtor_account VARCHAR(100),
    booking_date TIMESTAMP,
    value_date TIMESTAMP,
    confirmed_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),

    -- Indexes
    INDEX idx_funding_payment_id (payment_id),
    INDEX idx_funding_transaction_id (transaction_id),
    INDEX idx_funding_account_id (account_id),
    INDEX idx_funding_confirmed_at (confirmed_at)
);

COMMENT ON TABLE funding_events IS 'Confirmed funding events when real FIAT arrives on EMI accounts';
COMMENT ON COLUMN funding_events.payment_id IS 'Associated payment ID from obligation/settlement';
COMMENT ON COLUMN funding_events.transaction_id IS 'Bank transaction ID from camt.054 or API';
COMMENT ON COLUMN funding_events.end_to_end_id IS 'ISO 20022 end-to-end reference for matching';
COMMENT ON COLUMN funding_events.confirmed_at IS 'When the funding was confirmed and published to Token Engine';
