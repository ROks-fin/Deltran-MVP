#!/bin/bash

# Fix Go version in all Go services to use Go 1.23 (latest stable)

echo "🔧 Fixing Go version in all Go services..."

# Fix Gateway
echo "📦 Fixing Gateway..."
cd services/gateway

# Update Dockerfile to use Go 1.23
sed -i 's/golang:1.21-alpine/golang:1.23-alpine/g' Dockerfile
echo "  ✅ Dockerfile updated"

# Update go.mod to require Go 1.23
sed -i 's/go 1.24.0/go 1.23.0/g' go.mod
echo "  ✅ go.mod updated"

cd ../..

# Fix Reporting Engine
echo "📦 Fixing Reporting Engine..."
cd services/reporting-engine

# Update Dockerfile
sed -i 's/golang:1.21-alpine/golang:1.23-alpine/g' Dockerfile
echo "  ✅ Dockerfile updated"

# Update go.mod if it has version requirement
if [ -f go.mod ]; then
    sed -i 's/go 1.24.0/go 1.23.0/g' go.mod
    sed -i 's/go 1.21/go 1.23.0/g' go.mod
    echo "  ✅ go.mod updated"
fi

cd ../..

# Fix Notification Engine
echo "📦 Fixing Notification Engine..."
cd services/notification-engine

# Update Dockerfile
sed -i 's/golang:1.21-alpine/golang:1.23-alpine/g' Dockerfile
echo "  ✅ Dockerfile updated"

# Update go.mod if it has version requirement
if [ -f go.mod ]; then
    sed -i 's/go 1.24.0/go 1.23.0/g' go.mod
    sed -i 's/go 1.21/go 1.23.0/g' go.mod
    echo "  ✅ go.mod updated"
fi

cd ../..

echo ""
echo "✅ All Go services fixed!"
echo ""
echo "📋 Summary:"
echo "  - Gateway: Go 1.23"
echo "  - Reporting Engine: Go 1.23"
echo "  - Notification Engine: Go 1.23"
