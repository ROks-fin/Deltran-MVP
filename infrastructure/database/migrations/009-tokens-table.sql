-- Migration 009: Tokens Table for Token Engine
-- Creates the tokens table to track tokenized fiat currencies

-- Tokens table
CREATE TABLE IF NOT EXISTS tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    currency VARCHAR(10) NOT NULL, -- e.g., 'xUSD', 'xINR', 'xAED'
    amount NUMERIC(20, 2) NOT NULL CHECK (amount >= 0),
    bank_id UUID NOT NULL REFERENCES banks(id),
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE',
    clearing_window BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    burned_at TIMESTAMPTZ,
    reference VARCHAR(255) NOT NULL, -- Transaction reference for tracking
    metadata JSONB,

    CONSTRAINT valid_status CHECK (status IN ('ACTIVE', 'LOCKED', 'BURNED', 'CONVERTING'))
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_tokens_bank_id ON tokens(bank_id);
CREATE INDEX IF NOT EXISTS idx_tokens_currency ON tokens(currency);
CREATE INDEX IF NOT EXISTS idx_tokens_status ON tokens(status);
CREATE INDEX IF NOT EXISTS idx_tokens_reference ON tokens(reference);
CREATE INDEX IF NOT EXISTS idx_tokens_created_at ON tokens(created_at);

-- Composite index for common queries
CREATE INDEX IF NOT EXISTS idx_tokens_bank_currency_status
    ON tokens(bank_id, currency, status);

-- Add comments
COMMENT ON TABLE tokens IS 'Tokenized fiat currency tracking for banks';
COMMENT ON COLUMN tokens.currency IS 'Tokenized currency code (xUSD, xINR, xAED, etc.)';
COMMENT ON COLUMN tokens.amount IS 'Amount of tokenized currency';
COMMENT ON COLUMN tokens.bank_id IS 'Bank that owns this token';
COMMENT ON COLUMN tokens.status IS 'Current status: ACTIVE, LOCKED, BURNED, CONVERTING';
COMMENT ON COLUMN tokens.clearing_window IS 'Clearing window identifier for settlement';
COMMENT ON COLUMN tokens.reference IS 'Transaction reference for auditability';
