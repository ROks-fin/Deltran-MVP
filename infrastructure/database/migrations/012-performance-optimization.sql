-- Migration 012: Performance Optimization for Obligation and Clearing Engines
-- Optimizes database queries for high-throughput clearing and netting operations

-- ============================================================================
-- PART 1: COMPOSITE INDEXES FOR OBLIGATION ENGINE
-- ============================================================================

-- Fast lookup of obligations by status and clearing window
CREATE INDEX IF NOT EXISTS idx_obligations_status_window
    ON obligations(status, clearing_window_id)
    WHERE status IN ('Pending', 'Netted');

-- Bilateral netting lookup (bank pair + currency + window)
CREATE INDEX IF NOT EXISTS idx_obligations_bilateral_netting
    ON obligations(debtor_bank_id, creditor_bank_id, currency, clearing_window_id)
    WHERE status = 'Pending';

-- Reverse bilateral lookup for netting
CREATE INDEX IF NOT EXISTS idx_obligations_reverse_bilateral
    ON obligations(creditor_bank_id, debtor_bank_id, currency, clearing_window_id)
    WHERE status = 'Pending';

-- Multilateral netting - all pending obligations in a window
CREATE INDEX IF NOT EXISTS idx_obligations_multilateral_netting
    ON obligations(clearing_window_id, currency, status)
    WHERE status = 'Pending';

-- ============================================================================
-- PART 2: COMPOSITE INDEXES FOR CLEARING WINDOWS
-- ============================================================================

-- Active clearing windows lookup (using actual column names)
CREATE INDEX IF NOT EXISTS idx_clearing_windows_active
    ON clearing_windows(status, start_time)
    WHERE status IN ('Scheduled', 'Open', 'InProgress');

-- Historical clearing window analytics
CREATE INDEX IF NOT EXISTS idx_clearing_windows_completed
    ON clearing_windows(status, completed_at DESC)
    WHERE status = 'Completed';

-- ============================================================================
-- PART 3: PARTITIONING FOR OBLIGATIONS TABLE (Time-series optimization)
-- ============================================================================

-- Note: Partitioning requires rebuilding the table, so we create functions first
-- Create partitioned obligations table for better performance with high volume

-- Function to create monthly partitions automatically
CREATE OR REPLACE FUNCTION create_obligation_partition(partition_date DATE)
RETURNS VOID AS $$
DECLARE
    partition_name TEXT;
    start_date DATE;
    end_date DATE;
BEGIN
    partition_name := 'obligations_y' || to_char(partition_date, 'YYYY') ||
                      'm' || to_char(partition_date, 'MM');
    start_date := date_trunc('month', partition_date);
    end_date := start_date + interval '1 month';

    EXECUTE format('CREATE TABLE IF NOT EXISTS %I PARTITION OF obligations
                    FOR VALUES FROM (%L) TO (%L)',
                   partition_name, start_date, end_date);
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 4: MATERIALIZED VIEW FOR NETTING CALCULATIONS
-- ============================================================================

-- Pre-calculate bilateral net positions for faster netting
CREATE MATERIALIZED VIEW IF NOT EXISTS bilateral_net_positions AS
SELECT
    o1.clearing_window_id,
    o1.currency,
    LEAST(o1.debtor_bank_id, o1.creditor_bank_id) AS bank_a_id,
    GREATEST(o1.debtor_bank_id, o1.creditor_bank_id) AS bank_b_id,
    COALESCE(SUM(CASE
        WHEN o1.debtor_bank_id < o1.creditor_bank_id THEN o1.amount
        ELSE -o1.amount
    END), 0) AS net_amount,
    COUNT(*) AS obligation_count,
    ARRAY_AGG(o1.id) AS obligation_ids
FROM obligations o1
WHERE o1.status = 'Pending'
GROUP BY o1.clearing_window_id, o1.currency,
         LEAST(o1.debtor_bank_id, o1.creditor_bank_id),
         GREATEST(o1.debtor_bank_id, o1.creditor_bank_id);

-- Index for fast bilateral net position lookup
CREATE INDEX IF NOT EXISTS idx_bilateral_net_positions_window
    ON bilateral_net_positions(clearing_window_id, currency);

-- Refresh function for the materialized view
CREATE OR REPLACE FUNCTION refresh_bilateral_net_positions()
RETURNS TRIGGER AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY bilateral_net_positions;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Auto-refresh trigger when obligations change
CREATE TRIGGER trigger_refresh_net_positions
    AFTER INSERT OR UPDATE OR DELETE ON obligations
    FOR EACH STATEMENT
    EXECUTE FUNCTION refresh_bilateral_net_positions();

-- ============================================================================
-- PART 5: QUERY OPTIMIZATION STATISTICS
-- ============================================================================

-- Analyze tables to update statistics for query planner
ANALYZE obligations;
ANALYZE clearing_windows;
ANALYZE settlement_instructions;

-- ============================================================================
-- PART 6: CONNECTION POOLING CONFIGURATION
-- ============================================================================

-- Increase work_mem for complex netting queries (per-connection)
ALTER DATABASE deltran SET work_mem = '64MB';

-- Increase shared_buffers recommendation (requires PostgreSQL restart)
-- Note: This is a hint - actual configuration in postgresql.conf
-- SHOW shared_buffers; -- Should be ~25% of RAM

-- Enable parallel query execution for large netting operations
ALTER DATABASE deltran SET max_parallel_workers_per_gather = 4;
ALTER DATABASE deltran SET parallel_tuple_cost = 0.01;
ALTER DATABASE deltran SET parallel_setup_cost = 100.0;

-- ============================================================================
-- PART 7: NETTING ALGORITHM OPTIMIZATION FUNCTIONS
-- ============================================================================

-- Fast bilateral netting function
CREATE OR REPLACE FUNCTION calculate_bilateral_netting(
    p_clearing_window_id UUID,
    p_currency VARCHAR(3)
)
RETURNS TABLE (
    bank_a_id UUID,
    bank_b_id UUID,
    net_amount NUMERIC(20, 2),
    net_direction VARCHAR(10), -- 'A_TO_B' or 'B_TO_A'
    gross_obligations_count INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        bnp.bank_a_id,
        bnp.bank_b_id,
        ABS(bnp.net_amount) AS net_amount,
        CASE
            WHEN bnp.net_amount > 0 THEN 'A_TO_B'
            WHEN bnp.net_amount < 0 THEN 'B_TO_A'
            ELSE 'BALANCED'
        END AS net_direction,
        bnp.obligation_count::INTEGER AS gross_obligations_count
    FROM bilateral_net_positions bnp
    WHERE bnp.clearing_window_id = p_clearing_window_id
      AND bnp.currency = p_currency
      AND ABS(bnp.net_amount) > 0.01; -- Ignore negligible amounts
END;
$$ LANGUAGE plpgsql STABLE PARALLEL SAFE;

-- Multilateral netting preparation (graph-based)
CREATE OR REPLACE FUNCTION prepare_multilateral_netting(
    p_clearing_window_id UUID,
    p_currency VARCHAR(3)
)
RETURNS TABLE (
    bank_id UUID,
    total_payable NUMERIC(20, 2),
    total_receivable NUMERIC(20, 2),
    net_position NUMERIC(20, 2)
) AS $$
BEGIN
    RETURN QUERY
    WITH bank_obligations AS (
        SELECT
            debtor_bank_id AS bank_id,
            SUM(amount) AS total_payable,
            0::NUMERIC(20, 2) AS total_receivable
        FROM obligations
        WHERE clearing_window_id = p_clearing_window_id
          AND currency = p_currency
          AND status = 'Pending'
        GROUP BY debtor_bank_id

        UNION ALL

        SELECT
            creditor_bank_id AS bank_id,
            0::NUMERIC(20, 2) AS total_payable,
            SUM(amount) AS total_receivable
        FROM obligations
        WHERE clearing_window_id = p_clearing_window_id
          AND currency = p_currency
          AND status = 'Pending'
        GROUP BY creditor_bank_id
    )
    SELECT
        bo.bank_id,
        SUM(bo.total_payable) AS total_payable,
        SUM(bo.total_receivable) AS total_receivable,
        SUM(bo.total_receivable) - SUM(bo.total_payable) AS net_position
    FROM bank_obligations bo
    GROUP BY bo.bank_id
    ORDER BY net_position DESC;
END;
$$ LANGUAGE plpgsql STABLE PARALLEL SAFE;

-- ============================================================================
-- PART 8: PERFORMANCE MONITORING VIEW
-- ============================================================================

-- View to monitor clearing performance
CREATE OR REPLACE VIEW clearing_performance_metrics AS
SELECT
    cw.id AS clearing_window_id,
    cw.window_name,
    cw.status,
    cw.start_time,
    cw.end_time,
    cw.completed_at,
    EXTRACT(EPOCH FROM (cw.completed_at - cw.start_time)) AS duration_seconds,
    cw.transactions_count AS total_obligations,
    COUNT(DISTINCT o.currency) AS currencies_cleared,
    cw.total_gross_value,
    cw.total_net_value,
    cw.netting_efficiency,
    COUNT(DISTINCT CASE WHEN o.status = 'Netted' THEN o.id END) AS netted_obligations,
    COUNT(DISTINCT CASE WHEN o.status = 'Settled' THEN o.id END) AS settled_obligations
FROM clearing_windows cw
LEFT JOIN obligations o ON cw.id = o.clearing_window_id
WHERE cw.status IN ('InProgress', 'Completed')
GROUP BY cw.id, cw.window_name, cw.status, cw.start_time,
         cw.end_time, cw.completed_at, cw.transactions_count,
         cw.total_gross_value, cw.total_net_value, cw.netting_efficiency;

-- ============================================================================
-- PART 9: BATCH PROCESSING OPTIMIZATION
-- ============================================================================

-- Function to batch-process obligations for netting
CREATE OR REPLACE FUNCTION batch_process_netting(
    p_clearing_window_id UUID,
    p_batch_size INTEGER DEFAULT 100
)
RETURNS TABLE (
    batch_number INTEGER,
    obligations_processed INTEGER,
    net_positions_created INTEGER
) AS $$
DECLARE
    v_offset INTEGER := 0;
    v_batch INTEGER := 1;
    v_processed INTEGER;
BEGIN
    LOOP
        -- Process batch of obligations
        WITH batch AS (
            SELECT id
            FROM obligations
            WHERE clearing_window_id = p_clearing_window_id
              AND status = 'Pending'
            ORDER BY created_at
            LIMIT p_batch_size
            OFFSET v_offset
        )
        SELECT COUNT(*) INTO v_processed FROM batch;

        EXIT WHEN v_processed = 0;

        -- Return batch info
        RETURN QUERY SELECT v_batch, v_processed, 0;

        v_offset := v_offset + p_batch_size;
        v_batch := v_batch + 1;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- PART 10: COMMENTS
-- ============================================================================

COMMENT ON MATERIALIZED VIEW bilateral_net_positions IS
    'Pre-calculated bilateral net positions for fast netting operations';

COMMENT ON FUNCTION calculate_bilateral_netting IS
    'Calculates bilateral netting between bank pairs for a clearing window';

COMMENT ON FUNCTION prepare_multilateral_netting IS
    'Prepares multilateral netting by calculating each bank''s net position';

COMMENT ON FUNCTION batch_process_netting IS
    'Processes obligations in batches to avoid memory issues with large volumes';

COMMENT ON VIEW clearing_performance_metrics IS
    'Real-time monitoring of clearing window performance and netting efficiency';
