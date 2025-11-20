-- Migration: Create unmatched_transactions table
-- Stores transactions that could not be automatically matched with pending payments

CREATE TABLE IF NOT EXISTS unmatched_transactions (
    id UUID PRIMARY KEY,
    transaction_id VARCHAR(255) NOT NULL UNIQUE,
    account_id VARCHAR(100) NOT NULL,
    amount DECIMAL(20, 4) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    credit_debit_indicator VARCHAR(4) NOT NULL, -- CRDT or DBIT
    end_to_end_id VARCHAR(255),
    debtor_name VARCHAR(255),
    debtor_account VARCHAR(100),
    booking_date TIMESTAMP,
    value_date TIMESTAMP,
    detected_at TIMESTAMP NOT NULL DEFAULT NOW(),
    review_status VARCHAR(20) NOT NULL DEFAULT 'PENDING', -- PENDING, MATCHED, IGNORED
    matched_payment_id UUID,
    matched_at TIMESTAMP,
    matched_by VARCHAR(100),
    notes TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),

    -- Indexes
    INDEX idx_unmatched_transaction_id (transaction_id),
    INDEX idx_unmatched_account_id (account_id),
    INDEX idx_unmatched_review_status (review_status),
    INDEX idx_unmatched_detected_at (detected_at),
    INDEX idx_unmatched_end_to_end_id (end_to_end_id)
);

COMMENT ON TABLE unmatched_transactions IS 'Transactions that could not be automatically matched - require manual review';
COMMENT ON COLUMN unmatched_transactions.review_status IS 'PENDING: awaiting review, MATCHED: manually matched, IGNORED: not a valid payment';
COMMENT ON COLUMN unmatched_transactions.matched_payment_id IS 'Payment ID if manually matched';
COMMENT ON COLUMN unmatched_transactions.matched_by IS 'User who performed manual matching';
