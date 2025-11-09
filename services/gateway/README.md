# DelTran Gateway Service

Gateway service - единая точка входа для всех клиентов DelTran MVP.

## Описание

Gateway обеспечивает:
- Единый API endpoint для всех внешних запросов
- Аутентификацию и авторизацию (JWT)
- Rate limiting per bank (100 req/min default)
- Circuit breakers для всех backend сервисов
- Transaction flow orchestration
- Integration с всеми 9 backend сервисами

## Архитектура

```
External Client
    ↓
Envoy Proxy (edge)
    ↓
Gateway Service (this)
    ↓
├── Compliance Engine (sanctions, AML)
├── Risk Engine (risk scoring)
├── Liquidity Router (instant settlement prediction)
├── Obligation Engine (create obligations)
├── Token Engine (tokenization)
├── Clearing Engine (netting)
├── Settlement Engine (final settlement)
├── Notification Engine (alerts)
└── Reporting Engine (reports)
```

## Transaction Flow

1. **Compliance Check** → Sanctions, AML, PEP screening
2. **Risk Evaluation** → Risk score, approve/reject
3. **Liquidity Check** → Predict instant settlement probability
4. **Create Obligation** → Record obligation for clearing
5. **Tokenize** (optional) → For instant settlement
6. **Notification** → Send WebSocket/email notification

## API Endpoints

### Public Endpoints
- `GET /health` - Health check
- `POST /api/v1/auth/login` - Authentication

### Protected Endpoints (requires JWT)
- `POST /api/v1/transfer` - Initiate transfer
- `GET /api/v1/transaction/{id}` - Get transaction details
- `GET /api/v1/transactions` - List transactions
- `GET /api/v1/banks` - List supported banks
- `GET /api/v1/corridors` - List supported corridors
- `GET /api/v1/rates/{corridor}` - Get FX rates

## Структура проекта

```
gateway/
├── cmd/
│   └── main.go                    # Entry point
├── internal/
│   ├── clients/                   # HTTP/gRPC clients
│   │   ├── client_base.go         # Base HTTP client with circuit breaker
│   │   ├── compliance.go          # Compliance Engine client
│   │   ├── risk.go                # Risk Engine client
│   │   ├── liquidity.go           # Liquidity Router client
│   │   ├── obligation.go          # Obligation Engine client
│   │   ├── token.go               # Token Engine client
│   │   ├── clearing.go            # Clearing Engine client
│   │   ├── settlement.go          # Settlement Engine client
│   │   ├── notification.go        # Notification Engine client
│   │   └── reporting.go           # Reporting Engine client
│   ├── config/
│   │   └── config.go              # Configuration loading
│   ├── handlers/
│   │   └── handlers.go            # HTTP handlers
│   ├── middleware/
│   │   ├── auth.go                # JWT authentication
│   │   ├── ratelimit.go           # Rate limiting
│   │   ├── circuit_breaker.go     # Circuit breaker
│   │   ├── cors.go                # CORS
│   │   └── logging.go             # Request logging
│   ├── models/
│   │   └── models.go              # Data models
│   └── orchestration/
│       └── transaction_flow.go    # Transaction orchestration
├── tests/
│   ├── handlers_test.go           # Handler tests
│   └── middleware_test.go         # Middleware tests
├── proto/                         # gRPC proto files
├── Dockerfile
├── go.mod
└── README.md
```

## Конфигурация

Все параметры настраиваются через environment variables. См. `.env.example`.

### Ключевые настройки:

**Rate Limiting:**
- `RATE_LIMIT_RPM=100` - Requests per minute per bank
- `RATE_LIMIT_BURST=20` - Burst size

**Circuit Breaker:**
- `CB_TIMEOUT=5s` - Request timeout
- `CB_MAX_CONCURRENT=100` - Max concurrent requests
- `CB_ERROR_THRESHOLD=50` - Error percentage threshold
- `CB_SLEEP_WINDOW=10s` - Sleep window when circuit opens

**Authentication:**
- `JWT_SECRET` - Change in production!
- `JWT_EXPIRATION=24h` - Token expiration

## Запуск

### Локально (development)

```bash
# Установить зависимости
go mod download

# Скопировать конфигурацию
cp .env.example .env

# Запустить
go run cmd/main.go
```

### Docker

```bash
# Build
docker build -t deltran-gateway .

# Run
docker run -p 8080:8080 \
  -e JWT_SECRET=your-secret \
  -e TOKEN_ENGINE_URL=http://token-engine:8081 \
  deltran-gateway
```

### Docker Compose

```bash
# Запустить все сервисы
docker-compose up -d gateway
```

## Тестирование

### Unit тесты

```bash
# Все тесты
go test ./tests/...

# С coverage
go test -cover ./tests/...

# Verbose
go test -v ./tests/...
```

### Integration тесты

Требуют запущенные backend сервисы.

```bash
# Запустить backend
docker-compose up -d

# Тесты
go test -tags=integration ./tests/...
```

### Пример запроса

#### 1. Login

```bash
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "bank_id": "ICICI",
    "password": "demo"
  }'
```

Response:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "bank_id": "ICICI",
  "expires_in": 86400
}
```

#### 2. Transfer

```bash
curl -X POST http://localhost:8080/api/v1/transfer \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <token>" \
  -d '{
    "sender_bank": "ICICI",
    "receiver_bank": "ENBD",
    "amount": 100000,
    "from_currency": "INR",
    "to_currency": "AED",
    "sender_account": "ACC001",
    "receiver_account": "ACC002",
    "reference": "INV-2024-001"
  }'
```

Response:
```json
{
  "transaction_id": "TXN-a1b2c3d4",
  "status": "PROCESSING",
  "message": "Transaction initiated successfully",
  "instant_settlement": true,
  "estimated_time": "5-30 seconds",
  "created_at": "2025-11-07T10:30:00Z",
  "compliance_check": {
    "passed": true,
    "sanctions_check": true,
    "aml_check": true
  },
  "risk_score": {
    "score": 25.5,
    "level": "LOW",
    "approved": true
  }
}
```

## Мониторинг

Gateway экспортирует метрики для Prometheus:

- `gateway_requests_total` - Total requests
- `gateway_request_duration_seconds` - Request latency
- `gateway_circuit_breaker_state` - Circuit breaker states
- `gateway_rate_limit_exceeded_total` - Rate limit rejections

## Production Checklist

- [ ] Изменить `JWT_SECRET` на криптографически стойкий ключ
- [ ] Настроить HTTPS/TLS через Envoy
- [ ] Настроить mTLS для межсервисной коммуникации
- [ ] Включить persistent storage для idempotency keys (Redis)
- [ ] Настроить log aggregation (ELK/Loki)
- [ ] Настроить distributed tracing (Jaeger/Tempo)
- [ ] Load testing (k6, 100+ TPS)
- [ ] Security audit
- [ ] Disaster recovery plan

## Troubleshooting

### Circuit Breaker открылся

```
Circuit breaker open for compliance-engine: max concurrency
```

**Решение:** Проверить health backend сервиса, увеличить `CB_MAX_CONCURRENT` или `CB_ERROR_THRESHOLD`.

### Rate Limit Exceeded

```
Rate limit exceeded. Please try again later.
```

**Решение:** Подождать 1 минуту или увеличить `RATE_LIMIT_RPM` для банка.

### Token Invalid

```
Invalid or expired token
```

**Решение:** Получить новый token через `/api/v1/auth/login`.

## Лицензия

Proprietary - DelTran MVP

## Контакты

Для вопросов: tech@deltran.io
