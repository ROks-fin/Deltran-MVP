#!/bin/bash

# Comprehensive script to fix a single Rust service
# Usage: ./fix_service_complete.sh <service-name>

SERVICE=$1

if [ -z "$SERVICE" ]; then
    echo "Usage: $0 <service-name>"
    exit 1
fi

SERVICE_DIR="services/$SERVICE"

if [ ! -d "$SERVICE_DIR" ]; then
    echo "Service directory $SERVICE_DIR not found"
    exit 1
fi

echo "=== Fixing $SERVICE ==="

cd "$SERVICE_DIR"

# 1. Fix Cargo.toml - add rust_decimal feature to sqlx
echo "1. Updating Cargo.toml..."
if [ -f "Cargo.toml" ]; then
    # Add rust_decimal to sqlx features if not present
    sed -i 's/sqlx = { version = "0.7", features = \[.*"chrono"\]/&, "rust_decimal"/' Cargo.toml
    # Remove "decimal" feature if present (doesn't exist in sqlx 0.7)
    sed -i 's/, "decimal"//g' Cargo.toml
    # Replace rdkafka with async-nats
    sed -i 's/rdkafka = .*/async-nats = "0.33"/g' Cargo.toml
fi

# 2. Fix config.rs - replace kafka with nats
echo "2. Updating config.rs..."
if [ -f "src/config.rs" ]; then
    # Update NatsConfig struct to use 'url' instead of 'brokers'
    sed -i 's/pub brokers: String,/pub url: String,/g' src/config.rs
    # Update config defaults from kafka to nats
    sed -i 's/"kafka\./"nats./g' src/config.rs
    # Update variable names
    sed -i 's/kafka_brokers/nats_url/g' src/config.rs
    sed -i 's/kafka\.brokers/nats.url/g' src/config.rs
    sed -i 's/self\.kafka\./self.nats./g' src/config.rs
    # Update error messages
    sed -i 's/Kafka brokers/NATS URL/g' src/config.rs
    # Update environment variable
    sed -i 's/NATS_BROKERS/NATS_URL/g' src/config.rs
fi

# 3. Fix main.rs - update NATS initialization
echo "3. Updating main.rs..."
if [ -f "src/main.rs" ]; then
    # Update variable name from kafka to nats
    sed -i 's/let kafka = /let nats = /g' src/main.rs
    # Update NatsProducer::new call to be async and use .url instead of .brokers
    sed -i 's/NatsProducer::new(&config\.nats\.brokers/NatsProducer::new(\&config.nats.url/g' src/main.rs
    # Add .await to NatsProducer::new if missing
    sed -i 's/NatsProducer::new([^)]*))$/&.await/g' src/main.rs
    # Update comment
    sed -i 's/Initialize Kafka/Initialize NATS/g' src/main.rs
    # Update success message
    sed -i 's/Kafka producer/NATS producer/g' src/main.rs
    # Update service initialization parameter
    sed -i 's/TokenService::new(db, kafka,/TokenService::new(db, nats,/g' src/main.rs
    sed -i 's/ObligationService::new(db, kafka,/ObligationService::new(db, nats,/g' src/main.rs
fi

# 4. Fix models.rs - remove #[validate(range)] from Decimal fields
echo "4. Updating models.rs..."
if [ -f "src/models.rs" ]; then
    # Remove #[validate(range(...))] from lines before Decimal fields
    sed -i '/pub amount: Decimal/i\    #[validate(range(min = 0.01))]' src/models.rs 2>/dev/null || true
    sed -i '/#\[validate(range.*)\]/d' src/models.rs
fi

# 5. Fix errors.rs - add DecimalParse error variant
echo "5. Updating errors.rs..."
if [ -f "src/errors.rs" ]; then
    # Check if DecimalParse error already exists
    if ! grep -q "DecimalParse" src/errors.rs; then
        # Add after Nats error
        sed -i '/Nats(String),/a\    \n    #[error("Decimal parse error: {0}")]\n    DecimalParse(#[from] rust_decimal::Error),' src/errors.rs

        # Add to status_code match
        sed -i '/TokenEngineError::Nats(_) => StatusCode::INTERNAL_SERVER_ERROR,/a\            TokenEngineError::DecimalParse(_) => StatusCode::BAD_REQUEST,' src/errors.rs
        sed -i '/ObligationEngineError::Nats(_) => StatusCode::INTERNAL_SERVER_ERROR,/a\            ObligationEngineError::DecimalParse(_) => StatusCode::BAD_REQUEST,' src/errors.rs

        # Add to error_type match
        sed -i '/TokenEngineError::Nats(_) => "messaging_error",/a\            TokenEngineError::DecimalParse(_) => "decimal_parse_error",' src/errors.rs
        sed -i '/ObligationEngineError::Nats(_) => "messaging_error",/a\            ObligationEngineError::DecimalParse(_) => "decimal_parse_error",' src/errors.rs
    fi
fi

echo "=== Fix complete for $SERVICE ==="
echo "Run: cd $SERVICE_DIR && cargo build --release"

cd ../..
