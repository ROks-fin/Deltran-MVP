-- Migration 010: Add unique constraint for tokens ON CONFLICT handling
-- This allows UPSERT operations for token transfers

-- Add unique constraint for ON CONFLICT operations
CREATE UNIQUE INDEX IF NOT EXISTS idx_tokens_bank_currency_window
    ON tokens(bank_id, currency, clearing_window)
    WHERE status = 'ACTIVE';

COMMENT ON INDEX idx_tokens_bank_currency_window IS 'Unique index for UPSERT operations during token transfers - only applies to ACTIVE tokens';
