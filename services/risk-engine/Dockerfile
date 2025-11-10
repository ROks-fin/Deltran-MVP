# Build stage
FROM rust:1.75 AS builder

WORKDIR /app

COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/risk-engine /app/risk-engine

RUN useradd -m -u 1001 -s /bin/bash deltran && \
    chown -R deltran:deltran /app

USER deltran

EXPOSE 8084

CMD ["./risk-engine"]