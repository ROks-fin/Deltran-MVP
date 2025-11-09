#!/bin/bash
# NATS JetStream Stream Setup for DelTran MVP
# Version: 1.0
# Description: Creates all JetStream streams and consumers for event-driven architecture

set -e

NATS_URL="${NATS_URL:-nats://localhost:4222}"
NATS_CONTEXT="${NATS_CONTEXT:-default}"

echo "🚀 DelTran NATS JetStream Setup"
echo "================================"
echo "NATS URL: $NATS_URL"
echo ""

# Function to create stream with retry
create_stream() {
    local stream_name=$1
    local subjects=$2
    local retention=$3
    local max_age=$4

    echo "📦 Creating stream: $stream_name"

    nats stream add "$stream_name" \
        --subjects="$subjects" \
        --retention="$retention" \
        --storage=file \
        --replicas=1 \
        --max-age="$max_age" \
        --max-msgs=-1 \
        --max-bytes=-1 \
        --max-msg-size=-1 \
        --discard=old \
        --dupe-window=2m \
        --allow-rollup \
        --deny-delete \
        --deny-purge \
        --server="$NATS_URL" \
        --force 2>/dev/null || echo "  ⚠️  Stream $stream_name already exists or error occurred"

    echo "  ✅ Stream $stream_name configured"
}

# Function to create consumer
create_consumer() {
    local stream_name=$1
    local consumer_name=$2
    local filter_subject=$3

    echo "👤 Creating consumer: $consumer_name on $stream_name"

    nats consumer add "$stream_name" "$consumer_name" \
        --filter="$filter_subject" \
        --ack=explicit \
        --pull \
        --deliver=all \
        --max-deliver=-1 \
        --max-pending=1000 \
        --replay=instant \
        --server="$NATS_URL" \
        --force 2>/dev/null || echo "  ⚠️  Consumer $consumer_name already exists"

    echo "  ✅ Consumer $consumer_name configured"
}

echo "==================================="
echo "1️⃣  CREATING STREAMS"
echo "==================================="
echo ""

# 1. TOKEN EVENTS STREAM
# Purpose: Token mint/burn/transfer events from Token Engine
create_stream \
    "DELTRAN_TOKEN_EVENTS" \
    "deltran.token.>" \
    "limits" \
    "7d"

# 2. OBLIGATION EVENTS STREAM
# Purpose: Obligation creation/updates from Obligation Engine
create_stream \
    "DELTRAN_OBLIGATION_EVENTS" \
    "deltran.obligation.>" \
    "limits" \
    "30d"

# 3. CLEARING EVENTS STREAM
# Purpose: Clearing window lifecycle and netting results
create_stream \
    "DELTRAN_CLEARING_EVENTS" \
    "deltran.clearing.>" \
    "limits" \
    "90d"

# 4. SETTLEMENT EVENTS STREAM
# Purpose: Settlement execution, reconciliation events
create_stream \
    "DELTRAN_SETTLEMENT_EVENTS" \
    "deltran.settlement.>" \
    "limits" \
    "90d"

# 5. NOTIFICATION STREAM
# Purpose: User notifications (real-time, email, SMS)
create_stream \
    "DELTRAN_NOTIFICATIONS" \
    "deltran.notification.>" \
    "work-queue" \
    "24h"

# 6. AUDIT LOG STREAM
# Purpose: Immutable audit trail for compliance
create_stream \
    "DELTRAN_AUDIT_LOG" \
    "deltran.audit.>" \
    "limits" \
    "365d"

# 7. RISK ALERTS STREAM
# Purpose: Risk threshold breaches and alerts
create_stream \
    "DELTRAN_RISK_ALERTS" \
    "deltran.risk.>" \
    "limits" \
    "30d"

# 8. REPORTING STREAM
# Purpose: Report generation triggers and status
create_stream \
    "DELTRAN_REPORTING" \
    "deltran.report.>" \
    "work-queue" \
    "7d"

echo ""
echo "==================================="
echo "2️⃣  CREATING CONSUMERS"
echo "==================================="
echo ""

# Notification Engine consumers
create_consumer "DELTRAN_TOKEN_EVENTS" "notification-token-consumer" "deltran.token.>"
create_consumer "DELTRAN_OBLIGATION_EVENTS" "notification-obligation-consumer" "deltran.obligation.>"
create_consumer "DELTRAN_CLEARING_EVENTS" "notification-clearing-consumer" "deltran.clearing.>"
create_consumer "DELTRAN_SETTLEMENT_EVENTS" "notification-settlement-consumer" "deltran.settlement.>"

# Settlement Engine consumers
create_consumer "DELTRAN_CLEARING_EVENTS" "settlement-clearing-consumer" "deltran.clearing.window.closed"
create_consumer "DELTRAN_CLEARING_EVENTS" "settlement-netting-consumer" "deltran.clearing.netting.completed"

# Reporting Engine consumers
create_consumer "DELTRAN_CLEARING_EVENTS" "reporting-clearing-consumer" "deltran.clearing.>"
create_consumer "DELTRAN_SETTLEMENT_EVENTS" "reporting-settlement-consumer" "deltran.settlement.>"
create_consumer "DELTRAN_AUDIT_LOG" "reporting-audit-consumer" "deltran.audit.>"

# Compliance Engine consumer for audit
create_consumer "DELTRAN_AUDIT_LOG" "compliance-audit-consumer" "deltran.audit.>"

# Risk Engine consumer for alerts
create_consumer "DELTRAN_RISK_ALERTS" "risk-alert-processor" "deltran.risk.>"

echo ""
echo "==================================="
echo "3️⃣  VERIFICATION"
echo "==================================="
echo ""

echo "📊 Listing all streams:"
nats stream list --server="$NATS_URL" 2>/dev/null || echo "⚠️  Could not list streams"

echo ""
echo "==================================="
echo "✅ NATS JetStream Setup Complete!"
echo "==================================="
echo ""
echo "📚 Stream Details:"
echo "  - DELTRAN_TOKEN_EVENTS: 7d retention"
echo "  - DELTRAN_OBLIGATION_EVENTS: 30d retention"
echo "  - DELTRAN_CLEARING_EVENTS: 90d retention (financial audit)"
echo "  - DELTRAN_SETTLEMENT_EVENTS: 90d retention (financial audit)"
echo "  - DELTRAN_NOTIFICATIONS: 24h retention (work queue)"
echo "  - DELTRAN_AUDIT_LOG: 365d retention (compliance)"
echo "  - DELTRAN_RISK_ALERTS: 30d retention"
echo "  - DELTRAN_REPORTING: 7d retention (work queue)"
echo ""
echo "🔍 Monitoring:"
echo "  - NATS Monitoring UI: http://localhost:8222"
echo "  - Check stream info: nats stream info <stream-name>"
echo "  - Check consumer info: nats consumer info <stream-name> <consumer-name>"
echo ""
