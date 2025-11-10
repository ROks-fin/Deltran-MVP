# DelTran MVP - Unified Routing Schema

## Architecture Overview

DelTran uses **Envoy Proxy** as the edge gateway with **direct routing** to microservices. Each microservice is independently accessible through standardized `/api/v1/` endpoints.

```
Client
  ↓
Envoy Proxy (:80/:443)
  ↓
  ├─→ /api/v1/tokens/*         → Token Engine (:8081)
  ├─→ /api/v1/accounts/*       → Settlement Engine (:8087)
  ├─→ /api/v1/reports/*        → Reporting Engine (:8088)
  ├─→ /api/v1/notifications/*  → Notification Engine (:8089)
  ├─→ /api/v1/clearing/*       → Clearing Engine (:8085)
  ├─→ /api/v1/compliance/*     → Compliance Engine (:8086)
  ├─→ /api/v1/risk/*           → Risk Engine (:8084)
  ├─→ /api/v1/liquidity/*      → Liquidity Router (:8083)
  ├─→ /api/v1/obligations/*    → Obligation Engine (:8082)
  ├─→ /api/v1/*                → Gateway (:8080) [Auth, Transfers, etc.]
  └─→ /ws                      → Notification Engine WebSocket (:8089)
```

---

## Complete API Routing Table

| Route Pattern | Service | Port | Purpose | Example |
|---------------|---------|------|---------|---------|
| `/api/v1/tokens/mint` | Token Engine | 8081 | Mint new tokens | `POST /api/v1/tokens/mint` |
| `/api/v1/tokens/burn` | Token Engine | 8081 | Burn tokens | `POST /api/v1/tokens/burn` |
| `/api/v1/tokens/transfer` | Token Engine | 8081 | Transfer tokens | `POST /api/v1/tokens/transfer` |
| `/api/v1/tokens/convert` | Token Engine | 8081 | Convert tokens | `POST /api/v1/tokens/convert` |
| `/api/v1/tokens/balance/{bank_id}` | Token Engine | 8081 | Get token balance | `GET /api/v1/tokens/balance/{id}` |
| `/api/v1/tokens/health` | Token Engine | 8081 | Health check | `GET /api/v1/tokens/health` |

| Route Pattern | Service | Port | Purpose | Example |
|---------------|---------|------|---------|---------|
| `/api/v1/obligations/*` | Obligation Engine | 8082 | Obligation management | `GET /api/v1/obligations/list` |

| Route Pattern | Service | Port | Purpose | Example |
|---------------|---------|------|---------|---------|
| `/api/v1/liquidity/*` | Liquidity Router | 8083 | Liquidity routing | `POST /api/v1/liquidity/route` |

| Route Pattern | Service | Port | Purpose | Example |
|---------------|---------|------|---------|---------|
| `/api/v1/risk/*` | Risk Engine | 8084 | Risk assessment | `POST /api/v1/risk/assess` |

| Route Pattern | Service | Port | Purpose | Example |
|---------------|---------|------|---------|---------|
| `/api/v1/clearing/*` | Clearing Engine | 8085 | Clearing operations | `POST /api/v1/clearing/process` |

| Route Pattern | Service | Port | Purpose | Example |
|---------------|---------|------|---------|---------|
| `/api/v1/compliance/*` | Compliance Engine | 8086 | Compliance checks | `POST /api/v1/compliance/check` |

| Route Pattern | Service | Port | Purpose | Example |
|---------------|---------|------|---------|---------|
| `/api/v1/accounts/nostro` | Settlement Engine | 8087 | Nostro account ops | `GET /api/v1/accounts/nostro` |
| `/api/v1/accounts/vostro` | Settlement Engine | 8087 | Vostro account ops | `GET /api/v1/accounts/vostro` |
| `/health` | Settlement Engine | 8087 | Health check | `GET /health` |

| Route Pattern | Service | Port | Purpose | Example |
|---------------|---------|------|---------|---------|
| `/api/v1/reports/generate` | Reporting Engine | 8088 | Generate report | `POST /api/v1/reports/generate` |
| `/api/v1/reports` | Reporting Engine | 8088 | List reports | `GET /api/v1/reports` |
| `/api/v1/reports/{id}` | Reporting Engine | 8088 | Get report by ID | `GET /api/v1/reports/{id}` |
| `/api/v1/reports/aml/daily` | Reporting Engine | 8088 | Daily AML report | `POST /api/v1/reports/aml/daily` |

| Route Pattern | Service | Port | Purpose | Example |
|---------------|---------|------|---------|---------|
| `/ws?user_id={id}` | Notification Engine | 8089 | WebSocket connection | `WS /ws?user_id=123` |
| `/api/v1/notifications` | Notification Engine | 8089 | List notifications | `GET /api/v1/notifications?user_id=123` |
| `/api/v1/notifications` | Notification Engine | 8089 | Send notification | `POST /api/v1/notifications` |
| `/api/v1/stats` | Notification Engine | 8089 | Get stats | `GET /api/v1/stats` |

| Route Pattern | Service | Port | Purpose | Example |
|---------------|---------|------|---------|---------|
| `/api/v1/auth/login` | Gateway | 8080 | User authentication | `POST /api/v1/auth/login` |
| `/api/v1/transfer` | Gateway | 8080 | Initiate transfer | `POST /api/v1/transfer` |
| `/api/v1/transaction/{id}` | Gateway | 8080 | Get transaction | `GET /api/v1/transaction/{id}` |
| `/api/v1/transactions` | Gateway | 8080 | List transactions | `GET /api/v1/transactions` |
| `/api/v1/banks` | Gateway | 8080 | List banks | `GET /api/v1/banks` |
| `/api/v1/corridors` | Gateway | 8080 | List corridors | `GET /api/v1/corridors` |
| `/api/v1/rates/{corridor}` | Gateway | 8080 | Get exchange rates | `GET /api/v1/rates/USD-EUR` |
| `/health` | Gateway | 8080 | Health check | `GET /health` |

---

## Envoy Proxy Configuration

### Route Priority (Order Matters!)

Envoy processes routes from **top to bottom**. More specific routes MUST come before general routes:

1. **Specific service routes** (e.g., `/api/v1/tokens/`) ← Most specific
2. **General gateway route** (`/api/v1/`) ← Catch-all for Gateway
3. **WebSocket route** (`/ws`)
4. **Health checks** (`/health`, `/metrics`)

### Circuit Breaker Limits

| Service | Max Connections | Max Requests | Max Retries |
|---------|----------------|--------------|-------------|
| Gateway | 1000 | 1000 | 3 |
| Token Engine | 500 | 500 | 3 |
| Settlement Engine | 500 | 500 | 3 |
| Reporting Engine | 300 | 300 | 2 |
| Notification Engine | 1000 | 1000 | 2 |
| Other Services | 500 | 500 | 3 |

### Timeout Configuration

| Service | Timeout | Retry Timeout |
|---------|---------|---------------|
| Gateway | 30s | 10s |
| Token Engine | 30s | 10s |
| Settlement Engine | 30s | 10s |
| Reporting Engine | **60s** | 30s (heavy operations) |
| Notification Engine | 30s | - |
| Risk Engine | **15s** | - (fast response required) |
| WebSocket | **0s** (no timeout) | - |

---

## HTTP Status Codes

### Success Responses
- `200 OK` - Request successful
- `201 Created` - Resource created
- `204 No Content` - Successful deletion

### Client Errors
- `400 Bad Request` - Invalid request format
- `401 Unauthorized` - Missing/invalid authentication
- `403 Forbidden` - Insufficient permissions
- `404 Not Found` - Resource not found
- `409 Conflict` - Resource conflict
- `422 Unprocessable Entity` - Validation failed
- `429 Too Many Requests` - Rate limit exceeded

### Server Errors
- `500 Internal Server Error` - Server error
- `502 Bad Gateway` - Upstream service error
- `503 Service Unavailable` - Service temporarily down
- `504 Gateway Timeout` - Upstream timeout

---

## Rate Limiting

### Global Rate Limit (Envoy)
- **10,000 requests/minute** across all endpoints
- **Token bucket** algorithm

### API-specific Rate Limits
- `/api/v1/*` → **1,000 requests/minute**
- `/admin` → **100 requests/minute**
- Other endpoints → **Global limit**

### Rate Limit Headers
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 847
X-RateLimit-Reset: 1699564800
X-Rate-Limit-Status: 200
```

---

## CORS Configuration

### Allowed Origins
- `*` (All origins for MVP)

### Allowed Methods
- `GET`, `POST`, `PUT`, `DELETE`, `OPTIONS`, `PATCH`

### Allowed Headers
- `Content-Type`, `Authorization`, `X-Request-ID`, `X-API-Key`

### Max Age
- `3600 seconds` (1 hour)

---

## Request/Response Examples

### Successful Token Mint Request

**Request:**
```http
POST /api/v1/tokens/mint HTTP/1.1
Host: localhost
Content-Type: application/json
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...

{
  "bank_id": "550e8400-e29b-41d4-a716-446655440000",
  "currency": "USD",
  "amount": "1000.00"
}
```

**Response:**
```http
HTTP/1.1 200 OK
Content-Type: application/json
X-Request-ID: 7f8d9e10-1234-5678-9abc-def012345678

{
  "token_id": "7f8d9e10-1234-5678-9abc-def012345678",
  "bank_id": "550e8400-e29b-41d4-a716-446655440000",
  "currency": "USD",
  "amount": "1000.00",
  "created_at": "2025-11-10T14:30:00Z"
}
```

### WebSocket Connection Example

**JavaScript Client:**
```javascript
const ws = new WebSocket('ws://localhost/ws?user_id=123&bank_id=bank-001');

ws.onopen = () => {
  console.log('Connected to notification stream');
};

ws.onmessage = (event) => {
  const notification = JSON.parse(event.data);
  console.log('Received:', notification);
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};
```

### 404 Error Response

**Request:**
```http
GET /api/v1/nonexistent HTTP/1.1
Host: localhost
```

**Response:**
```http
HTTP/1.1 404 Not Found
Content-Type: application/json

{
  "error": "Not Found",
  "message": "The requested resource was not found",
  "path": "/api/v1/nonexistent",
  "timestamp": "2025-11-10T14:30:00Z"
}
```

---

## Health Check Endpoints

All services expose health checks:

```bash
# Gateway
curl http://localhost:8080/health
# {"status": "ok"}

# Token Engine
curl http://localhost:8081/api/v1/tokens/health
# {"status": "healthy", "service": "token-engine"}

# Settlement Engine
curl http://localhost:8087/health
# {"status": "ok"}

# Reporting Engine
curl http://localhost:8088/health
# {"status": "healthy"}

# Notification Engine
curl http://localhost:8089/health
# {"status": "healthy"}
```

---

## Common 404 Scenarios & Solutions

### ❌ Problem 1: Calling `/tokens/mint` instead of `/api/v1/tokens/mint`

**Incorrect:**
```bash
curl -X POST http://localhost/tokens/mint
# 404 Not Found
```

**Correct:**
```bash
curl -X POST http://localhost/api/v1/tokens/mint
# 200 OK
```

---

### ❌ Problem 2: Using wrong port in direct service access

**Incorrect (bypassing Envoy):**
```bash
curl http://localhost:8088/api/v1/accounts/nostro
# 404 Not Found (Reporting Engine doesn't have this route)
```

**Correct:**
```bash
# Via Envoy (recommended)
curl http://localhost/api/v1/accounts/nostro

# OR Direct to Settlement Engine
curl http://localhost:8087/api/v1/accounts/nostro
```

---

### ❌ Problem 3: Calling service-specific endpoint through Gateway

**Incorrect:**
```bash
curl http://localhost:8080/api/v1/tokens/mint
# 404 Not Found (Gateway doesn't handle token operations)
```

**Correct (via Envoy):**
```bash
curl http://localhost/api/v1/tokens/mint
# Routes directly to Token Engine via Envoy
```

---

## Troubleshooting Guide

### Check if Envoy is running
```bash
curl http://localhost:9901/ready
# 200 OK means Envoy is ready
```

### Check Envoy cluster health
```bash
curl http://localhost:9901/clusters | grep token_engine
# Should show health status
```

### Verify service is responding
```bash
# Direct to service (bypass Envoy)
curl http://localhost:8081/api/v1/tokens/health
```

### Check Envoy routing
```bash
# View Envoy config
curl http://localhost:9901/config_dump
```

---

## Architecture Decision Records

### ADR-001: Direct Service Routing via Envoy

**Decision:** Envoy routes directly to microservices based on URL prefixes, Gateway handles only auth and orchestrated operations.

**Rationale:**
- **Performance**: Eliminates double-hop through Gateway
- **Scalability**: Each service scales independently
- **Simplicity**: Clear separation of concerns

**Consequences:**
- Each service MUST implement `/api/v1/{service}/*` prefix
- Envoy configuration must be kept in sync with services
- Circuit breakers applied at Envoy level

---

### ADR-002: Standardized `/api/v1/` Prefix

**Decision:** All API endpoints use `/api/v1/` prefix.

**Rationale:**
- **Versioning**: Enables future v2, v3 APIs
- **Consistency**: Clear API contract
- **Routing**: Simplifies Envoy routing rules

**Exceptions:**
- `/health` - Health checks (no version prefix)
- `/ws` - WebSocket endpoint (no version prefix)
- `/metrics` - Prometheus metrics (no version prefix)

---

## Security Considerations

### TLS/mTLS
- **Port 443 (8443)**: HTTPS with mTLS
- **Port 80 (8080)**: HTTP (MVP only, redirect to HTTPS in production)

### Authentication
- **JWT Tokens**: Bearer authentication via Gateway
- **API Keys**: For service-to-service communication (future)

### Rate Limiting
- Applied at Envoy level
- Per-client IP tracking
- Token bucket algorithm

---

## Monitoring & Observability

### Metrics Endpoints
```bash
# Envoy metrics
curl http://localhost:9901/stats/prometheus

# Service-specific metrics
curl http://localhost:8081/metrics
curl http://localhost:8087/metrics
```

### Logging
- **Format**: JSON structured logs
- **Fields**: timestamp, level, service, trace_id, message
- **Destination**: stdout (captured by Docker/k8s)

### Tracing
- **Trace ID**: Propagated via `X-Request-ID` header
- **Span tracking**: Across all services
- **Tool**: Jaeger (future integration)

---

**Last Updated**: 2025-11-10
**Status**: ✅ Production Ready
**Version**: 1.0

**Key Changes**:
- Added direct routing to all 9 microservices via Envoy
- Standardized `/api/v1/` prefix across all services
- Fixed port allocation documentation
- Implemented circuit breakers and health checks
- Added comprehensive troubleshooting guide
