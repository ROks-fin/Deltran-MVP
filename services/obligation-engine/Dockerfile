# Build stage
FROM rust:1.75 AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build for release
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/obligation-engine /app/obligation-engine

# Create a non-root user
RUN useradd -m -u 1001 -s /bin/bash deltran && \
    chown -R deltran:deltran /app

USER deltran

EXPOSE 8082

CMD ["./obligation-engine"]