#!/bin/bash

# DelTran Services Build Script
# Builds all services and reports status

echo "=================================="
echo "DelTran Services Build Process"
echo "=================================="
echo ""

FAILED_SERVICES=()
SUCCESSFUL_SERVICES=()

# Function to build a Rust service
build_rust_service() {
    local service_name=$1
    local service_path=$2

    echo "Building Rust service: $service_name..."
    cd "$service_path" || { echo "Failed to enter $service_path"; return 1; }

    if cargo build --release 2>&1 | tee "/tmp/build_${service_name}.log"; then
        echo "✓ $service_name built successfully"
        SUCCESSFUL_SERVICES+=("$service_name")
        cd - > /dev/null
        return 0
    else
        echo "✗ $service_name build failed"
        FAILED_SERVICES+=("$service_name")
        cd - > /dev/null
        return 1
    fi
}

# Function to build a Go service
build_go_service() {
    local service_name=$1
    local service_path=$2

    echo "Building Go service: $service_name..."
    cd "$service_path" || { echo "Failed to enter $service_path"; return 1; }

    if go build -v ./... 2>&1 | tee "/tmp/build_${service_name}.log"; then
        echo "✓ $service_name built successfully"
        SUCCESSFUL_SERVICES+=("$service_name")
        cd - > /dev/null
        return 0
    else
        echo "✗ $service_name build failed"
        FAILED_SERVICES+=("$service_name")
        cd - > /dev/null
        return 1
    fi
}

cd "$(dirname "$0")"

echo "Step 1: Building Rust Services"
echo "================================"

# Build all Rust services
build_rust_service "clearing-engine" "services/clearing-engine"
build_rust_service "compliance-engine" "services/compliance-engine"
build_rust_service "risk-engine" "services/risk-engine"
build_rust_service "liquidity-router" "services/liquidity-router"
build_rust_service "obligation-engine" "services/obligation-engine"
build_rust_service "settlement-engine" "services/settlement-engine"
build_rust_service "token-engine" "services/token-engine"
build_rust_service "account-monitor" "services/account-monitor"

echo ""
echo "Step 2: Building Go Services"
echo "================================"

# Build all Go services
build_go_service "gateway" "services/gateway"
build_go_service "notification-engine" "services/notification-engine"
build_go_service "reporting-engine" "services/reporting-engine"

echo ""
echo "=================================="
echo "Build Summary"
echo "=================================="
echo ""
echo "Successful services (${#SUCCESSFUL_SERVICES[@]}):"
for service in "${SUCCESSFUL_SERVICES[@]}"; do
    echo "  ✓ $service"
done

if [ ${#FAILED_SERVICES[@]} -gt 0 ]; then
    echo ""
    echo "Failed services (${#FAILED_SERVICES[@]}):"
    for service in "${FAILED_SERVICES[@]}"; do
        echo "  ✗ $service"
        echo "     Log: /tmp/build_${service}.log"
    done
    exit 1
else
    echo ""
    echo "All services built successfully!"
    exit 0
fi
