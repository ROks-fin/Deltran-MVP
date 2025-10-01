#!/bin/bash

# Test payment submission to DelTran Gateway

echo "🚀 Testing DelTran Payment API..."
echo ""

# Test 1: Health Check
echo "1️⃣ Health Check:"
curl -s http://localhost:8080/health | jq .
echo ""

# Test 2: Create Payment (would work if we had gRPC endpoint)
echo "2️⃣ Submit Payment (requires gRPC implementation):"
echo "POST /api/v1/payments - Not implemented yet (gRPC only)"
echo ""

# Test 3: Metrics
echo "3️⃣ Check Metrics:"
curl -s http://localhost:8080/metrics | grep gateway_queue_depth
echo ""

echo "✅ Gateway is running!"
echo "📊 Dashboard: http://localhost:8080/dashboard.html"
