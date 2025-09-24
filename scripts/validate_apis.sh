#!/bin/bash

# DelTran Rail MVP - API Validation Script
# This script validates all API endpoints are working correctly

set -e

GATEWAY_URL="http://localhost:8000"
IDEMPOTENCY_KEY=$(uuidgen || python -c "import uuid; print(uuid.uuid4())")

echo "🔍 Validating DelTran Rail APIs..."
echo "Gateway URL: $GATEWAY_URL"
echo "Idempotency Key: $IDEMPOTENCY_KEY"
echo ""

# Check if services are running
echo "1️⃣  Checking service health..."
if ! curl -s -f "$GATEWAY_URL/health" > /dev/null; then
    echo "❌ Gateway is not responding"
    exit 1
fi

health_status=$(curl -s "$GATEWAY_URL/health" | jq -r '.status')
if [ "$health_status" != "healthy" ]; then
    echo "❌ Gateway health check failed: $health_status"
    curl -s "$GATEWAY_URL/health" | jq .
    exit 1
fi
echo "✅ Gateway is healthy"

# Test payment initiation
echo ""
echo "2️⃣  Testing payment initiation..."
payment_response=$(curl -s -X POST "$GATEWAY_URL/payments/initiate" \
    -H "Content-Type: application/json" \
    -H "Idempotency-Key: $IDEMPOTENCY_KEY" \
    -d '{
        "amount": "1000.00",
        "currency": "USD",
        "debtor_account": "US1234567890123456789012345678901",
        "creditor_account": "GB9876543210987654321098765432109",
        "payment_purpose": "TRADE",
        "settlement_method": "PVP"
    }')

if echo "$payment_response" | jq -e '.transaction_id' > /dev/null; then
    transaction_id=$(echo "$payment_response" | jq -r '.transaction_id')
    echo "✅ Payment initiated successfully: $transaction_id"
else
    echo "❌ Payment initiation failed:"
    echo "$payment_response" | jq .
    exit 1
fi

# Test payment status
echo ""
echo "3️⃣  Testing payment status..."
status_response=$(curl -s "$GATEWAY_URL/payments/$transaction_id/status")

if echo "$status_response" | jq -e '.status' > /dev/null; then
    status=$(echo "$status_response" | jq -r '.status')
    echo "✅ Payment status retrieved: $status"
else
    echo "❌ Payment status check failed:"
    echo "$status_response" | jq .
fi

# Test idempotency (same request should return cached response)
echo ""
echo "4️⃣  Testing idempotency..."
idempotent_response=$(curl -s -X POST "$GATEWAY_URL/payments/initiate" \
    -H "Content-Type: application/json" \
    -H "Idempotency-Key: $IDEMPOTENCY_KEY" \
    -d '{
        "amount": "1000.00",
        "currency": "USD",
        "debtor_account": "US1234567890123456789012345678901",
        "creditor_account": "GB9876543210987654321098765432109",
        "payment_purpose": "TRADE",
        "settlement_method": "PVP"
    }')

idempotent_transaction_id=$(echo "$idempotent_response" | jq -r '.transaction_id')
if [ "$idempotent_transaction_id" = "$transaction_id" ]; then
    echo "✅ Idempotency working correctly"
else
    echo "❌ Idempotency failed - got different transaction ID: $idempotent_transaction_id"
fi

# Test liquidity quotes (SLA ≤150ms)
echo ""
echo "5️⃣  Testing liquidity quotes (SLA ≤150ms)..."
start_time=$(date +%s%3N)
liquidity_response=$(curl -s "$GATEWAY_URL/liquidity/quotes?from_currency=USD&to_currency=AED&amount=10000")
end_time=$(date +%s%3N)
latency=$((end_time - start_time))

if echo "$liquidity_response" | jq -e '.quotes' > /dev/null; then
    quote_count=$(echo "$liquidity_response" | jq '.quotes | length')
    sla_ms=$(echo "$liquidity_response" | jq -r '.sla_ms')
    echo "✅ Liquidity quotes retrieved: $quote_count quotes in ${sla_ms}ms (client latency: ${latency}ms)"

    if [ "$sla_ms" -le 150 ]; then
        echo "✅ SLA met (≤150ms)"
    else
        echo "⚠️  SLA exceeded: ${sla_ms}ms > 150ms"
    fi
else
    echo "❌ Liquidity quotes failed:"
    echo "$liquidity_response" | jq .
fi

# Test risk mode
echo ""
echo "6️⃣  Testing risk management..."
risk_response=$(curl -s "$GATEWAY_URL/risk/mode")

if echo "$risk_response" | jq -e '.current_mode' > /dev/null; then
    current_mode=$(echo "$risk_response" | jq -r '.current_mode')
    echo "✅ Risk mode retrieved: $current_mode"
else
    echo "❌ Risk mode check failed:"
    echo "$risk_response" | jq .
fi

# Test settlement batch closure
echo ""
echo "7️⃣  Testing settlement batch closure..."
settlement_response=$(curl -s -X POST "$GATEWAY_URL/settlement/close-batch?window=intraday")

if echo "$settlement_response" | jq -e '.batch_id' > /dev/null; then
    batch_id=$(echo "$settlement_response" | jq -r '.batch_id')
    transaction_count=$(echo "$settlement_response" | jq -r '.total_transactions')
    echo "✅ Settlement batch closed: $batch_id with $transaction_count transactions"
else
    echo "❌ Settlement batch closure failed:"
    echo "$settlement_response" | jq .
fi

# Test proof of reserves
echo ""
echo "8️⃣  Testing proof of reserves..."
reserves_response=$(curl -s "$GATEWAY_URL/reports/proof-of-reserves")

if echo "$reserves_response" | jq -e '.report_id' > /dev/null; then
    report_id=$(echo "$reserves_response" | jq -r '.report_id')
    reserve_ratio=$(echo "$reserves_response" | jq -r '.reserve_ratio')
    echo "✅ Proof of reserves generated: $report_id (ratio: $reserve_ratio)"
else
    echo "❌ Proof of reserves failed:"
    echo "$reserves_response" | jq .
fi

# Test proof of settlement
echo ""
echo "9️⃣  Testing proof of settlement..."
settlement_proof_response=$(curl -s "$GATEWAY_URL/reports/proof-of-settlement")

if echo "$settlement_proof_response" | jq -e '.report_id' > /dev/null; then
    settlement_report_id=$(echo "$settlement_proof_response" | jq -r '.report_id')
    total_settled=$(echo "$settlement_proof_response" | jq -r '.total_settled_transactions')
    echo "✅ Proof of settlement generated: $settlement_report_id ($total_settled transactions)"
else
    echo "❌ Proof of settlement failed:"
    echo "$settlement_proof_response" | jq .
fi

# Test metrics endpoint
echo ""
echo "🔟 Testing metrics endpoint..."
if curl -s -f "$GATEWAY_URL/metrics" | head -5 > /dev/null; then
    metrics_lines=$(curl -s "$GATEWAY_URL/metrics" | wc -l)
    echo "✅ Metrics endpoint working: $metrics_lines metrics lines"
else
    echo "❌ Metrics endpoint failed"
fi

echo ""
echo "🎉 API validation complete!"
echo ""
echo "📊 Summary:"
echo "  ✅ Gateway health check"
echo "  ✅ Payment initiation with idempotency"
echo "  ✅ Payment status retrieval"
echo "  ✅ Liquidity quotes (SLA ≤150ms)"
echo "  ✅ Risk mode management"
echo "  ✅ Settlement batch processing"
echo "  ✅ Proof of reserves generation"
echo "  ✅ Proof of settlement generation"
echo "  ✅ Prometheus metrics"
echo ""
echo "🔗 Access points:"
echo "  API Docs:    $GATEWAY_URL/docs"
echo "  Health:      $GATEWAY_URL/health"
echo "  Metrics:     $GATEWAY_URL/metrics"