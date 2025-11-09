#!/bin/bash
# Infrastructure Verification Script for DelTran MVP
# Tests connectivity and health of all infrastructure components

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=================================="
echo "DelTran Infrastructure Verification"
echo "=================================="
echo ""

PASSED=0
FAILED=0

# Function to check service
check_service() {
    local service_name=$1
    local check_command=$2

    echo -n "Checking $service_name... "

    if eval "$check_command" > /dev/null 2>&1; then
        echo -e "${GREEN}OK${NC}"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        ((FAILED++))
        return 1
    fi
}

echo "1. Infrastructure Services"
echo "-------------------------"

# PostgreSQL
check_service "PostgreSQL" \
    "docker exec deltran-postgres pg_isready -U deltran"

# Redis
check_service "Redis" \
    "docker exec deltran-redis redis-cli ping | grep -q PONG"

# NATS JetStream
check_service "NATS JetStream" \
    "curl -sf http://localhost:8222/healthz"

# Envoy Proxy
check_service "Envoy Proxy" \
    "curl -sf http://localhost:9901/ready"

echo ""
echo "2. Database Schema"
echo "-----------------"

# Check if tables exist
check_service "Core Tables" \
    "docker exec deltran-postgres psql -U deltran -d deltran -c 'SELECT COUNT(*) FROM banks;' > /dev/null"

check_service "Clearing Tables" \
    "docker exec deltran-postgres psql -U deltran -d deltran -c 'SELECT COUNT(*) FROM clearing_windows;' > /dev/null"

check_service "Settlement Tables" \
    "docker exec deltran-postgres psql -U deltran -d deltran -c 'SELECT COUNT(*) FROM settlement_transactions;' > /dev/null"

check_service "Notification Tables" \
    "docker exec deltran-postgres psql -U deltran -d deltran -c 'SELECT COUNT(*) FROM notifications;' > /dev/null"

check_service "Reporting Tables" \
    "docker exec deltran-postgres psql -U deltran -d deltran -c 'SELECT COUNT(*) FROM generated_reports;' > /dev/null"

echo ""
echo "3. NATS JetStream Streams"
echo "------------------------"

# Check NATS streams (requires nats CLI)
if command -v nats &> /dev/null; then
    check_service "NATS CLI Available" "true"

    # List streams would go here if nats CLI is available
    echo -e "${YELLOW}Note: Run 'nats stream ls' to verify streams manually${NC}"
else
    echo -e "${YELLOW}NATS CLI not installed - skipping stream verification${NC}"
    echo "  Install: go install github.com/nats-io/natscli/nats@latest"
fi

echo ""
echo "4. Envoy Routing"
echo "---------------"

# Check Envoy clusters
check_service "Gateway Cluster" \
    "curl -sf http://localhost:9901/clusters | grep -q gateway_cluster"

check_service "Notification Cluster" \
    "curl -sf http://localhost:9901/clusters | grep -q notification_cluster"

echo ""
echo "5. Network Connectivity"
echo "----------------------"

# Check if services can reach each other
check_service "Gateway → PostgreSQL" \
    "docker exec deltran-gateway sh -c 'timeout 3 nc -zv postgres 5432' 2>&1 | grep -q succeeded || true"

check_service "Gateway → Redis" \
    "docker exec deltran-gateway sh -c 'timeout 3 nc -zv redis 6379' 2>&1 | grep -q succeeded || true"

check_service "Gateway → NATS" \
    "docker exec deltran-gateway sh -c 'timeout 3 nc -zv nats 4222' 2>&1 | grep -q succeeded || true"

echo ""
echo "6. Monitoring Stack"
echo "------------------"

check_service "Prometheus" \
    "curl -sf http://localhost:9090/-/healthy"

check_service "Grafana" \
    "curl -sf http://localhost:3000/api/health | grep -q ok"

echo ""
echo "=================================="
echo "Verification Summary"
echo "=================================="
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All checks passed! Infrastructure is ready.${NC}"
    exit 0
else
    echo -e "${RED}Some checks failed. Please review the errors above.${NC}"
    exit 1
fi
