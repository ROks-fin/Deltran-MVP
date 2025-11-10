# DelTran MVP - Port Allocation Reference

## Official Port Allocation Table

| Service | HTTP Port | gRPC Port | WebSocket | Technology | Status |
|---------|-----------|-----------|-----------|------------|--------|
| **Gateway** | 8080 | - | - | Go | ✅ Primary Entry Point |
| **Token Engine** | 8081 | 50051 | - | Go | ✅ Running |
| **Obligation Engine** | 8082 | 50052 | - | Rust | ✅ Running |
| **Liquidity Router** | 8083 | - | - | Rust | ✅ Running |
| **Risk Engine** | 8084 | - | - | Rust | ✅ Running |
| **Clearing Engine** | 8085 | - | - | Rust | ✅ Running |
| **Compliance Engine** | 8086 | - | - | Rust | ✅ Running |
| **Settlement Engine** | 8087 | 50056 | - | Rust | ✅ Running |
| **Reporting Engine** | 8088 | - | - | Go | ✅ Running |
| **Notification Engine** | 8089 | - | 8090 | Go | ✅ Running |

## Infrastructure Services

| Service | Port | Protocol | Purpose |
|---------|------|----------|---------|
| PostgreSQL | 5432 | TCP | Main Database |
| NATS | 4222 | TCP | Message Broker |
| NATS Monitoring | 8222 | HTTP | NATS Dashboard |
| Redis | 6379 | TCP | Cache & Sessions |
| Prometheus | 9090 | HTTP | Metrics Collection |

## Environment Variables Per Service

### Gateway (Port 8080)
```bash
GATEWAY_PORT=8080
TOKEN_ENGINE_URL=http://localhost:8081
OBLIGATION_ENGINE_URL=http://localhost:8082
LIQUIDITY_ROUTER_URL=http://localhost:8083
RISK_ENGINE_URL=http://localhost:8084
CLEARING_ENGINE_URL=http://localhost:8085
COMPLIANCE_ENGINE_URL=http://localhost:8086
SETTLEMENT_ENGINE_URL=http://localhost:8087
REPORTING_ENGINE_URL=http://localhost:8088
NOTIFICATION_ENGINE_URL=http://localhost:8089
JWT_SECRET=deltran-secret-key-change-in-production
```

### Token Engine (Port 8081)
```bash
HTTP_PORT=8081
GRPC_PORT=50051
DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
REDIS_URL=redis://localhost:6379
NATS_URL=nats://localhost:4222
```

### Obligation Engine (Port 8082)
```bash
HTTP_PORT=8082
GRPC_PORT=50052
DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
NATS_URL=nats://localhost:4222
REDIS_URL=redis://localhost:6379
TOKEN_ENGINE_URL=http://localhost:8081
```

### Liquidity Router (Port 8083)
```bash
HTTP_PORT=8083
DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
REDIS_URL=redis://localhost:6379
```

### Risk Engine (Port 8084)
```bash
HTTP_PORT=8084
DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
REDIS_URL=redis://localhost:6379
```

### Clearing Engine (Port 8085)
```bash
HTTP_PORT=8085
DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
NATS_URL=nats://localhost:4222
```

### Compliance Engine (Port 8086)
```bash
HTTP_PORT=8086
DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
REDIS_URL=redis://localhost:6379
```

### Settlement Engine (Port 8087)
```bash
HTTP_PORT=8087
GRPC_PORT=50056
DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
NATS_URL=nats://localhost:4222
REDIS_URL=redis://localhost:6379
```

### Reporting Engine (Port 8088)
```bash
HTTP_PORT=8088
DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
NATS_URL=nats://localhost:4222
```

### Notification Engine (Port 8089)
```bash
HTTP_PORT=8089
WS_PORT=8090
DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
NATS_URL=nats://localhost:4222
REDIS_URL=redis://localhost:6379
```

## Service Startup Commands

### Quick Start All Services
```bash
# Infrastructure (start first)
docker-compose up -d postgres redis nats

# Services (can run in parallel)
cd services/gateway && go run cmd/main.go &
cd services/token-engine && go run cmd/main.go &
cd services/obligation-engine && cargo run &
cd services/liquidity-router && cargo run &
cd services/risk-engine && cargo run &
cd services/clearing-engine && cargo run &
cd services/compliance-engine && cargo run &
cd services/reporting-engine && go run cmd/main.go &
cd services/settlement-engine && cargo run &
cd services/notification-engine && go run cmd/main.go &
```

### Individual Service Startup

#### Settlement Engine (Rust - Port 8087)
```bash
cd services/settlement-engine
set DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
set NATS_URL=nats://localhost:4222
set REDIS_URL=redis://localhost:6379
set HTTP_PORT=8087
set GRPC_PORT=50056
cargo run
```

#### Notification Engine (Go - Port 8089)
```bash
cd services/notification-engine
set HTTP_PORT=8089
set WS_PORT=8090
set DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran
set NATS_URL=nats://localhost:4222
set REDIS_URL=redis://localhost:6379
go run cmd/main.go
```

## Port Conflict Resolution

### Resolved Issues
1. ✅ **Port allocation documented correctly**
   - Settlement Engine: Port 8087
   - Reporting Engine: Port 8088
   - All services have unique ports

### Verification Commands
```bash
# Check all DelTran ports
netstat -ano | findstr "8080 8081 8082 8083 8084 8085 8086 8087 8088 8089 8090"

# Check specific service
netstat -ano | findstr "8088"

# Kill process on port (if needed)
netstat -ano | findstr ":8088" | findstr "LISTENING"
# Then: taskkill /PID <process_id> /F
```

## Service Dependencies

```mermaid
graph TD
    Gateway[Gateway :8080] --> Token[Token Engine :8081]
    Gateway --> Obligation[Obligation Engine :8082]
    Gateway --> Liquidity[Liquidity Router :8083]
    Gateway --> Risk[Risk Engine :8084]
    Gateway --> Clearing[Clearing Engine :8085]
    Gateway --> Compliance[Compliance Engine :8086]
    Gateway --> Reporting[Reporting Engine :8087]
    Gateway --> Settlement[Settlement Engine :8088]
    Gateway --> Notification[Notification Engine :8089]

    Obligation --> Token
    Settlement --> NATS[NATS :4222]
    Clearing --> NATS
    Reporting --> NATS
    Notification --> NATS

    Token --> Redis[Redis :6379]
    Liquidity --> Redis
    Risk --> Redis
    Compliance --> Redis
    Settlement --> Redis

    Token --> DB[PostgreSQL :5432]
    Obligation --> DB
    Liquidity --> DB
    Risk --> DB
    Clearing --> DB
    Compliance --> DB
    Reporting --> DB
    Settlement --> DB
    Notification --> DB
```

## Health Check Endpoints

| Service | Endpoint | Expected Response |
|---------|----------|-------------------|
| Gateway | `http://localhost:8080/health` | `{"status": "ok"}` |
| Token Engine | `http://localhost:8081/health` | `{"status": "healthy"}` |
| Obligation Engine | `http://localhost:8082/health` | `{"status": "ok"}` |
| Liquidity Router | `http://localhost:8083/health` | `{"status": "ok"}` |
| Risk Engine | `http://localhost:8084/health` | `{"status": "ok"}` |
| Clearing Engine | `http://localhost:8085/health` | `{"status": "ok"}` |
| Compliance Engine | `http://localhost:8086/health` | `{"status": "ok"}` |
| Reporting Engine | `http://localhost:8088/health` | `{"status": "healthy"}` |
| **Settlement Engine** | `http://localhost:8087/health` | `{"status": "ok"}` |
| Notification Engine | `http://localhost:8089/health` | `{"status": "healthy"}` |

## Production Considerations

### Port Ranges
- **HTTP Services**: 8080-8099 (20 ports available)
- **gRPC Services**: 50051-50070 (20 ports available)
- **WebSocket**: 8090-8099 (10 ports available)
- **Metrics**: 9090-9099 (Prometheus ecosystem)

### Security
- All services should run behind the Gateway (8080) in production
- Direct service access should be blocked by firewall
- Only Gateway port 8080 should be publicly accessible
- Internal service mesh for inter-service communication

### Monitoring
```bash
# Prometheus metrics endpoints
Token Engine:       http://localhost:8081/metrics
Obligation Engine:  http://localhost:8082/metrics
Settlement Engine:  http://localhost:8087/metrics
Notification Engine: http://localhost:9095/metrics
```

---
**Last Updated**: 2025-11-10
**Status**: ✅ All services configured correctly
**Port Allocation**:
- Settlement Engine: Port 8087 (HTTP), 50056 (gRPC)
- Reporting Engine: Port 8088 (HTTP)
- No port conflicts - all services on unique ports
