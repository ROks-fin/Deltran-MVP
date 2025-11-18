# 🚀 DelTran MVP - Quick Start Guide

## Prerequisites

- Docker Desktop installed and running
- Rust 1.70+ installed (`rustup` recommended)
- PostgreSQL client tools (psql)
- NATS CLI tools (optional, for stream management)

---

## Step 1: Start Infrastructure

```bash
# Start PostgreSQL
docker run -d --name deltran-postgres \
  -p 5432:5432 \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=deltran2025 \
  -e POSTGRES_DB=deltran \
  postgres:14

# Start NATS with JetStream
docker run -d --name deltran-nats \
  -p 4222:4222 \
  -p 8222:8222 \
  nats:latest -js -m 8222

# Verify NATS is running
curl http://localhost:8222/varz
```

---

## Step 2: Database Setup

```bash
# Navigate to project root
cd "C:\Users\User\Desktop\Deltran MVP"

# Apply migrations
psql -h localhost -U postgres -d deltran -f infrastructure/database/migrations/001-initial-schema.sql
psql -h localhost -U postgres -d deltran -f infrastructure/database/migrations/002-emi-accounts.sql

# Verify tables created
psql -h localhost -U postgres -d deltran -c "\dt"
```

**Expected Output:**
```
                    List of relations
 Schema |              Name              | Type  |  Owner
--------+--------------------------------+-------+----------
 public | atomic_operations              | table | postgres
 public | banks                          | table | postgres
 public | clearing_metrics               | table | postgres
 public | clearing_windows               | table | postgres
 public | emi_account_snapshots          | table | postgres
 public | emi_accounts                   | table | postgres
 public | emi_transactions               | table | postgres
 public | net_positions                  | table | postgres
 public | obligations                    | table | postgres
 public | operation_checkpoints          | table | postgres
 public | reconciliation_discrepancies   | table | postgres
 public | reserve_buffer_calculations    | table | postgres
 public | settlement_instructions        | table | postgres
 public | window_events                  | table | postgres
 public | window_locks                   | table | postgres
```

---

## Step 3: Insert Test Data

```sql
-- Connect to database
psql -h localhost -U postgres -d deltran

-- Create test banks
INSERT INTO banks (id, bank_code, bank_name, swift_bic, country_code, region, status)
VALUES
    ('11111111-1111-1111-1111-111111111111', 'BANK_UAE_001', 'UAE Bank One', 'UAEBAE21', 'ARE', 'ADGM', 'ACTIVE'),
    ('22222222-2222-2222-2222-222222222222', 'BANK_UAE_002', 'UAE Bank Two', 'UAEBAE22', 'ARE', 'ADGM', 'ACTIVE'),
    ('33333333-3333-3333-3333-333333333333', 'BANK_IND_001', 'India Bank One', 'INDBINBB', 'IND', 'AsiaPacific', 'ACTIVE'),
    ('44444444-4444-4444-4444-444444444444', 'BANK_IND_002', 'India Bank Two', 'INDBINBC', 'IND', 'AsiaPacific', 'ACTIVE');

-- Verify
SELECT bank_code, bank_name, swift_bic FROM banks;

-- Create EMI accounts
INSERT INTO emi_accounts (
    id, bank_id, account_number, iban, swift_bic, currency, country_code, account_type,
    ledger_balance, bank_reported_balance, reserved_balance
)
VALUES
    (uuid_generate_v4(), '11111111-1111-1111-1111-111111111111', 'UAE001CLIENT', 'AE070331234567890123456', 'UAEBAE21', 'USD', 'ARE', 'client_funds', 1000000.00, 1000000.00, 0.00),
    (uuid_generate_v4(), '22222222-2222-2222-2222-222222222222', 'UAE002CLIENT', 'AE070331234567890123457', 'UAEBAE22', 'USD', 'ARE', 'client_funds', 500000.00, 500000.00, 0.00),
    (uuid_generate_v4(), '33333333-3333-3333-3333-333333333333', 'IND001CLIENT', 'IN0123456789012345', 'INDBINBB', 'INR', 'IND', 'client_funds', 50000000.00, 50000000.00, 0.00);

-- Verify
SELECT bank_id, account_number, currency, ledger_balance, available_balance FROM emi_accounts;

\q
```

---

## Step 4: Build Clearing Engine

```bash
# Navigate to clearing engine
cd services/clearing-engine

# Build in release mode
cargo build --release

# Expected output: Compiling... (may take 3-5 minutes first time)
```

---

## Step 5: Configure Environment

```bash
# Create .env file
cat > .env << 'EOF'
DATABASE_URL=postgresql://postgres:deltran2025@localhost:5432/deltran
NATS_URL=nats://localhost:4222
SERVICE_PORT=8085
RUST_LOG=info,clearing_engine=debug
CLEARING_SCHEDULE=0 0,6,12,18 * * *
GRACE_PERIOD_MINUTES=30
WINDOW_DURATION_HOURS=6
REGION=Global
EOF

# Verify
cat .env
```

---

## Step 6: Run Clearing Engine

```bash
# Run the service
cargo run --release

# Expected output:
# 2025-11-17T10:00:00.000Z INFO clearing_engine: 🚀 Clearing Engine starting on port 8085
# 2025-11-17T10:00:00.001Z INFO clearing_engine::window::scheduler: Starting clearing window scheduler
# 2025-11-17T10:00:00.002Z INFO clearing_engine::window::scheduler: Window scheduler started successfully
```

**Service will start and:**
1. Connect to PostgreSQL
2. Connect to NATS
3. Start window scheduler
4. Listen on port 8085

---

## Step 7: Test the API

### Health Check
```bash
curl http://localhost:8085/health

# Expected:
{
  "status": "healthy",
  "service": "clearing-engine",
  "version": "0.1.0"
}
```

### Get Current Window
```bash
curl http://localhost:8085/api/v1/clearing/windows/current

# Expected:
{
  "window": {
    "id": 1,
    "window_name": "CLEAR_Global_20251117_1000",
    "status": "Open",
    "start_time": "2025-11-17T10:00:00Z",
    "end_time": "2025-11-17T16:00:00Z",
    ...
  }
}
```

### Get Metrics
```bash
curl http://localhost:8085/api/v1/clearing/metrics

# Expected:
{
  "total_windows": 1,
  "active_windows": 1,
  "netting_efficiency": 0.0
}
```

### Prometheus Metrics
```bash
curl http://localhost:8085/metrics

# Expected:
# HELP clearing_engine_up Service is running
# TYPE clearing_engine_up gauge
clearing_engine_up 1
```

---

## Step 8: Create Test Clearing Cycle

```sql
-- Connect to database
psql -h localhost -U postgres -d deltran

-- Manually create a window for testing
INSERT INTO clearing_windows (
    window_name, start_time, end_time, cutoff_time, status, region, grace_period_seconds
)
VALUES (
    'TEST_WINDOW_001',
    NOW(),
    NOW() + INTERVAL '6 hours',
    NOW() + INTERVAL '5 hours 30 minutes',
    'OPEN',
    'Global',
    1800
)
RETURNING id;

-- Note the window ID (e.g., 2)

-- Create test obligations
DO $$
DECLARE
    window_id BIGINT := 2; -- Use the ID from above
BEGIN
    -- UAE1 owes UAE2: $10,000
    INSERT INTO obligations (id, window_id, payer_id, payee_id, amount, currency, status)
    VALUES (uuid_generate_v4(), window_id, '11111111-1111-1111-1111-111111111111', '22222222-2222-2222-2222-222222222222', 10000.00, 'USD', 'PENDING');

    -- UAE2 owes UAE1: $3,000
    INSERT INTO obligations (id, window_id, payer_id, payee_id, amount, currency, status)
    VALUES (uuid_generate_v4(), window_id, '22222222-2222-2222-2222-222222222222', '11111111-1111-1111-1111-111111111111', 3000.00, 'USD', 'PENDING');

    -- IND1 owes IND2: ₹500,000
    INSERT INTO obligations (id, window_id, payer_id, payee_id, amount, currency, status)
    VALUES (uuid_generate_v4(), window_id, '33333333-3333-3333-3333-333333333333', '44444444-4444-4444-4444-444444444444', 500000.00, 'INR', 'PENDING');

    -- IND2 owes IND1: ₹200,000
    INSERT INTO obligations (id, window_id, payer_id, payee_id, amount, currency, status)
    VALUES (uuid_generate_v4(), window_id, '44444444-4444-4444-4444-444444444444', '33333333-3333-3333-3333-333333333333', 200000.00, 'INR', 'PENDING');
END $$;

-- Verify
SELECT
    o.id,
    pb.bank_code as payer,
    rb.bank_code as payee,
    o.amount,
    o.currency
FROM obligations o
JOIN banks pb ON o.payer_id = pb.id
JOIN banks rb ON o.payee_id = rb.id
WHERE o.window_id = 2;

-- Update window status to PROCESSING
UPDATE clearing_windows SET status = 'PROCESSING' WHERE id = 2;

\q
```

---

## Step 9: Run Manual Clearing

```bash
# Run clearing for test window
# (This would typically be triggered by the scheduler or an API call)

# Option 1: Via API (if clearing endpoint implemented)
curl -X POST http://localhost:8085/api/v1/clearing/execute \
  -H "Content-Type: application/json" \
  -d '{"window_id": 2}'

# Option 2: Trigger via SQL (update status will trigger processing in next cycle)
psql -h localhost -U postgres -d deltran -c \
  "UPDATE clearing_windows SET status = 'PROCESSING' WHERE id = 2;"
```

---

## Step 10: Verify Results

```sql
-- Connect to database
psql -h localhost -U postgres -d deltran

-- Check net positions
SELECT
    np.id,
    ba.bank_code as bank_a,
    bb.bank_code as bank_b,
    np.currency,
    np.gross_debit_a_to_b,
    np.gross_credit_b_to_a,
    np.net_amount,
    np.amount_saved,
    np.netting_ratio
FROM net_positions np
JOIN banks ba ON np.bank_a_id = ba.id
JOIN banks bb ON np.bank_b_id = bb.id
WHERE np.window_id = 2;

-- Expected results:
-- USD: NET $7,000 (UAE1 owes UAE2) - saved $6,000
-- INR: NET ₹300,000 (IND1 owes IND2) - saved ₹400,000

-- Check settlement instructions
SELECT
    si.id,
    pb.bank_code as payer,
    rb.bank_code as receiver,
    si.amount,
    si.currency,
    si.status
FROM settlement_instructions si
JOIN banks pb ON si.payer_bank_id = pb.id
JOIN banks rb ON si.payee_bank_id = rb.id
WHERE si.window_id = 2;

-- Check window metrics
SELECT
    window_name,
    obligations_count,
    total_gross_value,
    total_net_value,
    saved_amount,
    netting_efficiency,
    status
FROM clearing_windows
WHERE id = 2;

\q
```

---

## Step 11: Monitor with NATS

```bash
# Subscribe to clearing events
nats sub "clearing.events.>"

# In another terminal, trigger clearing
# You should see events like:
# [#1] Received on "clearing.events.completed"
# {"event_type":"clearing.completed","window_id":2,"timestamp":"2025-11-17T10:30:00Z"}
```

---

## Troubleshooting

### Issue: Database connection failed

```bash
# Check PostgreSQL is running
docker ps | grep postgres

# Check connection
psql -h localhost -U postgres -d deltran -c "SELECT 1;"

# Check logs
docker logs deltran-postgres
```

### Issue: NATS connection failed

```bash
# Check NATS is running
docker ps | grep nats

# Check JetStream enabled
curl http://localhost:8222/varz | grep jetstream

# Check NATS logs
docker logs deltran-nats
```

### Issue: Compilation errors

```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

### Issue: Missing dependencies

```bash
# Check Cargo.toml
cat services/clearing-engine/Cargo.toml

# Update dependencies
cargo update
```

---

## Next Steps

1. **Review Implementation Guide:** See [IMPLEMENTATION_GUIDE.md](IMPLEMENTATION_GUIDE.md)
2. **Run Tests:** `cargo test`
3. **Load Testing:** Use K6 scripts in `tests/load/`
4. **Configure for Production:** Update `.env` with production values
5. **Enable Monitoring:** Integrate Prometheus & Grafana

---

## Quick Reference

| Component | Port | URL |
|-----------|------|-----|
| Clearing Engine | 8085 | http://localhost:8085 |
| PostgreSQL | 5432 | postgresql://localhost:5432/deltran |
| NATS | 4222 | nats://localhost:4222 |
| NATS Monitor | 8222 | http://localhost:8222 |

### Useful Commands

```bash
# View logs
cargo run --release 2>&1 | tee clearing-engine.log

# Database console
psql -h localhost -U postgres -d deltran

# NATS console
nats -s localhost:4222

# Stop all
docker stop deltran-postgres deltran-nats

# Clean up
docker rm deltran-postgres deltran-nats
```

---

**Happy Clearing! 🎉**
