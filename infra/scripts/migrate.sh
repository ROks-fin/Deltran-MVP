#!/bin/bash
# DelTran Database Migration Script
# Handles schema creation, upgrades, and rollbacks

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SQL_DIR="${SCRIPT_DIR}/../sql"
LOG_DIR="${SCRIPT_DIR}/../../logs"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Database connection parameters
DB_HOST="${POSTGRES_HOST:-localhost}"
DB_PORT="${POSTGRES_PORT:-5432}"
DB_NAME="${POSTGRES_DB:-deltran}"
DB_USER="${POSTGRES_USER:-deltran_app}"
DB_PASSWORD="${POSTGRES_PASSWORD:-changeme123}"

# Connection string
DB_CONN="postgresql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

# Create log directory
mkdir -p "${LOG_DIR}"
LOG_FILE="${LOG_DIR}/migration_$(date +%Y%m%d_%H%M%S).log"

# Logging function
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "${LOG_FILE}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR:${NC} $1" | tee -a "${LOG_FILE}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING:${NC} $1" | tee -a "${LOG_FILE}"
}

# Check if PostgreSQL is available
check_postgres() {
    log "Checking PostgreSQL connection..."

    if ! command -v psql &> /dev/null; then
        error "psql command not found. Please install PostgreSQL client."
        exit 1
    fi

    if ! PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres -c "SELECT 1" &> /dev/null; then
        error "Cannot connect to PostgreSQL at ${DB_HOST}:${DB_PORT}"
        exit 1
    fi

    log "PostgreSQL connection successful"
}

# Create database if not exists
create_database() {
    log "Creating database '${DB_NAME}' if not exists..."

    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres -tc \
        "SELECT 1 FROM pg_database WHERE datname = '${DB_NAME}'" | grep -q 1 || \
        PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres \
            -c "CREATE DATABASE ${DB_NAME}"

    log "Database '${DB_NAME}' ready"
}

# Create migration tracking table
create_migration_table() {
    log "Creating migration tracking table..."

    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" <<-EOSQL
        CREATE SCHEMA IF NOT EXISTS public;

        CREATE TABLE IF NOT EXISTS public.schema_migrations (
            id SERIAL PRIMARY KEY,
            version VARCHAR(50) UNIQUE NOT NULL,
            name VARCHAR(255) NOT NULL,
            applied_at TIMESTAMPTZ DEFAULT NOW(),
            execution_time_ms INTEGER,
            checksum VARCHAR(64),
            success BOOLEAN DEFAULT true
        );

        CREATE INDEX IF NOT EXISTS idx_schema_migrations_version
            ON public.schema_migrations(version);

        CREATE INDEX IF NOT EXISTS idx_schema_migrations_applied_at
            ON public.schema_migrations(applied_at DESC);
EOSQL

    log "Migration tracking table ready"
}

# Get current schema version
get_current_version() {
    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" -t -c \
        "SELECT COALESCE(MAX(version), '000') FROM public.schema_migrations WHERE success = true" | xargs
}

# Calculate checksum of SQL file
calculate_checksum() {
    local file=$1
    if command -v sha256sum &> /dev/null; then
        sha256sum "$file" | awk '{print $1}'
    elif command -v shasum &> /dev/null; then
        shasum -a 256 "$file" | awk '{print $1}'
    else
        echo "unknown"
    fi
}

# Apply single migration
apply_migration() {
    local file=$1
    local version=$(basename "$file" | sed 's/_.*//')
    local name=$(basename "$file" .sql | sed 's/^[0-9]*_//')

    log "Applying migration: ${version} - ${name}"

    local checksum=$(calculate_checksum "$file")
    local start_time=$(date +%s%3N)

    # Execute migration
    if PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" \
        -v ON_ERROR_STOP=1 -f "$file" >> "${LOG_FILE}" 2>&1; then

        local end_time=$(date +%s%3N)
        local execution_time=$((end_time - start_time))

        # Record successful migration
        PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" <<-EOSQL
            INSERT INTO public.schema_migrations (version, name, execution_time_ms, checksum, success)
            VALUES ('${version}', '${name}', ${execution_time}, '${checksum}', true)
            ON CONFLICT (version) DO UPDATE SET
                applied_at = NOW(),
                execution_time_ms = ${execution_time},
                checksum = '${checksum}',
                success = true;
EOSQL

        log "Migration ${version} applied successfully (${execution_time}ms)"
        return 0
    else
        error "Migration ${version} failed"

        # Record failed migration
        PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" <<-EOSQL
            INSERT INTO public.schema_migrations (version, name, checksum, success)
            VALUES ('${version}', '${name}', '${checksum}', false)
            ON CONFLICT (version) DO UPDATE SET
                applied_at = NOW(),
                checksum = '${checksum}',
                success = false;
EOSQL

        return 1
    fi
}

# Run all pending migrations
migrate_up() {
    log "Starting database migration..."

    check_postgres
    create_database
    create_migration_table

    local current_version=$(get_current_version)
    log "Current schema version: ${current_version}"

    local migration_count=0
    local failed=0

    # Find and apply migrations
    for migration_file in "${SQL_DIR}"/*.sql; do
        if [ -f "$migration_file" ]; then
            local version=$(basename "$migration_file" | sed 's/_.*//')

            # Skip if already applied
            if [ "$version" \> "$current_version" ] || [ "$version" = "000" ]; then
                if apply_migration "$migration_file"; then
                    migration_count=$((migration_count + 1))
                else
                    failed=1
                    error "Migration failed, stopping"
                    break
                fi
            else
                log "Skipping already applied migration: ${version}"
            fi
        fi
    done

    if [ $failed -eq 0 ]; then
        log "Migration complete! Applied ${migration_count} migration(s)"
        log "Log file: ${LOG_FILE}"
    else
        error "Migration failed. Check log file: ${LOG_FILE}"
        exit 1
    fi
}

# Show migration status
status() {
    log "Migration status:"

    check_postgres

    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d "${DB_NAME}" <<-EOSQL
        SELECT
            version,
            name,
            applied_at,
            execution_time_ms || 'ms' as execution_time,
            CASE WHEN success THEN '✓' ELSE '✗' END as status
        FROM public.schema_migrations
        ORDER BY version DESC
        LIMIT 10;
EOSQL
}

# Create backup before migration
backup() {
    log "Creating database backup..."

    local backup_dir="${SCRIPT_DIR}/../../data/backups"
    mkdir -p "${backup_dir}"

    local backup_file="${backup_dir}/deltran_backup_$(date +%Y%m%d_%H%M%S).sql"

    PGPASSWORD="${DB_PASSWORD}" pg_dump -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" \
        -d "${DB_NAME}" -F p -f "${backup_file}"

    log "Backup created: ${backup_file}"

    # Compress backup
    gzip "${backup_file}"
    log "Backup compressed: ${backup_file}.gz"
}

# Restore from backup
restore() {
    local backup_file=$1

    if [ -z "$backup_file" ]; then
        error "Usage: $0 restore <backup_file>"
        exit 1
    fi

    if [ ! -f "$backup_file" ]; then
        error "Backup file not found: ${backup_file}"
        exit 1
    fi

    warn "This will DROP and recreate the database. Are you sure? (yes/no)"
    read -r confirmation

    if [ "$confirmation" != "yes" ]; then
        log "Restore cancelled"
        exit 0
    fi

    log "Restoring database from: ${backup_file}"

    # Drop existing database
    PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" -d postgres \
        -c "DROP DATABASE IF EXISTS ${DB_NAME}"

    # Create fresh database
    create_database

    # Restore
    if [[ "$backup_file" == *.gz ]]; then
        gunzip -c "${backup_file}" | PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" \
            -U "${DB_USER}" -d "${DB_NAME}"
    else
        PGPASSWORD="${DB_PASSWORD}" psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" \
            -d "${DB_NAME}" -f "${backup_file}"
    fi

    log "Restore complete"
}

# Rollback to specific version
rollback() {
    local target_version=$1

    if [ -z "$target_version" ]; then
        error "Usage: $0 rollback <version>"
        exit 1
    fi

    warn "This will rollback to version ${target_version}. Are you sure? (yes/no)"
    read -r confirmation

    if [ "$confirmation" != "yes" ]; then
        log "Rollback cancelled"
        exit 0
    fi

    error "Rollback not implemented yet. Please restore from backup."
    exit 1
}

# Show help
help() {
    cat <<EOF
DelTran Database Migration Tool

Usage: $0 <command> [options]

Commands:
    up              Apply all pending migrations
    status          Show migration status
    backup          Create database backup
    restore <file>  Restore from backup
    rollback <ver>  Rollback to specific version (not implemented)
    help            Show this help message

Environment Variables:
    POSTGRES_HOST     Database host (default: localhost)
    POSTGRES_PORT     Database port (default: 5432)
    POSTGRES_DB       Database name (default: deltran)
    POSTGRES_USER     Database user (default: deltran_app)
    POSTGRES_PASSWORD Database password (default: changeme123)

Examples:
    # Apply migrations
    $0 up

    # Check status
    $0 status

    # Create backup
    $0 backup

    # Restore from backup
    $0 restore ./data/backups/deltran_backup_20251012.sql.gz

EOF
}

# Main
case "${1:-}" in
    up)
        migrate_up
        ;;
    status)
        status
        ;;
    backup)
        backup
        ;;
    restore)
        restore "$2"
        ;;
    rollback)
        rollback "$2"
        ;;
    help|--help|-h)
        help
        ;;
    *)
        error "Unknown command: ${1:-}"
        help
        exit 1
        ;;
esac
