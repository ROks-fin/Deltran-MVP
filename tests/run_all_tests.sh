#!/bin/bash

# DelTran System - Comprehensive Test Suite Runner
# This script runs all tests in sequence and generates a report

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

GATEWAY_URL="http://localhost:8080"
TEST_RESULTS_DIR="$SCRIPT_DIR/results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   DelTran System - Comprehensive Test Suite${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""

# Create results directory
mkdir -p "$TEST_RESULTS_DIR"

# Check if gateway is running
echo -e "${YELLOW}[1/5] Checking Gateway Status...${NC}"
if curl -s "$GATEWAY_URL/health" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Gateway is running at $GATEWAY_URL${NC}"
else
    echo -e "${RED}✗ Gateway is not running at $GATEWAY_URL${NC}"
    echo -e "${YELLOW}Please start the gateway service first:${NC}"
    echo "  cd gateway && cargo run --release"
    exit 1
fi
echo ""

# Run Component Tests
echo -e "${YELLOW}[2/5] Running Component Tests...${NC}"
python3 "$SCRIPT_DIR/component_test_suite.py" 2>&1 | tee "$TEST_RESULTS_DIR/component_tests_$TIMESTAMP.log"
COMPONENT_TEST_RESULT=$?
echo ""

if [ $COMPONENT_TEST_RESULT -eq 0 ]; then
    echo -e "${GREEN}✓ Component tests completed${NC}"
else
    echo -e "${YELLOW}⚠ Component tests completed with warnings${NC}"
fi
echo ""

# Run Multi-Bank Stress Test
echo -e "${YELLOW}[3/5] Running Multi-Bank Integration Stress Test...${NC}"
echo -e "${BLUE}This will take approximately 5 minutes...${NC}"
python3 "$SCRIPT_DIR/stress_test_multibank.py" 2>&1 | tee "$TEST_RESULTS_DIR/stress_test_$TIMESTAMP.log"
STRESS_TEST_RESULT=$?
echo ""

if [ $STRESS_TEST_RESULT -eq 0 ]; then
    echo -e "${GREEN}✓ Stress test completed${NC}"
else
    echo -e "${RED}✗ Stress test failed${NC}"
fi
echo ""

# Check Gateway Metrics
echo -e "${YELLOW}[4/5] Checking System Metrics...${NC}"
METRICS=$(curl -s "$GATEWAY_URL/api/v1/metrics/live" 2>/dev/null)
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Metrics endpoint responsive${NC}"
    echo "$METRICS" | jq '.' 2>/dev/null || echo "$METRICS"
else
    echo -e "${RED}✗ Failed to fetch metrics${NC}"
fi
echo ""

# Check Recent Transactions
echo -e "${YELLOW}[5/5] Checking Recent Transactions...${NC}"
TRANSACTIONS=$(curl -s "$GATEWAY_URL/api/payments" 2>/dev/null)
if [ $? -eq 0 ]; then
    PAYMENT_COUNT=$(echo "$TRANSACTIONS" | jq '.payments | length' 2>/dev/null || echo "0")
    echo -e "${GREEN}✓ Found $PAYMENT_COUNT payments in system${NC}"
else
    echo -e "${RED}✗ Failed to fetch transactions${NC}"
fi
echo ""

# Generate Summary Report
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   TEST SUMMARY${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo "Test Run: $TIMESTAMP"
echo "Gateway URL: $GATEWAY_URL"
echo ""
echo "Results:"

if [ $COMPONENT_TEST_RESULT -eq 0 ]; then
    echo -e "  ${GREEN}✓${NC} Component Tests"
else
    echo -e "  ${YELLOW}⚠${NC} Component Tests (with warnings)"
fi

if [ $STRESS_TEST_RESULT -eq 0 ]; then
    echo -e "  ${GREEN}✓${NC} Stress Test"
else
    echo -e "  ${RED}✗${NC} Stress Test"
fi

echo ""
echo "Log files saved to: $TEST_RESULTS_DIR"
echo ""

# Overall result
if [ $COMPONENT_TEST_RESULT -eq 0 ] && [ $STRESS_TEST_RESULT -eq 0 ]; then
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}   ALL TESTS PASSED ✓${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    exit 0
else
    echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${YELLOW}   TESTS COMPLETED WITH ISSUES${NC}"
    echo -e "${YELLOW}═══════════════════════════════════════════════════════════════${NC}"
    exit 1
fi
