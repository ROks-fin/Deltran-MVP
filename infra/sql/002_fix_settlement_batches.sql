-- DelTran Rail MVP - Fix Settlement Batches Column Name
-- Version: 002
-- Description: Safely rename 'window' column to 'batch_window' in settlement_batches table
-- This migration is safe for both fresh installations and existing databases

-- Check if settlement_batches table exists
DO $$
BEGIN
    -- If table doesn't exist, create it with the correct schema
    IF NOT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'settlement_batches') THEN
        CREATE TABLE settlement_batches (
            batch_id UUID PRIMARY KEY,
            batch_window VARCHAR(20) NOT NULL,
            total_transactions INTEGER NOT NULL DEFAULT 0,
            total_amount DECIMAL(20,2) NOT NULL DEFAULT 0,
            net_positions JSONB,
            status VARCHAR(20) DEFAULT 'OPEN',
            closed_at TIMESTAMP WITH TIME ZONE,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

            CONSTRAINT valid_batch_window CHECK (batch_window IN ('intraday', 'EOD')),
            CONSTRAINT valid_status CHECK (status IN ('OPEN', 'CLOSED', 'SETTLED'))
        );
        
        -- Create indexes
        CREATE INDEX idx_settlement_batches_batch_window ON settlement_batches(batch_window);
        CREATE INDEX idx_settlement_batches_closed_at ON settlement_batches(closed_at);
        CREATE INDEX idx_settlement_batches_status ON settlement_batches(status);
        
    ELSE
        -- Table exists, check if we need to rename the column
        IF EXISTS (SELECT FROM information_schema.columns 
                  WHERE table_name = 'settlement_batches' AND column_name = 'window') THEN
            
            -- Rename the column from 'window' to 'batch_window'
            ALTER TABLE settlement_batches RENAME COLUMN window TO batch_window;
            
            -- Update the constraint name
            ALTER TABLE settlement_batches DROP CONSTRAINT IF EXISTS valid_window;
            ALTER TABLE settlement_batches ADD CONSTRAINT valid_batch_window 
                CHECK (batch_window IN ('intraday', 'EOD'));
            
            -- Update index name
            DROP INDEX IF EXISTS idx_settlement_batches_window;
            CREATE INDEX idx_settlement_batches_batch_window ON settlement_batches(batch_window);
        END IF;
    END IF;
END $$;

-- Create compatibility view for backward compatibility
-- This allows existing code to continue working with 'window' column name
CREATE OR REPLACE VIEW settlement_batches_compat AS
SELECT 
    batch_id,
    batch_window AS window,  -- Alias for backward compatibility
    total_transactions,
    total_amount,
    net_positions,
    status,
    closed_at,
    created_at
FROM settlement_batches;

-- Insert migration record
INSERT INTO schema_migrations (version) VALUES ('002_fix_settlement_batches')
ON CONFLICT (version) DO NOTHING;
