#!/bin/bash

# Script to add JWT middleware to all Rust services
# Based on Agent-Security instructions

echo "========================================="
echo "Adding JWT middleware to Rust services"
echo "========================================="

# List of Rust services (excluding token-engine which already has middleware)
SERVICES=(
    "clearing-engine"
    "settlement-engine"
    "obligation-engine"
    "risk-engine"
    "compliance-engine"
    "liquidity-router"
)

# Source middleware directory
SOURCE_MIDDLEWARE="services/token-engine/src/middleware"

# Check if source middleware exists
if [ ! -d "$SOURCE_MIDDLEWARE" ]; then
    echo "❌ Error: Source middleware directory not found: $SOURCE_MIDDLEWARE"
    exit 1
fi

echo "✅ Found source middleware at: $SOURCE_MIDDLEWARE"
echo ""

# Copy middleware to each service
for service in "${SERVICES[@]}"; do
    echo "📦 Processing: $service"

    SERVICE_DIR="services/$service"
    TARGET_MIDDLEWARE="$SERVICE_DIR/src/middleware"
    CARGO_TOML="$SERVICE_DIR/Cargo.toml"

    # Check if service exists
    if [ ! -d "$SERVICE_DIR" ]; then
        echo "  ⚠️  Skipping: Service directory not found"
        continue
    fi

    # Copy middleware
    if [ -d "$TARGET_MIDDLEWARE" ]; then
        echo "  ℹ️  Middleware already exists, updating..."
        rm -rf "$TARGET_MIDDLEWARE"
    fi

    cp -r "$SOURCE_MIDDLEWARE" "$TARGET_MIDDLEWARE"
    echo "  ✅ Middleware copied"

    # Update Cargo.toml if needed
    if ! grep -q "jsonwebtoken" "$CARGO_TOML"; then
        echo "  📝 Adding dependencies to Cargo.toml..."
        cat >> "$CARGO_TOML" << EOF

# Security - JWT authentication
jsonwebtoken = "9.2"

# Rate limiting
governor = "0.6"

# Additional utilities
futures-util = "0.3"
EOF
        echo "  ✅ Dependencies added"
    else
        echo "  ℹ️  Dependencies already exist"
    fi

    echo "  ✅ $service updated successfully"
    echo ""
done

echo "========================================="
echo "✅ All services updated!"
echo "========================================="
echo ""
echo "Next steps:"
echo "1. Update each service's main.rs to use the middleware:"
echo "   - Add: mod middleware;"
echo "   - Add: use middleware::{auth::JwtAuth, rate_limit::RateLimiter, audit::AuditLog};"
echo "   - Wrap App with: .wrap(AuditLog).wrap(JwtAuth::new(jwt_secret)).wrap(RateLimiter::new(rate_limit))"
echo ""
echo "2. Build and test each service:"
echo "   cd services/SERVICE_NAME && cargo build"
echo ""
echo "3. Set environment variables:"
echo "   export JWT_SECRET=\"your-secret-key\""
echo "   export RATE_LIMIT_PER_MINUTE=100"
