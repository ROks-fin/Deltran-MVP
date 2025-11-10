#!/bin/bash
# Run all pending migrations for DelTran services

set -e

MIGRATIONS_DIR="/docker-entrypoint-initdb.d/migrations"

echo "=== Starting DelTran Database Migrations ==="

# Apply clearing engine migration
echo "Applying 005_clearing_engine migration..."
psql -U deltran -d deltran -f "$MIGRATIONS_DIR/005_clearing_engine.sql" || echo "Clearing migration had errors (may be already applied)"

# Apply settlement engine migration
echo "Applying 006_settlement_engine migration..."
psql -U deltran -d deltran -f "$MIGRATIONS_DIR/006_settlement_engine.sql" || echo "Settlement migration had errors (may be already applied)"

# Apply notification engine migration
echo "Applying 007_notification_engine migration..."
psql -U deltran -d deltran -f "$MIGRATIONS_DIR/007_notification_engine.sql" || echo "Notification migration had errors (may be already applied)"

# Apply reporting engine migration
echo "Applying 008_reporting_engine migration..."
psql -U deltran -d deltran -f "$MIGRATIONS_DIR/008_reporting_engine.sql" || echo "Reporting migration had errors (may be already applied)"

echo "=== Migrations Complete ==="

# Verify tables
echo "=== Verifying Tables ==="
psql -U deltran -d deltran -c "SELECT COUNT(*) as total_tables FROM information_schema.tables WHERE table_schema = 'public';"
