#!/usr/bin/env bash
#
# Snapshot Manager for DelTran
#
# Manages database and volume snapshots for backup and disaster recovery
#
# Usage:
#   ./snapshot-manager.sh create --type [db|volume|all]
#   ./snapshot-manager.sh list --age [days]
#   ./snapshot-manager.sh prune --keep [count]
#   ./snapshot-manager.sh restore --snapshot-id [id]

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_FILE="/var/log/deltran/snapshot-manager.log"

# AWS Configuration
AWS_REGION="${AWS_REGION:-me-central-1}"
DB_CLUSTER_ID="${DB_CLUSTER_ID:-deltran-prod-cluster}"
VOLUME_TAG="Project=DelTran,Environment=production"

# Retention policy
DAILY_SNAPSHOTS_KEEP=7
WEEKLY_SNAPSHOTS_KEEP=4
MONTHLY_SNAPSHOTS_KEEP=12

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging
log() {
    echo "[$(date +'%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

error() {
    echo -e "${RED}[ERROR]${NC} $*" | tee -a "$LOG_FILE" >&2
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*" | tee -a "$LOG_FILE"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $*" | tee -a "$LOG_FILE"
}

# Check prerequisites
check_prerequisites() {
    command -v aws >/dev/null 2>&1 || { error "AWS CLI not installed"; exit 1; }
    command -v jq >/dev/null 2>&1 || { error "jq not installed"; exit 1; }

    # Check AWS credentials
    aws sts get-caller-identity >/dev/null 2>&1 || { error "AWS credentials not configured"; exit 1; }
}

# Create database snapshot
create_db_snapshot() {
    local snapshot_type="${1:-manual}"
    local timestamp=$(date +%Y%m%d-%H%M%S)
    local snapshot_id="deltran-${snapshot_type}-${timestamp}"

    log "Creating database snapshot: $snapshot_id"

    aws rds create-db-cluster-snapshot \
        --db-cluster-identifier "$DB_CLUSTER_ID" \
        --db-cluster-snapshot-identifier "$snapshot_id" \
        --region "$AWS_REGION" \
        --tags "Key=Type,Value=$snapshot_type" "Key=Timestamp,Value=$timestamp" \
               "Key=Project,Value=DelTran" "Key=Environment,Value=production" \
        >/dev/null

    log "Waiting for snapshot to complete..."

    aws rds wait db-cluster-snapshot-available \
        --db-cluster-snapshot-identifier "$snapshot_id" \
        --region "$AWS_REGION"

    success "Database snapshot created: $snapshot_id"

    # Copy to DR region
    log "Copying snapshot to DR region (me-south-1)..."

    aws rds copy-db-cluster-snapshot \
        --source-db-cluster-snapshot-identifier "arn:aws:rds:$AWS_REGION:$(aws sts get-caller-identity --query Account --output text):cluster-snapshot:$snapshot_id" \
        --target-db-cluster-snapshot-identifier "$snapshot_id" \
        --source-region "$AWS_REGION" \
        --region me-south-1 \
        >/dev/null || warning "Failed to copy to DR region"

    echo "$snapshot_id"
}

# Create volume snapshots
create_volume_snapshots() {
    local snapshot_type="${1:-manual}"
    local timestamp=$(date +%Y%m%d-%H%M%S)

    log "Creating volume snapshots for DelTran production volumes"

    # Find all volumes with project tag
    local volumes=$(aws ec2 describe-volumes \
        --filters "Name=tag:Project,Values=DelTran" "Name=tag:Environment,Values=production" \
        --region "$AWS_REGION" \
        --query 'Volumes[*].VolumeId' \
        --output text)

    if [ -z "$volumes" ]; then
        warning "No volumes found with tag: $VOLUME_TAG"
        return
    fi

    local snapshot_ids=()

    for volume_id in $volumes; do
        local volume_name=$(aws ec2 describe-volumes \
            --volume-ids "$volume_id" \
            --region "$AWS_REGION" \
            --query 'Volumes[0].Tags[?Key==`Name`].Value' \
            --output text)

        local snapshot_id="deltran-${snapshot_type}-${volume_name}-${timestamp}"

        log "Creating snapshot for volume $volume_id ($volume_name)"

        aws ec2 create-snapshot \
            --volume-id "$volume_id" \
            --description "DelTran $snapshot_type snapshot - $volume_name" \
            --tag-specifications "ResourceType=snapshot,Tags=[{Key=Type,Value=$snapshot_type},{Key=VolumeId,Value=$volume_id},{Key=Timestamp,Value=$timestamp},{Key=Project,Value=DelTran}]" \
            --region "$AWS_REGION" \
            --output text \
            >/dev/null

        snapshot_ids+=("$snapshot_id")
    done

    success "Created ${#snapshot_ids[@]} volume snapshots"

    printf '%s\n' "${snapshot_ids[@]}"
}

# Create NATS JetStream backup
create_nats_backup() {
    local timestamp=$(date +%Y%m%d-%H%M%S)
    local backup_file="nats-jetstream-${timestamp}.tar.gz"
    local backup_path="/tmp/$backup_file"

    log "Creating NATS JetStream backup"

    # Create backup from NATS data directory
    docker exec deltran-nats-1 tar -czf "/tmp/$backup_file" /data 2>/dev/null || {
        error "Failed to create NATS backup"
        return 1
    }

    # Copy backup from container
    docker cp "deltran-nats-1:/tmp/$backup_file" "$backup_path"

    # Upload to S3
    aws s3 cp "$backup_path" "s3://deltran-backups-$AWS_REGION/nats/$backup_file" \
        --storage-class STANDARD_IA

    # Copy to DR region
    aws s3 cp "s3://deltran-backups-$AWS_REGION/nats/$backup_file" \
        "s3://deltran-backups-me-south-1/nats/$backup_file" || warning "Failed to copy to DR region"

    # Cleanup
    rm -f "$backup_path"
    docker exec deltran-nats-1 rm -f "/tmp/$backup_file"

    success "NATS backup created: $backup_file"

    echo "$backup_file"
}

# List snapshots
list_snapshots() {
    local snapshot_type="${1:-all}"
    local max_age_days="${2:-30}"

    log "Listing snapshots (type: $snapshot_type, max age: $max_age_days days)"

    echo "=== Database Snapshots ==="

    aws rds describe-db-cluster-snapshots \
        --db-cluster-identifier "$DB_CLUSTER_ID" \
        --region "$AWS_REGION" \
        --query "DBClusterSnapshots[?SnapshotCreateTime > \`$(date -u -d "$max_age_days days ago" +%Y-%m-%dT%H:%M:%S)Z\`].[DBClusterSnapshotIdentifier,SnapshotCreateTime,Status,AllocatedStorage]" \
        --output table

    echo ""
    echo "=== Volume Snapshots ==="

    aws ec2 describe-snapshots \
        --owner-ids self \
        --filters "Name=tag:Project,Values=DelTran" \
        --region "$AWS_REGION" \
        --query "Snapshots[?StartTime > \`$(date -u -d "$max_age_days days ago" +%Y-%m-%dT%H:%M:%S)Z\`].[SnapshotId,StartTime,State,VolumeSize,Tags[?Key=='Type'].Value | [0]]" \
        --output table
}

# Prune old snapshots
prune_snapshots() {
    local keep_count="${1:-$DAILY_SNAPSHOTS_KEEP}"
    local dry_run="${2:-false}"

    log "Pruning old snapshots (keep: $keep_count, dry-run: $dry_run)"

    # Prune database snapshots
    local db_snapshots=$(aws rds describe-db-cluster-snapshots \
        --db-cluster-identifier "$DB_CLUSTER_ID" \
        --region "$AWS_REGION" \
        --query 'sort_by(DBClusterSnapshots[?SnapshotType==`manual`], &SnapshotCreateTime)[].DBClusterSnapshotIdentifier' \
        --output text)

    local db_snapshot_array=($db_snapshots)
    local db_total=${#db_snapshot_array[@]}
    local db_delete_count=$((db_total - keep_count))

    if [ $db_delete_count -gt 0 ]; then
        log "Database snapshots: $db_total found, will delete oldest $db_delete_count"

        for ((i=0; i<$db_delete_count; i++)); do
            local snapshot_id="${db_snapshot_array[$i]}"

            if [ "$dry_run" = "true" ]; then
                echo "[DRY-RUN] Would delete database snapshot: $snapshot_id"
            else
                log "Deleting database snapshot: $snapshot_id"
                aws rds delete-db-cluster-snapshot \
                    --db-cluster-snapshot-identifier "$snapshot_id" \
                    --region "$AWS_REGION" >/dev/null || warning "Failed to delete $snapshot_id"
            fi
        done

        success "Pruned $db_delete_count database snapshots"
    else
        log "No database snapshots to prune (total: $db_total, keep: $keep_count)"
    fi

    # Prune volume snapshots
    local volume_snapshots=$(aws ec2 describe-snapshots \
        --owner-ids self \
        --filters "Name=tag:Project,Values=DelTran" "Name=tag:Type,Values=manual" \
        --region "$AWS_REGION" \
        --query 'sort_by(Snapshots, &StartTime)[].SnapshotId' \
        --output text)

    local vol_snapshot_array=($volume_snapshots)
    local vol_total=${#vol_snapshot_array[@]}
    local vol_delete_count=$((vol_total - keep_count))

    if [ $vol_delete_count -gt 0 ]; then
        log "Volume snapshots: $vol_total found, will delete oldest $vol_delete_count"

        for ((i=0; i<$vol_delete_count; i++)); do
            local snapshot_id="${vol_snapshot_array[$i]}"

            if [ "$dry_run" = "true" ]; then
                echo "[DRY-RUN] Would delete volume snapshot: $snapshot_id"
            else
                log "Deleting volume snapshot: $snapshot_id"
                aws ec2 delete-snapshot \
                    --snapshot-id "$snapshot_id" \
                    --region "$AWS_REGION" >/dev/null || warning "Failed to delete $snapshot_id"
            fi
        done

        success "Pruned $vol_delete_count volume snapshots"
    else
        log "No volume snapshots to prune (total: $vol_total, keep: $keep_count)"
    fi

    # Prune NATS backups
    log "Pruning old NATS backups from S3"

    if [ "$dry_run" = "true" ]; then
        aws s3 ls "s3://deltran-backups-$AWS_REGION/nats/" | \
            awk '{print $4}' | \
            sort -r | \
            tail -n +$((keep_count + 1)) | \
            xargs -I {} echo "[DRY-RUN] Would delete s3://deltran-backups-$AWS_REGION/nats/{}"
    else
        aws s3 ls "s3://deltran-backups-$AWS_REGION/nats/" | \
            awk '{print $4}' | \
            sort -r | \
            tail -n +$((keep_count + 1)) | \
            xargs -I {} aws s3 rm "s3://deltran-backups-$AWS_REGION/nats/{}"
    fi
}

# Restore from snapshot
restore_snapshot() {
    local snapshot_id="$1"
    local target_id="${2:-deltran-restored-$(date +%Y%m%d-%H%M%S)}"

    log "Restoring database from snapshot: $snapshot_id to $target_id"

    aws rds restore-db-cluster-from-snapshot \
        --db-cluster-identifier "$target_id" \
        --snapshot-identifier "$snapshot_id" \
        --engine aurora-postgresql \
        --engine-version 15.3 \
        --region "$AWS_REGION"

    log "Waiting for restore to complete (this may take 10-15 minutes)..."

    aws rds wait db-cluster-available \
        --db-cluster-identifier "$target_id" \
        --region "$AWS_REGION"

    success "Database restored to: $target_id"

    # Get endpoint
    local endpoint=$(aws rds describe-db-clusters \
        --db-cluster-identifier "$target_id" \
        --region "$AWS_REGION" \
        --query 'DBClusters[0].Endpoint' \
        --output text)

    echo "Connection endpoint: $endpoint"
}

# Verify snapshot integrity
verify_snapshot() {
    local snapshot_id="$1"

    log "Verifying snapshot integrity: $snapshot_id"

    # Check snapshot exists and is available
    local status=$(aws rds describe-db-cluster-snapshots \
        --db-cluster-snapshot-identifier "$snapshot_id" \
        --region "$AWS_REGION" \
        --query 'DBClusterSnapshots[0].Status' \
        --output text)

    if [ "$status" != "available" ]; then
        error "Snapshot $snapshot_id is not available (status: $status)"
        return 1
    fi

    # Check snapshot size
    local size=$(aws rds describe-db-cluster-snapshots \
        --db-cluster-snapshot-identifier "$snapshot_id" \
        --region "$AWS_REGION" \
        --query 'DBClusterSnapshots[0].AllocatedStorage' \
        --output text)

    if [ "$size" -lt 10 ]; then
        warning "Snapshot size seems too small: ${size}GB"
    fi

    success "Snapshot $snapshot_id verified successfully"
}

# Main command dispatcher
main() {
    check_prerequisites

    local command="${1:-}"

    case "$command" in
        create)
            local type="${2:-all}"
            case "$type" in
                --type)
                    type="${3:-all}"
                    ;;
            esac

            case "$type" in
                db)
                    create_db_snapshot "manual"
                    ;;
                volume)
                    create_volume_snapshots "manual"
                    ;;
                nats)
                    create_nats_backup
                    ;;
                all)
                    create_db_snapshot "manual"
                    create_volume_snapshots "manual"
                    create_nats_backup
                    ;;
                *)
                    error "Unknown snapshot type: $type"
                    echo "Valid types: db, volume, nats, all"
                    exit 1
                    ;;
            esac
            ;;

        list)
            local age="${2:-30}"
            case "$age" in
                --age)
                    age="${3:-30}"
                    ;;
            esac
            list_snapshots "all" "$age"
            ;;

        prune)
            local keep="${2:-$DAILY_SNAPSHOTS_KEEP}"
            local dry_run="false"

            shift
            while [[ $# -gt 0 ]]; do
                case "$1" in
                    --keep)
                        keep="$2"
                        shift 2
                        ;;
                    --dry-run)
                        dry_run="true"
                        shift
                        ;;
                    *)
                        shift
                        ;;
                esac
            done

            prune_snapshots "$keep" "$dry_run"
            ;;

        restore)
            local snapshot_id=""
            local target_id=""

            shift
            while [[ $# -gt 0 ]]; do
                case "$1" in
                    --snapshot-id)
                        snapshot_id="$2"
                        shift 2
                        ;;
                    --target-id)
                        target_id="$2"
                        shift 2
                        ;;
                    *)
                        shift
                        ;;
                esac
            done

            if [ -z "$snapshot_id" ]; then
                error "Snapshot ID required: --snapshot-id <id>"
                exit 1
            fi

            restore_snapshot "$snapshot_id" "$target_id"
            ;;

        verify)
            local snapshot_id="${2}"
            if [ -z "$snapshot_id" ]; then
                error "Snapshot ID required"
                exit 1
            fi
            verify_snapshot "$snapshot_id"
            ;;

        --help|help|*)
            cat <<EOF
DelTran Snapshot Manager

Usage:
  $0 create --type [db|volume|nats|all]
  $0 list [--age DAYS]
  $0 prune [--keep COUNT] [--dry-run]
  $0 restore --snapshot-id ID [--target-id ID]
  $0 verify SNAPSHOT_ID

Commands:
  create    Create new snapshots
  list      List existing snapshots
  prune     Delete old snapshots
  restore   Restore from snapshot
  verify    Verify snapshot integrity

Examples:
  # Create all snapshots
  $0 create --type all

  # List snapshots from last 7 days
  $0 list --age 7

  # Prune old snapshots (keep 7 most recent)
  $0 prune --keep 7

  # Test pruning (dry-run)
  $0 prune --keep 7 --dry-run

  # Restore database
  $0 restore --snapshot-id deltran-manual-20250930-100000

Environment Variables:
  AWS_REGION        AWS region (default: me-central-1)
  DB_CLUSTER_ID     Database cluster ID (default: deltran-prod-cluster)

For more information, see: docs/runbooks/snapshot-management.md
EOF
            ;;
    esac
}

# Run main
main "$@"
