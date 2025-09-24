#!/bin/bash

# DelTran Rail MVP - Acceptance Checks
# This script performs comprehensive acceptance testing

set -e

GATEWAY_URL="http://localhost:8000"
GRAFANA_URL="http://localhost:3000"
JAEGER_URL="http://localhost:16686"
PROMETHEUS_URL="http://localhost:9090"

echo "🎯 Running DelTran Rail MVP Acceptance Checks..."
echo ""

# Track results
PASS=0
FAIL=0
WARN=0

check_pass() {
    echo "✅ $1"
    PASS=$((PASS + 1))
}

check_fail() {
    echo "❌ $1"
    FAIL=$((FAIL + 1))
}

check_warn() {
    echo "⚠️  $1"
    WARN=$((WARN + 1))
}

# 1. Service Availability
echo "1️⃣  Service Availability Tests"
echo "================================"

# Gateway
if curl -s -f "$GATEWAY_URL/health" > /dev/null; then
    health_data=$(curl -s "$GATEWAY_URL/health")
    status=$(echo "$health_data" | jq -r '.status')
    if [ "$status" = "healthy" ]; then
        check_pass "Gateway is healthy and responding"
    else
        check_fail "Gateway health check failed: $status"
    fi
else
    check_fail "Gateway is not responding at $GATEWAY_URL"
fi

# Grafana
if curl -s -f "$GRAFANA_URL/api/health" > /dev/null; then
    check_pass "Grafana is responding"
else
    check_warn "Grafana is not responding at $GRAFANA_URL"
fi

# Jaeger
if curl -s -f "$JAEGER_URL/api/services" > /dev/null; then
    check_pass "Jaeger is responding"
else
    check_warn "Jaeger is not responding at $JAEGER_URL"
fi

# Prometheus
if curl -s -f "$PROMETHEUS_URL/api/v1/status/config" > /dev/null; then
    check_pass "Prometheus is responding"
else
    check_warn "Prometheus is not responding at $PROMETHEUS_URL"
fi

echo ""

# 2. Core API Functionality
echo "2️⃣  Core API Functionality Tests"
echo "===================================="

# Generate test data
IDEMPOTENCY_KEY=$(uuidgen || python -c "import uuid; print(uuid.uuid4())")
TEST_AMOUNT="1000.00"
TEST_CURRENCY="USD"

# Payment initiation
echo "Testing payment initiation..."
payment_data=$(curl -s -X POST "$GATEWAY_URL/payments/initiate" \
    -H "Content-Type: application/json" \
    -H "Idempotency-Key: $IDEMPOTENCY_KEY" \
    -d "{
        \"amount\": \"$TEST_AMOUNT\",
        \"currency\": \"$TEST_CURRENCY\",
        \"debtor_account\": \"US1234567890123456789012345678901\",
        \"creditor_account\": \"GB9876543210987654321098765432109\"
    }")

if echo "$payment_data" | jq -e '.transaction_id' > /dev/null; then
    TRANSACTION_ID=$(echo "$payment_data" | jq -r '.transaction_id')
    check_pass "Payment initiated successfully: $TRANSACTION_ID"

    # Status check
    status_data=$(curl -s "$GATEWAY_URL/payments/$TRANSACTION_ID/status")
    if echo "$status_data" | jq -e '.status' > /dev/null; then
        check_pass "Payment status retrieved successfully"
    else
        check_fail "Payment status retrieval failed"
    fi
else
    check_fail "Payment initiation failed"
    echo "$payment_data" | jq .
fi

# Idempotency test
echo "Testing idempotency..."
duplicate_payment_data=$(curl -s -X POST "$GATEWAY_URL/payments/initiate" \
    -H "Content-Type: application/json" \
    -H "Idempotency-Key: $IDEMPOTENCY_KEY" \
    -d "{
        \"amount\": \"$TEST_AMOUNT\",
        \"currency\": \"$TEST_CURRENCY\",
        \"debtor_account\": \"US1234567890123456789012345678901\",
        \"creditor_account\": \"GB9876543210987654321098765432109\"
    }")

duplicate_id=$(echo "$duplicate_payment_data" | jq -r '.transaction_id')
if [ "$duplicate_id" = "$TRANSACTION_ID" ]; then
    check_pass "Idempotency working correctly"
else
    check_fail "Idempotency failed - different transaction ID returned"
fi

echo ""

# 3. Liquidity SLA Testing
echo "3️⃣  Liquidity SLA Testing (≤150ms)"
echo "===================================="

for i in {1..5}; do
    start_time=$(date +%s%3N)
    liquidity_data=$(curl -s "$GATEWAY_URL/liquidity/quotes?from_currency=USD&to_currency=AED&amount=10000")
    end_time=$(date +%s%3N)
    client_latency=$((end_time - start_time))

    if echo "$liquidity_data" | jq -e '.sla_ms' > /dev/null; then
        server_sla=$(echo "$liquidity_data" | jq -r '.sla_ms')
        quote_count=$(echo "$liquidity_data" | jq '.quotes | length')

        if [ "$server_sla" -le 150 ]; then
            check_pass "Liquidity SLA met: ${server_sla}ms (${quote_count} quotes)"
        else
            check_fail "Liquidity SLA exceeded: ${server_sla}ms > 150ms"
        fi
    else
        check_fail "Liquidity quote failed on attempt $i"
    fi
done

echo ""

# 4. Risk Management Testing
echo "4️⃣  Risk Management Testing"
echo "============================="

# Check current risk mode
risk_data=$(curl -s "$GATEWAY_URL/risk/mode")
if echo "$risk_data" | jq -e '.current_mode' > /dev/null; then
    current_mode=$(echo "$risk_data" | jq -r '.current_mode')
    check_pass "Risk mode retrieved: $current_mode"

    # Test mode switching
    for mode in "High" "Low" "Medium"; do
        switch_result=$(curl -s -X POST "$GATEWAY_URL/risk/mode" \
            -H "Content-Type: application/json" \
            -d "{\"mode\": \"$mode\", \"reason\": \"Acceptance test\"}")

        if echo "$switch_result" | jq -e '.current_mode' > /dev/null; then
            new_mode=$(echo "$switch_result" | jq -r '.current_mode')
            if [ "$new_mode" = "$mode" ]; then
                check_pass "Risk mode switched to $mode"
            else
                check_fail "Risk mode switch failed - expected $mode, got $new_mode"
            fi
        else
            check_fail "Risk mode switch to $mode failed"
        fi
    done
else
    check_fail "Risk mode retrieval failed"
fi

echo ""

# 5. Settlement Testing
echo "5️⃣  Settlement Testing"
echo "======================="

settlement_data=$(curl -s -X POST "$GATEWAY_URL/settlement/close-batch?window=intraday")
if echo "$settlement_data" | jq -e '.batch_id' > /dev/null; then
    batch_id=$(echo "$settlement_data" | jq -r '.batch_id')
    transaction_count=$(echo "$settlement_data" | jq -r '.total_transactions')
    check_pass "Settlement batch closed: $batch_id ($transaction_count transactions)"
else
    check_warn "Settlement batch closure returned no transactions (expected for new system)"
fi

# Settlement status
status_data=$(curl -s "$GATEWAY_URL/settlement/status")
if echo "$status_data" | jq -e '.net_positions' > /dev/null; then
    check_pass "Settlement status retrieved successfully"
else
    check_fail "Settlement status retrieval failed"
fi

echo ""

# 6. Reporting Testing
echo "6️⃣  Reporting Testing"
echo "====================="

# Proof of reserves
reserves_data=$(curl -s "$GATEWAY_URL/reports/proof-of-reserves")
if echo "$reserves_data" | jq -e '.report_id' > /dev/null; then
    report_id=$(echo "$reserves_data" | jq -r '.report_id')
    attestation_hash=$(echo "$reserves_data" | jq -r '.attestation_hash')
    check_pass "Proof of reserves generated: $report_id"

    # Validate attestation hash format
    if [[ "$attestation_hash" =~ ^[a-f0-9]{64}$ ]]; then
        check_pass "Attestation hash format valid"
    else
        check_fail "Attestation hash format invalid: $attestation_hash"
    fi
else
    check_fail "Proof of reserves generation failed"
fi

# Proof of settlement
settlement_proof_data=$(curl -s "$GATEWAY_URL/reports/proof-of-settlement")
if echo "$settlement_proof_data" | jq -e '.report_id' > /dev/null; then
    settlement_report_id=$(echo "$settlement_proof_data" | jq -r '.report_id')
    iso20022_manifest=$(echo "$settlement_proof_data" | jq -e '.iso20022_manifest')
    check_pass "Proof of settlement generated: $settlement_report_id"

    # Validate ISO20022 manifest
    if echo "$settlement_proof_data" | jq -e '.iso20022_manifest.message_type' > /dev/null; then
        message_type=$(echo "$settlement_proof_data" | jq -r '.iso20022_manifest.message_type')
        check_pass "ISO20022 manifest valid: $message_type"
    else
        check_fail "ISO20022 manifest missing or invalid"
    fi
else
    check_fail "Proof of settlement generation failed"
fi

echo ""

# 7. Observability Testing
echo "7️⃣  Observability Testing"
echo "=========================="

# Metrics endpoint
if curl -s -f "$GATEWAY_URL/metrics" > /dev/null; then
    metrics_content=$(curl -s "$GATEWAY_URL/metrics")

    # Check for key metrics
    if echo "$metrics_content" | grep -q "http_requests_total"; then
        check_pass "HTTP request metrics available"
    else
        check_fail "HTTP request metrics missing"
    fi

    if echo "$metrics_content" | grep -q "payments_total"; then
        check_pass "Payment metrics available"
    else
        check_fail "Payment metrics missing"
    fi

    if echo "$metrics_content" | grep -q "http_request_duration_seconds"; then
        check_pass "Latency metrics available"
    else
        check_fail "Latency metrics missing"
    fi
else
    check_fail "Metrics endpoint not accessible"
fi

echo ""

# 8. Data Quality Testing
echo "8️⃣  Data Quality Testing"
echo "========================"

# Transaction report
report_data=$(curl -s "$GATEWAY_URL/reports/transactions?limit=10")
if echo "$report_data" | jq -e '.transactions' > /dev/null; then
    transaction_count=$(echo "$report_data" | jq '.transactions | length')
    check_pass "Transaction report generated: $transaction_count transactions"

    # Check for required fields
    if echo "$report_data" | jq -e '.transactions[0].transaction_id' > /dev/null 2>&1; then
        check_pass "Transaction data structure valid"
    else
        check_warn "No transaction data available (expected for new system)"
    fi
else
    check_fail "Transaction report generation failed"
fi

# Compliance report
compliance_data=$(curl -s "$GATEWAY_URL/reports/compliance")
if echo "$compliance_data" | jq -e '.report_id' > /dev/null; then
    compliance_rate=$(echo "$compliance_data" | jq -r '.compliance_rate')
    check_pass "Compliance report generated: ${compliance_rate}% compliance rate"
else
    check_fail "Compliance report generation failed"
fi

echo ""

# Final Summary
echo "📋 ACCEPTANCE TEST SUMMARY"
echo "=========================="
echo "✅ Passed: $PASS"
echo "❌ Failed: $FAIL"
echo "⚠️  Warnings: $WARN"
echo ""

TOTAL=$((PASS + FAIL))
if [ $TOTAL -gt 0 ]; then
    SUCCESS_RATE=$((PASS * 100 / TOTAL))
    echo "Success Rate: ${SUCCESS_RATE}%"
else
    SUCCESS_RATE=0
    echo "Success Rate: 0% (no tests executed)"
fi

echo ""

if [ $FAIL -eq 0 ]; then
    echo "🎉 ALL ACCEPTANCE TESTS PASSED!"
    echo ""
    echo "🔗 Access Points:"
    echo "  Gateway API:     $GATEWAY_URL"
    echo "  API Docs:        $GATEWAY_URL/docs"
    echo "  Grafana:         $GRAFANA_URL (admin/admin)"
    echo "  Jaeger:          $JAEGER_URL"
    echo "  Prometheus:      $PROMETHEUS_URL"
    echo ""
    echo "✨ The DelTran Rail MVP is ready for demonstration!"
    exit 0
else
    echo "❌ SOME TESTS FAILED - Review the failures above"
    exit 1
fi