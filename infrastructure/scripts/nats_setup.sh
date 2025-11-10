#!/bin/bash
# NATS JetStream Stream Setup for DelTran MVP
# This script creates all required streams with proper retention policies

set -e

NATS_URL="nats://localhost:4222"

echo "=== Setting up NATS JetStream Streams for DelTran ===" echo ""

# Function to create stream
create_stream() {
    local stream_name=$1
    local subjects=$2
    local retention_days=$3
    local description=$4

    echo "Creating stream: $stream_name"
    echo "  Subjects: $subjects"
    echo "  Retention: $retention_days days"

    # Convert days to nanoseconds (days * 24 * 60 * 60 * 1000000000)
    local max_age=$((retention_days * 86400 * 1000000000))

    # Create stream using NATS CLI (if available) or HTTP API
    curl -s -X POST "http://localhost:8222/jsz/api/v1/stream/add/$stream_name" \
        -H "Content-Type: application/json" \
        -d "{
            \"name\": \"$stream_name\",
            \"subjects\": [$subjects],
            \"retention\": \"limits\",
            \"max_age\": $max_age,
            \"storage\": \"file\",
            \"num_replicas\": 1,
            \"discard\": \"old\",
            \"max_msgs_per_subject\": -1,
            \"max_bytes\": -1,
            \"max_msg_size\": 8388608,
            \"duplicate_window\": 120000000000,
            \"allow_rollup_hdrs\": false
        }" || echo "  (Stream may already exist)"

    echo ""
}

# Create streams according to COMPLETE_SYSTEM_SPECIFICATION.md

# 1. Transactions Stream (7 day retention)
create_stream "transactions" \
    "\"transactions.created\",\"transactions.completed\",\"transactions.failed\"" \
    7 \
    "Transaction lifecycle events"

# 2. Settlement Stream (30 day retention - critical for audit)
create_stream "settlement" \
    "\"settlement.initiated\",\"settlement.completed\",\"settlement.rolled_back\",\"settlement.reconciled\"" \
    30 \
    "Settlement and reconciliation events"

# 3. Compliance Stream (90 day retention - regulatory requirement)
create_stream "compliance" \
    "\"compliance.alert\",\"compliance.sar\",\"compliance.ctr\",\"compliance.check_completed\"" \
    90 \
    "Compliance and AML events"

# 4. Notifications Stream (1 day retention - ephemeral)
create_stream "notifications" \
    "\"notifications.*\"" \
    1 \
    "Real-time notification events"

# 5. Clearing Stream (30 day retention)
create_stream "clearing" \
    "\"clearing.window_opened\",\"clearing.window_closed\",\"clearing.netting_completed\"" \
    30 \
    "Clearing window and netting events"

# 6. Risk Events Stream (7 day retention)
create_stream "risk" \
    "\"risk.evaluation\",\"risk.threshold_exceeded\",\"risk.circuit_breaker\"" \
    7 \
    "Risk management events"

# 7. Audit Log Stream (90 day retention - immutable audit trail)
create_stream "audit" \
    "\"audit.*\"" \
    90 \
    "System-wide audit trail"

echo "=== NATS JetStream Setup Complete ==="
echo ""
echo "To verify streams, run:"
echo "  curl http://localhost:8222/jsz"
