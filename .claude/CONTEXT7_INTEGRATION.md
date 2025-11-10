# Context7 Integration Guide

This document explains how Context7 is integrated into the DelTran MVP development workflow to ensure all implementations use up-to-date, accurate documentation.

## What is Context7?

Context7 is an MCP (Model Context Protocol) server that provides access to up-to-date documentation for thousands of libraries and frameworks. It ensures you're always working with current API signatures, best practices, and patterns.

## Why Use Context7 in DelTran?

DelTran uses a modern, complex tech stack:
- **Rust** (tokio, tonic, sqlx, serde, rust_decimal)
- **Go** (gin, grpc-go)
- **Infrastructure** (PostgreSQL, Redis, NATS, Envoy)

These libraries evolve rapidly. Context7 ensures we:
- ✅ Use current API signatures (avoid deprecated methods)
- ✅ Follow best practices from official docs
- ✅ Implement correct patterns (async, gRPC, database)
- ✅ Avoid common pitfalls documented by maintainers

## How to Use Context7

### Method 1: Slash Commands (Recommended)

Use the custom slash commands that automatically leverage Context7:

```bash
/docs         # Get documentation for any library
/solve        # Solve problems using Context7-backed solutions
```

**Example:**
```
You: /docs tokio async runtime
→ Automatically fetches tokio documentation focused on async runtime
→ Returns relevant examples and best practices
```

### Method 2: Direct MCP Tool Calls

For more control, call the MCP tools directly:

**Step 1: Resolve Library ID**
```
mcp__context7__resolve-library-id
libraryName: "tokio"
→ Returns: /tokio-rs/tokio
```

**Step 2: Get Documentation**
```
mcp__context7__get-library-docs
context7CompatibleLibraryID: "/tokio-rs/tokio"
topic: "async runtime"
tokens: 5000
```

### Method 3: Automatic Agent Integration

All specialized agents (Agent-Infra, Agent-Clearing, etc.) are configured to automatically use Context7 when needed through the context7-helper agent configuration.

## Common Use Cases

### Case 1: Fixing sqlx Compile-Time Verification

```
Problem: sqlx compile-time verification failing
↓
Action: /solve sqlx compile-time verification
↓
Context7: Fetches sqlx docs on offline mode
↓
Solution: Configure .sqlx/ metadata or DATABASE_URL
```

### Case 2: Implementing gRPC Server

```
Task: Create new gRPC service in Rust
↓
Action: /docs tonic server implementation
↓
Context7: Returns tonic server patterns
↓
Implementation: Follow documented patterns
```

### Case 3: Database Migrations

```
Task: Create PostgreSQL migration for new feature
↓
Action: /docs sqlx migrations
↓
Context7: Provides migration best practices
↓
Implementation: Create migration following patterns
```

## DelTran Technology Reference

### Rust Libraries

| Library | Purpose | Context7 ID | Common Topics |
|---------|---------|-------------|---------------|
| tokio | Async runtime | /tokio-rs/tokio | async runtime, channels, tasks |
| tonic | gRPC framework | /hyperium/tonic | server, client, streaming |
| sqlx | PostgreSQL driver | /launchbadge/sqlx | queries, migrations, compile-time |
| serde | Serialization | /serde-rs/serde | derive, attributes, custom |
| rust_decimal | Decimal math | /paupino/rust-decimal | precision, operations |

### Go Libraries

| Library | Purpose | Context7 ID | Common Topics |
|---------|---------|-------------|---------------|
| gin | HTTP framework | /gin-gonic/gin | routing, middleware, handlers |
| grpc-go | gRPC client | /grpc/grpc-go | client, interceptors |

### Infrastructure

| Tech | Purpose | Context7 ID | Common Topics |
|------|---------|-------------|---------------|
| PostgreSQL | Database | /postgres/postgres | queries, indexes, transactions |
| Redis | Cache | /redis/redis | commands, patterns, pub/sub |
| NATS | Message broker | /nats-io/nats.go | jetstream, subscribe, publish |

## Best Practices

### ✅ DO:
- Fetch documentation BEFORE writing code
- Focus on specific topics (e.g., "async runtime" not just "tokio")
- Reference the documentation in your code comments
- Use Context7 when encountering unfamiliar errors

### ❌ DON'T:
- Guess API signatures - verify with Context7
- Use deprecated patterns - check current docs
- Skip documentation for "simple" tasks - APIs change
- Ignore warning messages about outdated patterns

## Integration with Agent Workflow

The agent workflow now includes Context7 at each step:

```
1. Agent-Infra (Infrastructure Setup)
   ├─ Context7: NATS JetStream configuration
   ├─ Context7: PostgreSQL best practices
   └─ Context7: Envoy proxy patterns

2. Agent-Clearing (Clearing Engine)
   ├─ Context7: tokio async patterns
   ├─ Context7: sqlx compile-time verification
   └─ Context7: rust_decimal precision

3. Agent-Settlement (Settlement Engine)
   ├─ Context7: sqlx transaction patterns
   ├─ Context7: tonic gRPC server
   └─ Context7: tokio error handling

4. Agent-Gateway (Gateway Integration)
   ├─ Context7: gin middleware
   ├─ Context7: grpc-go client
   └─ Context7: JWT patterns

[... and so on for all agents]
```

## Quick Start

1. **Start working on a feature:**
   ```
   I need to implement XYZ feature
   ```

2. **System automatically identifies tech stack:**
   - Rust service? → tokio, tonic, sqlx
   - Go service? → gin, grpc-go
   - Infrastructure? → NATS, PostgreSQL, etc.

3. **Context7 fetches relevant docs:**
   ```
   📚 Fetching documentation for: tokio
   Topic: async runtime
   ```

4. **Implementation uses documented patterns:**
   ```rust
   // Following tokio official pattern for async tasks
   tokio::spawn(async move {
       // Implementation based on Context7 docs
   });
   ```

## Testing Context7 Integration

Try these commands to verify Context7 is working:

```bash
# Test 1: Get tokio documentation
/docs tokio async runtime

# Test 2: Solve a problem with Context7
/solve sqlx compile-time verification

# Test 3: Direct MCP call
# Use: mcp__context7__resolve-library-id with libraryName: "tonic"
```

## Troubleshooting

**Issue: Context7 not responding**
- Check MCP server is running
- Verify network connectivity
- Check Context7 server status

**Issue: Wrong documentation returned**
- Be more specific with topic parameter
- Use exact library names
- Check library ID is correct

**Issue: Documentation outdated**
- Context7 uses official sources
- Report issue if truly outdated
- May need to specify version

## Support

For Context7 integration issues:
1. Check [Context7 documentation](https://context7.com)
2. Review MCP server logs
3. Test with simple library first (e.g., "tokio")
4. Contact Context7 support if persistent issues

---

**Remember:** Context7 is now your primary documentation source. Always consult it before implementing, and trust the patterns it provides over generic solutions.
