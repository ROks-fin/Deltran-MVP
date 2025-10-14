#!/bin/bash
# Comprehensive Database Backup Strategy for DelTran
# Supports: PostgreSQL, Redis, RocksDB
# Features: Incremental backups, compression, encryption, S3 upload

set -e

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

# Configuration
BACKUP_ROOT="${BACKUP_ROOT:-./backups}"
BACKUP_RETENTION_DAYS="${BACKUP_RETENTION_DAYS:-30}"
S3_BUCKET="${S3_BUCKET:-s3://deltran-backups}"
ENCRYPTION_KEY="${ENCRYPTION_KEY:-}"

# Database configuration
DB_HOST="${DB_HOST:-localhost}"
DB_PORT="${DB_PORT:-5432}"
DB_NAME="${DB_NAME:-deltran}"
DB_USER="${DB_USER:-deltran_app}"
DB_PASSWORD="${DB_PASSWORD:-changeme123}"

REDIS_HOST="${REDIS_HOST:-localhost}"
REDIS_PORT="${REDIS_PORT:-6379}"
REDIS_PASSWORD="${REDIS_PASSWORD:-redis123}"

ROCKSDB_PATH="${ROCKSDB_PATH:-./data/rocksdb}"

# Timestamp for backups
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
DATE_DIR=$(date +%Y-%m-%d)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $(date +'%Y-%m-%d %H:%M:%S') - $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $(date +'%Y-%m-%d %H:%M:%S') - $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $(date +'%Y-%m-%d %H:%M:%S') - $1"
}

log_debug() {
    echo -e "${BLUE}[DEBUG]${NC} $(date +'%Y-%m-%d %H:%M:%S') - $1"
}

# Create backup directory structure
prepare_backup_dir() {
    local backup_dir="$BACKUP_ROOT/$DATE_DIR"
    mkdir -p "$backup_dir/postgres"
    mkdir -p "$backup_dir/redis"
    mkdir -p "$backup_dir/rocksdb"
    mkdir -p "$backup_dir/logs"
    echo "$backup_dir"
}

# ========================================
# POSTGRESQL BACKUP
# ========================================

backup_postgresql() {
    local backup_dir=$1
    local output_file="$backup_dir/postgres/deltran_pg_${TIMESTAMP}.dump"

    log_info "Starting PostgreSQL backup..."

    # Full backup using pg_dump (custom format for efficient restore)
    PGPASSWORD=$DB_PASSWORD pg_dump \
        -h $DB_HOST \
        -p $DB_PORT \
        -U $DB_USER \
        -d $DB_NAME \
        --format=custom \
        --compress=9 \
        --verbose \
        --file="$output_file" \
        2>&1 | tee "$backup_dir/logs/postgres_backup.log"

    if [ $? -eq 0 ]; then
        log_info "PostgreSQL backup completed: $output_file"

        # Calculate checksum
        local checksum=$(sha256sum "$output_file" | awk '{print $1}')
        echo "$checksum  $output_file" > "$output_file.sha256"

        # Get file size
        local size=$(du -h "$output_file" | cut -f1)
        log_info "Backup size: $size, SHA256: $checksum"

        echo "$output_file"
    else
        log_error "PostgreSQL backup failed"
        return 1
    fi
}

# Incremental WAL archiving
backup_postgresql_wal() {
    local backup_dir=$1
    local wal_dir="$backup_dir/postgres/wal"

    mkdir -p "$wal_dir"

    log_info "Archiving PostgreSQL WAL files..."

    # Get WAL files location
    local wal_location=$(PGPASSWORD=$DB_PASSWORD psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -t -c "SHOW wal_level;")

    if [[ "$wal_location" == *"replica"* ]] || [[ "$wal_location" == *"logical"* ]]; then
        # Copy WAL files if available
        docker exec deltran-postgres-primary tar czf - /var/lib/postgresql/data/pg_wal > "$wal_dir/pg_wal_${TIMESTAMP}.tar.gz"
        log_info "WAL files archived: $wal_dir/pg_wal_${TIMESTAMP}.tar.gz"
    else
        log_warn "WAL archiving not enabled (wal_level=$wal_location)"
    fi
}

# Schema-only backup (for documentation)
backup_postgresql_schema() {
    local backup_dir=$1
    local schema_file="$backup_dir/postgres/schema_${TIMESTAMP}.sql"

    log_info "Backing up PostgreSQL schema..."

    PGPASSWORD=$DB_PASSWORD pg_dump \
        -h $DB_HOST \
        -p $DB_PORT \
        -U $DB_USER \
        -d $DB_NAME \
        --schema-only \
        --no-owner \
        --no-privileges \
        > "$schema_file"

    log_info "Schema backup completed: $schema_file"
}

# ========================================
# REDIS BACKUP
# ========================================

backup_redis() {
    local backup_dir=$1
    local output_file="$backup_dir/redis/redis_${TIMESTAMP}.rdb"

    log_info "Starting Redis backup..."

    # Trigger BGSAVE
    redis-cli -h $REDIS_HOST -p $REDIS_PORT -a $REDIS_PASSWORD BGSAVE

    # Wait for BGSAVE to complete
    local save_in_progress=1
    while [ $save_in_progress -eq 1 ]; do
        local status=$(redis-cli -h $REDIS_HOST -p $REDIS_PORT -a $REDIS_PASSWORD LASTSAVE)
        sleep 2
        local new_status=$(redis-cli -h $REDIS_HOST -p $REDIS_PORT -a $REDIS_PASSWORD LASTSAVE)

        if [ "$status" != "$new_status" ]; then
            save_in_progress=0
        fi
    done

    # Copy RDB file from container
    docker cp deltran-redis-master:/data/dump.rdb "$output_file"

    if [ $? -eq 0 ]; then
        # Compress
        gzip -9 "$output_file"
        output_file="${output_file}.gz"

        log_info "Redis backup completed: $output_file"

        # Calculate checksum
        local checksum=$(sha256sum "$output_file" | awk '{print $1}')
        echo "$checksum  $output_file" > "$output_file.sha256"

        local size=$(du -h "$output_file" | cut -f1)
        log_info "Backup size: $size, SHA256: $checksum"

        echo "$output_file"
    else
        log_error "Redis backup failed"
        return 1
    fi
}

# ========================================
# ROCKSDB BACKUP
# ========================================

backup_rocksdb() {
    local backup_dir=$1
    local output_file="$backup_dir/rocksdb/rocksdb_${TIMESTAMP}.tar.gz"

    log_info "Starting RocksDB backup..."

    # Check if RocksDB path exists
    if [ ! -d "$ROCKSDB_PATH" ]; then
        log_warn "RocksDB path not found: $ROCKSDB_PATH"
        return 0
    fi

    # Create tarball with compression
    tar czf "$output_file" \
        -C "$(dirname $ROCKSDB_PATH)" \
        "$(basename $ROCKSDB_PATH)" \
        --exclude='*.tmp' \
        --exclude='LOCK'

    if [ $? -eq 0 ]; then
        log_info "RocksDB backup completed: $output_file"

        # Calculate checksum
        local checksum=$(sha256sum "$output_file" | awk '{print $1}')
        echo "$checksum  $output_file" > "$output_file.sha256"

        local size=$(du -h "$output_file" | cut -f1)
        log_info "Backup size: $size, SHA256: $checksum"

        echo "$output_file"
    else
        log_error "RocksDB backup failed"
        return 1
    fi
}

# ========================================
# ENCRYPTION
# ========================================

encrypt_backup() {
    local file=$1

    if [ -z "$ENCRYPTION_KEY" ]; then
        log_debug "Encryption key not set, skipping encryption"
        return 0
    fi

    log_info "Encrypting backup: $file"

    # Encrypt using AES-256-CBC
    openssl enc -aes-256-cbc -salt -pbkdf2 \
        -in "$file" \
        -out "${file}.enc" \
        -pass pass:$ENCRYPTION_KEY

    if [ $? -eq 0 ]; then
        # Remove unencrypted file
        rm "$file"
        log_info "Backup encrypted: ${file}.enc"
        echo "${file}.enc"
    else
        log_error "Encryption failed"
        return 1
    fi
}

# ========================================
# UPLOAD TO S3
# ========================================

upload_to_s3() {
    local file=$1
    local s3_path="$S3_BUCKET/$DATE_DIR/$(basename $file)"

    if ! command -v aws &> /dev/null; then
        log_warn "AWS CLI not found, skipping S3 upload"
        return 0
    fi

    log_info "Uploading to S3: $s3_path"

    aws s3 cp "$file" "$s3_path" \
        --storage-class STANDARD_IA \
        --metadata "backup-timestamp=$TIMESTAMP,backup-type=full"

    if [ $? -eq 0 ]; then
        log_info "Uploaded to S3: $s3_path"
    else
        log_error "S3 upload failed"
        return 1
    fi
}

# ========================================
# CLEANUP OLD BACKUPS
# ========================================

cleanup_old_backups() {
    log_info "Cleaning up backups older than $BACKUP_RETENTION_DAYS days..."

    find "$BACKUP_ROOT" -type f -mtime +$BACKUP_RETENTION_DAYS -delete
    find "$BACKUP_ROOT" -type d -empty -delete

    log_info "Old backups cleaned up"
}

# ========================================
# VERIFY BACKUP INTEGRITY
# ========================================

verify_backup() {
    local file=$1
    local checksum_file="${file}.sha256"

    if [ ! -f "$checksum_file" ]; then
        log_warn "Checksum file not found: $checksum_file"
        return 1
    fi

    log_info "Verifying backup integrity: $file"

    sha256sum -c "$checksum_file"

    if [ $? -eq 0 ]; then
        log_info "Backup integrity verified"
        return 0
    else
        log_error "Backup integrity check failed!"
        return 1
    fi
}

# ========================================
# GENERATE BACKUP MANIFEST
# ========================================

generate_manifest() {
    local backup_dir=$1
    local manifest_file="$backup_dir/manifest.json"

    log_info "Generating backup manifest..."

    cat > "$manifest_file" <<EOF
{
  "backup_timestamp": "$TIMESTAMP",
  "backup_date": "$DATE_DIR",
  "system": "DelTran Payment Rail",
  "version": "1.0",
  "components": {
    "postgresql": {
      "host": "$DB_HOST",
      "database": "$DB_NAME",
      "files": [$(find $backup_dir/postgres -type f -name "*.dump" -printf '"%f",' | sed 's/,$//')],
      "schema_file": "schema_${TIMESTAMP}.sql"
    },
    "redis": {
      "host": "$REDIS_HOST",
      "files": [$(find $backup_dir/redis -type f -name "*.rdb.gz" -printf '"%f",' | sed 's/,$//'))]
    },
    "rocksdb": {
      "path": "$ROCKSDB_PATH",
      "files": [$(find $backup_dir/rocksdb -type f -name "*.tar.gz" -printf '"%f",' | sed 's/,$//'))]
    }
  },
  "retention_days": $BACKUP_RETENTION_DAYS,
  "encryption_enabled": $([ -n "$ENCRYPTION_KEY" ] && echo "true" || echo "false")
}
EOF

    log_info "Manifest generated: $manifest_file"
}

# ========================================
# FULL BACKUP WORKFLOW
# ========================================

run_full_backup() {
    log_info "========================================="
    log_info "Starting FULL system backup"
    log_info "========================================="

    local backup_dir=$(prepare_backup_dir)
    log_info "Backup directory: $backup_dir"

    # Backup PostgreSQL
    local pg_file=$(backup_postgresql "$backup_dir")
    backup_postgresql_schema "$backup_dir"
    backup_postgresql_wal "$backup_dir"

    # Backup Redis
    local redis_file=$(backup_redis "$backup_dir")

    # Backup RocksDB
    local rocksdb_file=$(backup_rocksdb "$backup_dir")

    # Generate manifest
    generate_manifest "$backup_dir"

    # Encrypt if key provided
    if [ -n "$ENCRYPTION_KEY" ]; then
        [ -n "$pg_file" ] && encrypt_backup "$pg_file"
        [ -n "$redis_file" ] && encrypt_backup "$redis_file"
        [ -n "$rocksdb_file" ] && encrypt_backup "$rocksdb_file"
    fi

    # Upload to S3
    if [ -n "$S3_BUCKET" ]; then
        find "$backup_dir" -type f -exec bash -c 'upload_to_s3 "$0"' {} \;
    fi

    # Cleanup old backups
    cleanup_old_backups

    log_info "========================================="
    log_info "Backup completed successfully!"
    log_info "Location: $backup_dir"
    log_info "========================================="
}

# ========================================
# MAIN
# ========================================

main() {
    local command=${1:-full}

    case $command in
        full)
            run_full_backup
            ;;
        postgres)
            local backup_dir=$(prepare_backup_dir)
            backup_postgresql "$backup_dir"
            ;;
        redis)
            local backup_dir=$(prepare_backup_dir)
            backup_redis "$backup_dir"
            ;;
        rocksdb)
            local backup_dir=$(prepare_backup_dir)
            backup_rocksdb "$backup_dir"
            ;;
        verify)
            if [ -z "$2" ]; then
                log_error "Please provide backup file to verify"
                exit 1
            fi
            verify_backup "$2"
            ;;
        cleanup)
            cleanup_old_backups
            ;;
        help|--help|-h)
            echo "DelTran Database Backup Tool"
            echo ""
            echo "Usage: $0 [command]"
            echo ""
            echo "Commands:"
            echo "  full      - Full system backup (default)"
            echo "  postgres  - Backup PostgreSQL only"
            echo "  redis     - Backup Redis only"
            echo "  rocksdb   - Backup RocksDB only"
            echo "  verify    - Verify backup integrity"
            echo "  cleanup   - Remove old backups"
            echo "  help      - Show this help"
            echo ""
            echo "Environment Variables:"
            echo "  BACKUP_ROOT              - Backup root directory (default: ./backups)"
            echo "  BACKUP_RETENTION_DAYS    - Retention period (default: 30)"
            echo "  S3_BUCKET                - S3 bucket for upload (optional)"
            echo "  ENCRYPTION_KEY           - Encryption password (optional)"
            ;;
        *)
            log_error "Unknown command: $command"
            echo "Run '$0 help' for usage"
            exit 1
            ;;
    esac
}

main "$@"
