#!/bin/bash

# DelTran MVP - Comprehensive Service Testing
# Tests all microservices and their core functionality

echo "========================================="
echo "DelTran MVP - Comprehensive Testing"
echo "========================================="
echo ""

# Test UUIDs
BANK_ICICI="550e8400-e29b-41d4-a716-446655440001"
BANK_HDFC="550e8400-e29b-41d4-a716-446655440002"
BANK_AXIS="550e8400-e29b-41d4-a716-446655440003"
TRANSACTION_ID="550e8400-e29b-41d4-a716-001763689100"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

test_passed=0
test_failed=0

function test_service() {
    local service_name=$1
    local test_name=$2
    local command=$3

    echo -n "Testing $service_name - $test_name... "
    response=$(eval "$command" 2>&1)

    if echo "$response" | grep -q '"error"'; then
        echo -e "${RED}FAILED${NC}"
        echo "  Error: $(echo "$response" | python -m json.tool 2>/dev/null | grep -A 2 '"error"' | head -5)"
        ((test_failed++))
        return 1
    else
        echo -e "${GREEN}PASSED${NC}"
        ((test_passed++))
        return 0
    fi
}

echo "=== 1. TOKEN ENGINE TESTING ==="
echo ""

test_service "Token Engine" "Health Check" \
    "curl -s http://localhost:8081/health"

test_service "Token Engine" "Mint USD Tokens" \
    "curl -s -X POST http://localhost:8081/api/v1/tokens/mint -H 'Content-Type: application/json' -d '{\"bank_id\": \"$BANK_AXIS\", \"currency\": \"USD\", \"amount\": \"100000.00\", \"reference\": \"TEST-MINT-001\"}'"

test_service "Token Engine" "Get Balance" \
    "curl -s http://localhost:8081/api/v1/tokens/balances/$BANK_AXIS"

test_service "Token Engine" "Transfer Tokens" \
    "curl -s -X POST http://localhost:8081/api/v1/tokens/transfer -H 'Content-Type: application/json' -d '{\"from_bank_id\": \"$BANK_AXIS\", \"to_bank_id\": \"$BANK_HDFC\", \"currency\": \"USD\", \"amount\": \"10000.00\", \"reference\": \"TEST-TRANSFER-001\"}'"

echo ""
echo "=== 2. LIQUIDITY ROUTER TESTING ==="
echo ""

test_service "Liquidity Router" "Health Check" \
    "curl -s http://localhost:8083/health"

test_service "Liquidity Router" "Predict Liquidity" \
    "curl -s -X POST http://localhost:8083/api/v1/liquidity/predict -H 'Content-Type: application/json' -d '{\"corridor\": \"USD-EUR\", \"amount\": \"50000.00\", \"bank_id\": \"$BANK_ICICI\", \"time_horizon_hours\": 24}'"

echo ""
echo "=== 3. RISK ENGINE TESTING ==="
echo ""

test_service "Risk Engine" "Health Check" \
    "curl -s http://localhost:8084/health"

test_service "Risk Engine" "Evaluate Transaction Risk" \
    "curl -s -X POST http://localhost:8084/api/v1/risk/evaluate -H 'Content-Type: application/json' -d '{\"transaction_id\": \"$TRANSACTION_ID\", \"sender_bank_id\": \"$BANK_ICICI\", \"receiver_bank_id\": \"$BANK_HDFC\", \"amount\": \"50000.00\", \"from_currency\": \"USD\", \"to_currency\": \"USD\", \"sender_country\": \"IN\", \"receiver_country\": \"IN\", \"transaction_type\": \"B2B\"}'"

test_service "Risk Engine" "Get Risk Metrics" \
    "curl -s http://localhost:8084/api/v1/risk/metrics"

echo ""
echo "=== 4. COMPLIANCE ENGINE TESTING ==="
echo ""

test_service "Compliance Engine" "Health Check" \
    "curl -s http://localhost:8086/health"

test_service "Compliance Engine" "AML/KYC Check" \
    "curl -s -X POST http://localhost:8086/api/v1/compliance/check -H 'Content-Type: application/json' -d '{\"transaction_id\": \"$TRANSACTION_ID\", \"sender_name\": \"ICICI Bank\", \"sender_account\": \"ICICI-ACC-001\", \"sender_country\": \"IN\", \"sender_bank_id\": \"$BANK_ICICI\", \"receiver_name\": \"HDFC Bank\", \"receiver_account\": \"HDFC-ACC-001\", \"receiver_country\": \"IN\", \"receiver_bank_id\": \"$BANK_HDFC\", \"amount\": \"50000.00\", \"currency\": \"USD\", \"purpose\": \"Trade settlement\"}'"

echo ""
echo "=== 5. CLEARING ENGINE (OBLIGATION ENGINE) TESTING ==="
echo ""

test_service "Clearing Engine" "Health Check" \
    "curl -s http://localhost:8082/health"

echo ""
echo "=== 6. SETTLEMENT ENGINE TESTING ==="
echo ""

test_service "Settlement Engine" "Health Check" \
    "curl -s http://localhost:8087/health"

test_service "Settlement Engine" "Get Accounts" \
    "curl -s http://localhost:8087/api/v1/accounts"

echo ""
echo "=== 7. REPORTING ENGINE TESTING ==="
echo ""

test_service "Reporting Engine" "Health Check" \
    "curl -s http://localhost:8088/health"

test_service "Reporting Engine" "Transaction Summary" \
    "curl -s http://localhost:8088/api/v1/reports/transaction-summary"

echo ""
echo "=== 8. NOTIFICATION ENGINE TESTING ==="
echo ""

test_service "Notification Engine" "Health Check" \
    "curl -s http://localhost:8089/health"

echo ""
echo "========================================="
echo "TESTING SUMMARY"
echo "========================================="
echo -e "Tests Passed: ${GREEN}$test_passed${NC}"
echo -e "Tests Failed: ${RED}$test_failed${NC}"
echo "Total Tests: $((test_passed + test_failed))"
echo ""

if [ $test_failed -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed${NC}"
    exit 1
fi
