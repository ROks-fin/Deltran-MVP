---
description: Get up-to-date documentation for any library used in DelTran using Context7
---

You are helping with the DelTran MVP system. When solving problems or implementing features, ALWAYS use Context7 to fetch the latest documentation for the relevant technologies.

## DelTran Tech Stack

**Core Services (Rust):**
- tokio (async runtime)
- tonic (gRPC framework)
- sqlx (PostgreSQL driver with compile-time verification)
- serde (serialization)
- rust_decimal (precise decimal arithmetic for financial calculations)

**API Layer (Go):**
- gin (HTTP framework)
- grpc-go (gRPC client)
- golang.org/x/sync

**Infrastructure:**
- PostgreSQL 16 with TimescaleDB
- Redis Cluster
- NATS JetStream
- Envoy Proxy
- Docker & Kubernetes

**Frontend (if applicable):**
- React
- Next.js
- TypeScript

## Workflow

1. **Identify the technology** involved in the user's request
2. **Use Context7** to fetch the latest documentation:
   - Call `mcp__context7__resolve-library-id` with the library name
   - Then call `mcp__context7__get-library-docs` with the returned library ID
3. **Apply the documentation** to solve the problem correctly
4. **Reference specific documentation** when explaining your solution

## Example Usage

When fixing a tokio async issue:
1. Resolve: `mcp__context7__resolve-library-id` with "tokio"
2. Fetch docs: `mcp__context7__get-library-docs` with the library ID, topic "async runtime"
3. Apply the official patterns from documentation

When implementing a gRPC service:
1. Resolve: `mcp__context7__resolve-library-id` with "tonic"
2. Fetch docs: `mcp__context7__get-library-docs` focusing on "server implementation"
3. Follow the current best practices

## Priority Libraries

For any task involving:
- **Database operations** → Use Context7 for `sqlx`
- **Async code** → Use Context7 for `tokio`
- **gRPC services** → Use Context7 for `tonic` (Rust) or `grpc-go` (Go)
- **API endpoints** → Use Context7 for `gin` (Go)
- **Message queuing** → Use Context7 for `nats.rs` or `nats.go`
- **Financial calculations** → Use Context7 for `rust_decimal`

Now proceed to help with the user's request, using Context7 to ensure you're following the latest best practices and API patterns.
