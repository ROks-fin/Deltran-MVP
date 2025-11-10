#!/bin/bash

# K6 Test Runner for DelTran MVP
# Runs all K6 performance tests and generates reports

set -e

echo "=========================================="
echo "DelTran MVP - K6 Performance Test Runner"
echo "=========================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create results directory
RESULTS_DIR="./results"
mkdir -p "$RESULTS_DIR"

# Timestamp for this test run
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
RUN_DIR="$RESULTS_DIR/run_$TIMESTAMP"
mkdir -p "$RUN_DIR"

echo "📁 Results directory: $RUN_DIR"
echo ""

# Check if K6 is installed
if ! command -v k6 &> /dev/null; then
    echo -e "${RED}❌ K6 is not installed!${NC}"
    echo "Install K6: https://k6.io/docs/getting-started/installation/"
    exit 1
fi

echo -e "${GREEN}✅ K6 is installed${NC}"
echo ""

# Function to run a test
run_test() {
    local test_name=$1
    local test_file=$2
    local output_file=$3

    echo "=========================================="
    echo -e "${YELLOW}🧪 Running: $test_name${NC}"
    echo "=========================================="

    if k6 run --out json="$output_file" "$test_file"; then
        echo -e "${GREEN}✅ $test_name completed successfully${NC}"
        echo ""
        return 0
    else
        echo -e "${RED}❌ $test_name failed${NC}"
        echo ""
        return 1
    fi
}

# Test execution tracker
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 1. Integration Test - Health Checks
TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test "Integration Test - Health Checks" \
    "./scenarios/integration-test.js" \
    "$RUN_DIR/integration.json"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# 2. E2E Transaction Flow Test
TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test "E2E Transaction Flow Test" \
    "./scenarios/e2e-transaction.js" \
    "$RUN_DIR/e2e.json"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# 3. Load Test - Realistic Scenarios
TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test "Load Test - Realistic Scenarios" \
    "./scenarios/load-test-realistic.js" \
    "$RUN_DIR/load.json"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# 4. WebSocket Test - Notification Engine
TOTAL_TESTS=$((TOTAL_TESTS + 1))
if run_test "WebSocket Test - Notification Engine" \
    "./scenarios/websocket-test.js" \
    "$RUN_DIR/websocket.json"; then
    PASSED_TESTS=$((PASSED_TESTS + 1))
else
    FAILED_TESTS=$((FAILED_TESTS + 1))
fi

# Final Summary
echo "=========================================="
echo "📊 Test Run Summary"
echo "=========================================="
echo "Total Tests:  $TOTAL_TESTS"
echo -e "${GREEN}Passed:       $PASSED_TESTS${NC}"
if [ $FAILED_TESTS -gt 0 ]; then
    echo -e "${RED}Failed:       $FAILED_TESTS${NC}"
else
    echo "Failed:       $FAILED_TESTS"
fi
echo ""
echo "📁 Results saved to: $RUN_DIR"
echo ""

# Generate HTML report (if K6 HTML reporter is available)
if command -v k6-to-junit &> /dev/null; then
    echo "📄 Generating HTML reports..."
    for json_file in "$RUN_DIR"/*.json; do
        if [ -f "$json_file" ]; then
            base_name=$(basename "$json_file" .json)
            k6-to-junit "$json_file" > "$RUN_DIR/${base_name}.xml" 2>/dev/null || true
        fi
    done
    echo -e "${GREEN}✅ Reports generated${NC}"
else
    echo -e "${YELLOW}⚠️  k6-to-junit not found. Install for XML/HTML reports: npm install -g k6-to-junit${NC}"
fi

echo ""
echo "=========================================="
echo "🏁 Test run completed!"
echo "=========================================="

# Exit with error code if any tests failed
if [ $FAILED_TESTS -gt 0 ]; then
    exit 1
else
    exit 0
fi
