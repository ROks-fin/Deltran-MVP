#!/usr/bin/env bash
#
# Nightly Replay Tests for DelTran
#
# Replays production transactions from last 24 hours in test environment
# to verify protocol correctness and catch regressions.
#
# Usage:
#   ./nightly-replay-test.sh [--date YYYY-MM-DD] [--dry-run]

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
LOG_FILE="/var/log/deltran/replay-tests.log"

# Database configuration
PROD_DB_HOST="${PROD_DB_HOST:-prod-db.deltran.io}"
TEST_DB_HOST="${TEST_DB_HOST:-test-db.deltran.io}"
DB_USER="${DB_USER:-deltran_readonly}"

# Test environment
TEST_API_URL="${TEST_API_URL:-https://api-test.deltran.io}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

info() {
    echo -e "${BLUE}[INFO]${NC} $*" | tee -a "$LOG_FILE"
}

# Parse arguments
REPLAY_DATE=$(date -u -d "yesterday" +%Y-%m-%d)
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --date)
            REPLAY_DATE="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --help)
            cat <<EOF
Nightly Replay Tests

Usage:
  $0 [--date YYYY-MM-DD] [--dry-run]

Options:
  --date       Date to replay (default: yesterday)
  --dry-run    Don't actually execute, just show what would be done

Examples:
  $0                          # Replay yesterday's transactions
  $0 --date 2025-09-30        # Replay specific date
  $0 --dry-run                # Test run without execution
EOF
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            exit 1
            ;;
    esac
done

log "===== Nightly Replay Test Started ====="
log "Replaying transactions from: $REPLAY_DATE"
log "Dry run: $DRY_RUN"

# ============================================================================
# Step 1: Export production data
# ============================================================================

export_production_data() {
    log "Step 1: Exporting production data for $REPLAY_DATE"

    local export_file="/tmp/replay-$REPLAY_DATE.json"

    # Export completed payments from production
    psql -h "$PROD_DB_HOST" -U "$DB_USER" -d deltran_prod -t -A -F"," \
        -c "COPY (
            SELECT
                payment_id,
                corridor_id,
                debtor_bank,
                creditor_bank,
                amount,
                currency,
                created_at,
                completed_at,
                state,
                netting_batch_id,
                settlement_batch_id
            FROM payments
            WHERE DATE(created_at) = '$REPLAY_DATE'
              AND state = 'completed'
            ORDER BY created_at
        ) TO STDOUT WITH CSV HEADER" > "$export_file"

    local payment_count=$(wc -l < "$export_file")
    log "Exported $payment_count payments"

    if [ "$payment_count" -eq 0 ]; then
        error "No payments found for $REPLAY_DATE"
        return 1
    fi

    echo "$export_file"
}

# ============================================================================
# Step 2: Prepare test environment
# ============================================================================

prepare_test_environment() {
    log "Step 2: Preparing test environment"

    # Reset test database to clean state
    psql -h "$TEST_DB_HOST" -U deltran -d deltran_test -c "
        TRUNCATE TABLE payments CASCADE;
        TRUNCATE TABLE netting_batches CASCADE;
        TRUNCATE TABLE settlement_batches CASCADE;
        TRUNCATE TABLE payment_state_log CASCADE;
    "

    # Restart services in test environment
    kubectl --context test scale deployment --all --replicas=0
    sleep 5
    kubectl --context test scale deployment --all --replicas=1

    # Wait for services to be ready
    kubectl --context test wait --for=condition=ready pod --all --timeout=120s

    success "Test environment ready"
}

# ============================================================================
# Step 3: Replay transactions
# ============================================================================

replay_transactions() {
    local export_file="$1"

    log "Step 3: Replaying transactions"

    local total_payments=$(wc -l < "$export_file")
    local current=0
    local success_count=0
    local failure_count=0

    # Read CSV and replay each payment
    while IFS=',' read -r payment_id corridor_id debtor_bank creditor_bank amount currency created_at completed_at state netting_batch_id settlement_batch_id; do
        # Skip header
        if [ "$payment_id" = "payment_id" ]; then
            continue
        fi

        current=$((current + 1))

        info "[$current/$total_payments] Replaying payment: $payment_id"

        if [ "$DRY_RUN" = "true" ]; then
            echo "[DRY-RUN] Would replay: $payment_id ($amount $currency)"
            continue
        fi

        # Submit payment to test environment
        local response=$(curl -s -w "\n%{http_code}" -X POST "$TEST_API_URL/v1/payments" \
            -H "Authorization: Bearer $TEST_API_TOKEN" \
            -H "Content-Type: application/json" \
            -d "{
                \"payment_id\": \"$payment_id\",
                \"corridor_id\": \"$corridor_id\",
                \"debtor_bank\": \"$debtor_bank\",
                \"creditor_bank\": \"$creditor_bank\",
                \"amount\": $amount,
                \"currency\": \"$currency\",
                \"replay\": true
            }")

        local http_code=$(echo "$response" | tail -n1)
        local body=$(echo "$response" | head -n-1)

        if [ "$http_code" -eq 200 ] || [ "$http_code" -eq 201 ]; then
            success_count=$((success_count + 1))
        else
            error "Failed to replay $payment_id: HTTP $http_code - $body"
            failure_count=$((failure_count + 1))
        fi

        # Rate limiting
        sleep 0.1
    done < "$export_file"

    log "Replay completed: $success_count succeeded, $failure_count failed"

    return $failure_count
}

# ============================================================================
# Step 4: Verify results
# ============================================================================

verify_results() {
    local export_file="$1"

    log "Step 4: Verifying results"

    local expected_count=$(wc -l < "$export_file" | tail -1)

    # Wait for processing to complete (max 10 minutes)
    log "Waiting for payments to complete..."

    local timeout=600
    local elapsed=0

    while [ $elapsed -lt $timeout ]; do
        local completed=$(psql -h "$TEST_DB_HOST" -U deltran -d deltran_test -t -c \
            "SELECT COUNT(*) FROM payments WHERE state = 'completed';")

        log "Completed: $completed / $expected_count"

        if [ "$completed" -ge "$expected_count" ]; then
            break
        fi

        sleep 10
        elapsed=$((elapsed + 10))
    done

    # Verify payment states
    local state_mismatches=$(psql -h "$TEST_DB_HOST" -U deltran -d deltran_test -t -c "
        SELECT COUNT(*)
        FROM payments p_test
        INNER JOIN dblink(
            'host=$PROD_DB_HOST user=$DB_USER dbname=deltran_prod',
            'SELECT payment_id, state FROM payments WHERE DATE(created_at) = ''$REPLAY_DATE'''
        ) AS p_prod(payment_id uuid, state text)
        ON p_test.payment_id = p_prod.payment_id
        WHERE p_test.state != p_prod.state;
    ")

    if [ "$state_mismatches" -gt 0 ]; then
        error "Found $state_mismatches state mismatches!"
        return 1
    fi

    # Verify amounts (money conservation)
    local amount_mismatches=$(psql -h "$TEST_DB_HOST" -U deltran -d deltran_test -t -c "
        SELECT COUNT(*)
        FROM payments p_test
        INNER JOIN dblink(
            'host=$PROD_DB_HOST user=$DB_USER dbname=deltran_prod',
            'SELECT payment_id, amount FROM payments WHERE DATE(created_at) = ''$REPLAY_DATE'''
        ) AS p_prod(payment_id uuid, amount numeric)
        ON p_test.payment_id = p_prod.payment_id
        WHERE p_test.amount != p_prod.amount;
    ")

    if [ "$amount_mismatches" -gt 0 ]; then
        error "Found $amount_mismatches amount mismatches!"
        return 1
    fi

    # Verify netting efficiency
    local test_netting=$(psql -h "$TEST_DB_HOST" -U deltran -d deltran_test -t -c \
        "SELECT AVG(netting_efficiency) FROM netting_batches;")

    local prod_netting=$(psql -h "$PROD_DB_HOST" -U "$DB_USER" -d deltran_prod -t -c \
        "SELECT AVG(netting_efficiency) FROM netting_batches WHERE DATE(created_at) = '$REPLAY_DATE';")

    log "Netting efficiency - Test: $test_netting, Prod: $prod_netting"

    # Allow 1% variance
    local diff=$(echo "$test_netting - $prod_netting" | bc -l | tr -d '-')
    if (( $(echo "$diff > 0.01" | bc -l) )); then
        error "Netting efficiency differs by more than 1%"
        return 1
    fi

    success "All verifications passed"
}

# ============================================================================
# Step 5: Generate report
# ============================================================================

generate_report() {
    local export_file="$1"
    local success_count="$2"
    local failure_count="$3"

    log "Step 5: Generating report"

    local report_file="/tmp/replay-report-$REPLAY_DATE.html"

    cat > "$report_file" <<EOF
<!DOCTYPE html>
<html>
<head>
    <title>Replay Test Report - $REPLAY_DATE</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .success { color: green; }
        .failure { color: red; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #4CAF50; color: white; }
    </style>
</head>
<body>
    <h1>Nightly Replay Test Report</h1>
    <p><strong>Date:</strong> $REPLAY_DATE</p>
    <p><strong>Run Time:</strong> $(date)</p>

    <h2>Summary</h2>
    <table>
        <tr>
            <th>Metric</th>
            <th>Value</th>
        </tr>
        <tr>
            <td>Total Payments</td>
            <td>$(wc -l < "$export_file")</td>
        </tr>
        <tr>
            <td class="success">Successful Replays</td>
            <td>$success_count</td>
        </tr>
        <tr>
            <td class="failure">Failed Replays</td>
            <td>$failure_count</td>
        </tr>
        <tr>
            <td>Success Rate</td>
            <td>$(echo "scale=2; $success_count * 100 / ($success_count + $failure_count)" | bc)%</td>
        </tr>
    </table>

    <h2>Verification Results</h2>
    <ul>
        <li>State consistency: ✓</li>
        <li>Amount conservation: ✓</li>
        <li>Netting efficiency: ✓</li>
    </ul>

    <p>Full logs: <code>$LOG_FILE</code></p>
</body>
</html>
EOF

    log "Report generated: $report_file"

    # Send report via email
    if command -v mail &> /dev/null; then
        mail -s "Replay Test Report - $REPLAY_DATE" \
             -a "$report_file" \
             ops@deltran.io < "$report_file"
    fi

    # Upload to S3
    aws s3 cp "$report_file" "s3://deltran-reports/replay-tests/$REPLAY_DATE.html"

    success "Report uploaded"
}

# ============================================================================
# Main execution
# ============================================================================

main() {
    # Export data
    local export_file
    export_file=$(export_production_data) || exit 1

    # Prepare test environment
    prepare_test_environment || exit 1

    # Replay transactions
    replay_transactions "$export_file"
    local failures=$?

    # Verify results
    if [ "$DRY_RUN" = "false" ]; then
        verify_results "$export_file" || exit 1
    fi

    # Generate report
    local success_count=$(($(wc -l < "$export_file") - failures))
    generate_report "$export_file" "$success_count" "$failures"

    # Cleanup
    rm -f "$export_file"

    if [ "$failures" -eq 0 ]; then
        success "===== Nightly Replay Test Completed Successfully ====="
        exit 0
    else
        error "===== Nightly Replay Test Completed with $failures Failures ====="
        exit 1
    fi
}

main "$@"
