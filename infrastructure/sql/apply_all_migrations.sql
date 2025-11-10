-- Apply all pending migrations for DelTran MVP
-- This script consolidates all service migrations

-- Start transaction
BEGIN;

-- First, check if tables exist to avoid duplicate creation
DO $$
BEGIN
    -- Apply migrations only if tables don't exist
    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'clearing_window_events') THEN
        -- Include clearing engine migration
        \i /migrations/005_clearing_engine.sql
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'settlement_transactions') THEN
        -- Include settlement engine migration
        \i /migrations/006_settlement_engine.sql
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'websocket_connections') THEN
        -- Include notification engine migration
        \i /migrations/007_notification_engine.sql
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'report_schedules') THEN
        -- Include reporting engine migration
        \i /migrations/008_reporting_engine.sql
    END IF;
END $$;

COMMIT;
