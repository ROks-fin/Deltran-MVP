# Multi-stage build for DelTran Gateway Service

# Stage 1: Builder
FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build for release
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/deltran-gateway /usr/local/bin/gateway

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

# Create non-root user
RUN useradd -m -u 1000 deltran && \
    chown -R deltran:deltran /app

USER deltran

EXPOSE 8080

ENV RUST_LOG=info
ENV DATABASE_URL=postgres://deltran:deltran@localhost:5432/deltran_gateway
ENV NATS_URL=nats://localhost:4222
ENV BIND_ADDR=0.0.0.0:8080

CMD ["gateway"]
