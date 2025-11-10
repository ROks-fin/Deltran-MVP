#!/bin/bash

# Script to add Prometheus metrics to all Rust services
# Based on Agent-Analytics instructions

echo "========================================="
echo "Adding Prometheus metrics to Rust services"
echo "========================================="

# List of Rust services (excluding token-engine which already has metrics)
SERVICES=(
    "clearing-engine"
    "settlement-engine"
    "obligation-engine"
    "risk-engine"
    "compliance-engine"
    "liquidity-router"
)

# Source metrics file
SOURCE_METRICS="services/token-engine/src/metrics.rs"

# Check if source metrics exists
if [ ! -f "$SOURCE_METRICS" ]; then
    echo "❌ Error: Source metrics file not found: $SOURCE_METRICS"
    exit 1
fi

echo "✅ Found source metrics at: $SOURCE_METRICS"
echo ""

# Copy metrics to each service
for service in "${SERVICES[@]}"; do
    echo "📦 Processing: $service"

    SERVICE_DIR="services/$service"
    TARGET_METRICS="$SERVICE_DIR/src/metrics.rs"
    CARGO_TOML="$SERVICE_DIR/Cargo.toml"
    LIB_RS="$SERVICE_DIR/src/lib.rs"
    MAIN_RS="$SERVICE_DIR/src/main.rs"

    # Check if service exists
    if [ ! -d "$SERVICE_DIR" ]; then
        echo "  ⚠️  Skipping: Service directory not found"
        continue
    fi

    # Copy metrics.rs
    if [ -f "$TARGET_METRICS" ]; then
        echo "  ℹ️  Metrics already exists, updating..."
    fi

    cp "$SOURCE_METRICS" "$TARGET_METRICS"
    echo "  ✅ Metrics copied"

    # Update Cargo.toml if prometheus not already added
    if ! grep -q "prometheus" "$CARGO_TOML"; then
        echo "  📝 Adding Prometheus dependencies to Cargo.toml..."
        cat >> "$CARGO_TOML" << EOF

# Metrics - Prometheus
prometheus = { version = "0.13", features = ["process"] }
lazy_static = "1.4"
EOF
        echo "  ✅ Dependencies added"
    else
        echo "  ℹ️  Prometheus dependencies already exist"
    fi

    # Update lib.rs if it exists
    if [ -f "$LIB_RS" ]; then
        if ! grep -q "pub mod metrics" "$LIB_RS"; then
            echo "  📝 Adding metrics module to lib.rs..."
            # Add before the first 'pub use' line
            sed -i '/^pub use/i pub mod metrics;' "$LIB_RS"
            echo "  ✅ lib.rs updated"
        else
            echo "  ℹ️  Metrics module already in lib.rs"
        fi
    fi

    echo "  ✅ $service updated successfully"
    echo ""
done

echo "========================================="
echo "✅ All services updated!"
echo "========================================="
echo ""
echo "Next steps:"
echo "1. Add /metrics endpoint to each service's handlers or main.rs"
echo "2. Import metrics module: use crate::metrics;"
echo "3. Add route: .route(\"/metrics\", web::get().to(metrics_endpoint))"
echo "4. Build and test:"
echo "   cd services/SERVICE_NAME && cargo build"
echo "5. Test metrics endpoint:"
echo "   curl http://localhost:PORT/metrics"
