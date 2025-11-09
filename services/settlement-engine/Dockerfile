FROM rust:latest as builder

WORKDIR /app

# Copy all files
COPY . .

# Build application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/settlement-engine /app/settlement-engine

# Expose ports
EXPOSE 8087

# Run application
CMD ["/app/settlement-engine"]
