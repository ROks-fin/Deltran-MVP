---
description: Solve problems using Context7 for accurate, up-to-date solutions
---

You are solving a problem in the DelTran MVP system. Follow this systematic approach:

## Problem-Solving Protocol

### 1. Understand the Problem
- Read the error message or requirement carefully
- Identify which service/component is involved
- Determine the technology stack in use

### 2. Fetch Documentation (REQUIRED)
Before writing ANY code, use Context7 to get current documentation:

**For Rust services:**
- `tokio` - async runtime issues
- `tonic` - gRPC problems
- `sqlx` - database operations
- `serde` - serialization issues
- `rust_decimal` - financial calculation precision

**For Go services:**
- `gin` - HTTP/REST API issues
- `grpc-go` - gRPC client issues

**For Infrastructure:**
- `postgres` - PostgreSQL queries
- `redis` - caching strategies
- `nats` - message broker patterns

### 3. Apply Documentation
- Use the EXACT patterns from the fetched docs
- Follow current API signatures (not deprecated ones)
- Apply best practices from the documentation

### 4. Implement Solution
- Write code that matches the documented patterns
- Include proper error handling as per docs
- Add comments referencing the documentation approach

### 5. Explain with Insights
After solving, provide educational insights:
- Why this solution follows best practices
- What the documentation recommends
- Key patterns being applied

## Example Workflow

```
User reports: "sqlx compile-time verification failing"

1. Use Context7: resolve-library-id("sqlx")
2. Get docs: focus on "compile-time verification"
3. Review: offline mode, query metadata
4. Apply: Fix with proper DATABASE_URL or offline mode
5. Explain: Why compile-time checking is valuable
```

## Critical Rules

- ✅ ALWAYS fetch Context7 docs before coding
- ✅ Prefer documented patterns over custom solutions
- ✅ Reference specific documentation in your explanation
- ❌ DON'T guess API signatures
- ❌ DON'T use deprecated patterns
- ❌ DON'T skip documentation lookup

Now identify the relevant technology and fetch the documentation before proceeding.
