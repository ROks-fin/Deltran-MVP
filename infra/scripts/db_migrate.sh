#!/bin/bash
# Database Migration Script for DelTran
# Uses Flyway for PostgreSQL migrations

set -e

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Configuration
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-deltran}"
DB_USER="${DB_USER:-deltran_app}"
DB_PASSWORD="${DB_PASSWORD:-changeme123}"
MIGRATION_DIR="./infra/sql"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

echo_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if PostgreSQL is running
check_postgres() {
    echo_info "Checking PostgreSQL connection..."
    if pg_isready -h $DB_HOST -p $DB_PORT -U $DB_USER > /dev/null 2>&1; then
        echo_info "PostgreSQL is ready"
        return 0
    else
        echo_error "PostgreSQL is not running or not accessible"
        return 1
    fi
}

# Wait for PostgreSQL to be ready
wait_for_postgres() {
    echo_info "Waiting for PostgreSQL to be ready..."
    local max_attempts=30
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if pg_isready -h $DB_HOST -p $DB_PORT > /dev/null 2>&1; then
            echo_info "PostgreSQL is ready!"
            return 0
        fi

        echo_warn "Waiting for PostgreSQL... (attempt $attempt/$max_attempts)"
        sleep 2
        attempt=$((attempt + 1))
    done

    echo_error "PostgreSQL did not become ready in time"
    return 1
}

# Create database if not exists
create_database() {
    echo_info "Checking if database '$DB_NAME' exists..."

    PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U postgres -tc \
        "SELECT 1 FROM pg_database WHERE datname = '$DB_NAME'" | grep -q 1 || \
        PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U postgres -c \
        "CREATE DATABASE $DB_NAME"

    echo_info "Database '$DB_NAME' is ready"
}

# Run migrations using Flyway
run_flyway_migrate() {
    echo_info "Running Flyway migrations..."

    docker run --rm \
        --network host \
        -v "$(pwd)/$MIGRATION_DIR:/flyway/sql" \
        -v "$(pwd)/infra/flyway/conf:/flyway/conf" \
        flyway/flyway:9-alpine \
        -url="jdbc:postgresql://$DB_HOST:$DB_PORT/$DB_NAME" \
        -user="$DB_USER" \
        -password="$DB_PASSWORD" \
        -schemas=deltran \
        -locations="filesystem:/flyway/sql" \
        -validateMigrationNaming=true \
        migrate

    echo_info "Migrations completed successfully"
}

# Run migrations using native psql (alternative)
run_psql_migrate() {
    echo_info "Running migrations using psql..."

    for file in $MIGRATION_DIR/*.sql; do
        if [ -f "$file" ]; then
            echo_info "Executing migration: $(basename $file)"
            PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f "$file"
        fi
    done

    echo_info "Migrations completed successfully"
}

# Validate migrations
validate_migrations() {
    echo_info "Validating migrations..."

    docker run --rm \
        --network host \
        -v "$(pwd)/$MIGRATION_DIR:/flyway/sql" \
        flyway/flyway:9-alpine \
        -url="jdbc:postgresql://$DB_HOST:$DB_PORT/$DB_NAME" \
        -user="$DB_USER" \
        -password="$DB_PASSWORD" \
        -schemas=deltran \
        validate

    echo_info "Validation completed successfully"
}

# Show migration info
show_info() {
    echo_info "Showing migration status..."

    docker run --rm \
        --network host \
        flyway/flyway:9-alpine \
        -url="jdbc:postgresql://$DB_HOST:$DB_PORT/$DB_NAME" \
        -user="$DB_USER" \
        -password="$DB_PASSWORD" \
        -schemas=deltran \
        info

    echo_info "Migration info displayed"
}

# Rollback last migration (manual)
rollback_migration() {
    echo_warn "Rolling back last migration..."
    echo_warn "This requires manual SQL execution as Flyway doesn't support automatic rollback"

    # Show last migration
    PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c \
        "SELECT installed_rank, version, description, installed_on
         FROM flyway_schema_history
         ORDER BY installed_rank DESC
         LIMIT 1;"

    echo_warn "Please create a manual rollback script in $MIGRATION_DIR"
}

# Backup database
backup_database() {
    local backup_dir="./backups"
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_file="$backup_dir/deltran_backup_$timestamp.sql"

    mkdir -p $backup_dir

    echo_info "Creating backup: $backup_file"

    PGPASSWORD=$DB_PASSWORD pg_dump -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME \
        --format=custom \
        --file="$backup_file.dump" \
        --verbose

    echo_info "Backup created successfully: $backup_file.dump"
}

# Restore database from backup
restore_database() {
    local backup_file=$1

    if [ -z "$backup_file" ]; then
        echo_error "Please provide backup file path"
        echo "Usage: $0 restore <backup_file>"
        exit 1
    fi

    if [ ! -f "$backup_file" ]; then
        echo_error "Backup file not found: $backup_file"
        exit 1
    fi

    echo_warn "This will restore database from: $backup_file"
    read -p "Are you sure? (yes/no): " confirm

    if [ "$confirm" != "yes" ]; then
        echo_info "Restore cancelled"
        exit 0
    fi

    echo_info "Restoring database..."

    PGPASSWORD=$DB_PASSWORD pg_restore -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME \
        --clean \
        --if-exists \
        --verbose \
        "$backup_file"

    echo_info "Database restored successfully"
}

# Seed test data
seed_test_data() {
    echo_info "Seeding test data..."

    PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME <<EOF
-- Insert test banks
INSERT INTO deltran.banks (bic_code, name, country_code, is_active, risk_rating, kyc_status)
VALUES
    ('CHASUS33XXX', 'JPMorgan Chase Bank', 'US', true, 'LOW', 'verified'),
    ('DEUTDEFFXXX', 'Deutsche Bank AG', 'DE', true, 'LOW', 'verified'),
    ('HSBCHKHHHKH', 'HSBC Hong Kong', 'HK', true, 'LOW', 'verified'),
    ('SBININBBXXX', 'State Bank of India', 'IN', true, 'MEDIUM', 'verified')
ON CONFLICT (bic_code) DO NOTHING;

-- Insert test users
INSERT INTO deltran.users (email, username, password_hash, role, is_active)
VALUES
    ('admin@deltran.local', 'admin', '\$2a\$12\$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5lk3k7nB9jY9i', 'admin', true),
    ('operator@deltran.local', 'operator', '\$2a\$12\$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5lk3k7nB9jY9i', 'operator', true),
    ('auditor@deltran.local', 'auditor', '\$2a\$12\$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5lk3k7nB9jY9i', 'auditor', true)
ON CONFLICT (email) DO NOTHING;

-- Create bank accounts for test banks
WITH bank_data AS (
    SELECT id, bic_code FROM deltran.banks WHERE bic_code IN ('CHASUS33XXX', 'DEUTDEFFXXX', 'HSBCHKHHHKH', 'SBININBBXXX')
)
INSERT INTO deltran.bank_accounts (bank_id, account_number, currency, balance, available_balance, reserved_balance, is_active)
SELECT
    id,
    bic_code || '-USD-001',
    'USD',
    1000000.00,
    1000000.00,
    0.00,
    true
FROM bank_data
ON CONFLICT (bank_id, account_number, currency) DO NOTHING;

EOF

    echo_info "Test data seeded successfully"
}

# Clean database (development only!)
clean_database() {
    echo_warn "⚠️  WARNING: This will drop all tables and data!"
    read -p "Are you absolutely sure? Type 'DELETE ALL DATA' to confirm: " confirm

    if [ "$confirm" != "DELETE ALL DATA" ]; then
        echo_info "Clean cancelled"
        exit 0
    fi

    echo_info "Cleaning database..."

    PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME <<EOF
DROP SCHEMA IF EXISTS deltran CASCADE;
DELETE FROM flyway_schema_history WHERE 1=1;
EOF

    echo_info "Database cleaned"
}

# Main command handler
main() {
    local command=${1:-migrate}

    case $command in
        migrate)
            wait_for_postgres
            create_database
            run_flyway_migrate
            ;;
        migrate-psql)
            wait_for_postgres
            create_database
            run_psql_migrate
            ;;
        validate)
            validate_migrations
            ;;
        info)
            show_info
            ;;
        rollback)
            rollback_migration
            ;;
        backup)
            backup_database
            ;;
        restore)
            restore_database $2
            ;;
        seed)
            seed_test_data
            ;;
        clean)
            clean_database
            ;;
        help|--help|-h)
            echo "DelTran Database Migration Tool"
            echo ""
            echo "Usage: $0 [command]"
            echo ""
            echo "Commands:"
            echo "  migrate       - Run all pending migrations (default)"
            echo "  migrate-psql  - Run migrations using psql"
            echo "  validate      - Validate applied migrations"
            echo "  info          - Show migration status"
            echo "  rollback      - Show how to rollback last migration"
            echo "  backup        - Create database backup"
            echo "  restore       - Restore database from backup"
            echo "  seed          - Seed test data"
            echo "  clean         - Drop all tables (DANGEROUS!)"
            echo "  help          - Show this help message"
            echo ""
            echo "Environment Variables:"
            echo "  DB_HOST       - Database host (default: localhost)"
            echo "  DB_PORT       - Database port (default: 5432)"
            echo "  DB_NAME       - Database name (default: deltran)"
            echo "  DB_USER       - Database user (default: deltran_app)"
            echo "  DB_PASSWORD   - Database password (required)"
            ;;
        *)
            echo_error "Unknown command: $command"
            echo "Run '$0 help' for usage information"
            exit 1
            ;;
    esac
}

# Run main
main "$@"
