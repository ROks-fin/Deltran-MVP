# Multi-stage build for Account Monitor service

# Stage 1: Build
FROM rust:1.75 as builder

WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install required dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/account-monitor /app/account-monitor

# Copy migrations
COPY migrations ./migrations

# Expose port
EXPOSE 8090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8090/health || exit 1

# Run the service
CMD ["/app/account-monitor"]
