# План миграции Gateway Services
# Gateway Services Migration Plan

## 🎯 Текущая ситуация (Current State)

### Что есть сейчас:

1. **Gateway (Go)** - `services/gateway/`
   - ✅ Развёрнут в docker-compose.yml
   - ✅ Порт 8080
   - ✅ Работает для JSON API
   - ❌ Нет ISO 20022
   - ❌ Нет NATS integration

2. **Gateway (Rust)** - `services/gateway-rust/`
   - ✅ Код готов (production-ready)
   - ✅ ISO 20022 support (pain.001, pacs.008, camt.054)
   - ✅ NATS integration
   - ✅ PostgreSQL persistence
   - ❌ **НЕ развёрнут в docker-compose.yml**

### Проблема:

**Rust Gateway (production-ready) НЕ используется в текущей инфраструктуре!**

В `docker-compose.yml` работает **ТОЛЬКО Go Gateway**, который:
- Не поддерживает ISO 20022
- Не интегрирован с NATS
- Не сохраняет данные в PostgreSQL

**Это означает, что весь event-driven flow (Compliance → Obligation → Clearing → etc.) НЕ РАБОТАЕТ!**

---

## 🚨 Критическая проблема (Critical Issue)

### Docker Compose Configuration

```yaml
# docker-compose.yml

services:
  gateway:  # ← Это Go Gateway!
    build:
      context: ./services/gateway  # ← Go version
      dockerfile: Dockerfile
    container_name: deltran-gateway
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://...
      - REDIS_URL=redis://...
    # ❌ НЕТ NATS_URL!
    # ❌ НЕТ ISO 20022!
```

### Что это значит:

```
CLIENT → pain.001 XML
         │
         ↓
    ❌ Go Gateway (port 8080)
         │
         └─ ❌ Не понимает ISO 20022 XML
            ❌ Не публикует в NATS
            ❌ Не запускает Compliance Engine
            ❌ Не запускает весь DelTran flow

РЕЗУЛЬТАТ: ❌ DelTran MVP НЕ РАБОТАЕТ end-to-end!
```

---

## ✅ Решение (Solution)

### Вариант 1: Полная замена (Recommended)

Заменить Go Gateway на Rust Gateway в docker-compose.yml.

#### Шаги:

1. **Обновить docker-compose.yml**

```yaml
services:
  # ─────────────────────────────────────────────────
  # GATEWAY (Rust) - Production ISO 20022
  # ─────────────────────────────────────────────────
  gateway:
    build:
      context: ./services/gateway-rust  # ← ИЗМЕНИТЬ на Rust
      dockerfile: Dockerfile
    container_name: deltran-gateway-rust
    ports:
      - "8080:8080"  # ISO 20022 endpoints
    environment:
      - DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran
      - NATS_URL=nats://nats:4222  # ← ДОБАВИТЬ NATS
      - BIND_ADDR=0.0.0.0:8080
      - RUST_LOG=info,deltran_gateway=debug
    depends_on:
      - postgres
      - nats
    networks:
      - deltran-network

  # Go Gateway больше НЕ НУЖЕН для production
```

2. **Запустить миграции для Rust Gateway**

```bash
cd services/gateway-rust
sqlx migrate run
```

3. **Пересобрать и запустить**

```bash
docker-compose down
docker-compose build gateway
docker-compose up -d
```

4. **Проверить**

```bash
# Health check
curl http://localhost:8080/health

# Test pain.001 submission
curl -X POST http://localhost:8080/iso20022/pain.001 \
  -H "Content-Type: application/xml" \
  --data @test_pain001.xml
```

---

### Вариант 2: Параллельный запуск (для переходного периода)

Запустить ОБА Gateway на разных портах.

#### docker-compose.yml

```yaml
services:
  # ─────────────────────────────────────────────────
  # GATEWAY (Rust) - Production ISO 20022
  # ─────────────────────────────────────────────────
  gateway-rust:
    build:
      context: ./services/gateway-rust
      dockerfile: Dockerfile
    container_name: deltran-gateway-rust
    ports:
      - "8080:8080"  # ← ISO 20022 (PRODUCTION)
    environment:
      - DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran
      - NATS_URL=nats://nats:4222
      - BIND_ADDR=0.0.0.0:8080
      - RUST_LOG=info,deltran_gateway=debug
    depends_on:
      - postgres
      - nats
    networks:
      - deltran-network

  # ─────────────────────────────────────────────────
  # GATEWAY (Go) - Demo/UI Testing (OPTIONAL)
  # ─────────────────────────────────────────────────
  gateway-go:
    build:
      context: ./services/gateway
      dockerfile: Dockerfile
    container_name: deltran-gateway-go
    ports:
      - "8081:8080"  # ← Simple JSON API (DEMO)
    environment:
      - DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran
      - REDIS_URL=redis://redis:6379
      - GATEWAY_PORT=8080
    depends_on:
      - postgres
      - redis
    networks:
      - deltran-network
```

**Результат**:
- **Port 8080**: Rust Gateway (ISO 20022, NATS) - PRODUCTION
- **Port 8081**: Go Gateway (JSON API) - DEMO/UI

---

## 📋 Проверочный список (Checklist)

### До миграции:

- [ ] Проверить, что Rust Gateway компилируется
  ```bash
  cd services/gateway-rust
  cargo build --release
  ```

- [ ] Убедиться, что все зависимости доступны:
  - [ ] PostgreSQL running
  - [ ] NATS running
  - [ ] Rust Gateway migrations applied

### После миграции:

- [ ] Gateway (Rust) отвечает на `/health`
- [ ] pain.001 parsing работает
- [ ] pacs.008 parsing работает
- [ ] camt.054 parsing работает ⭐ **CRITICAL**
- [ ] NATS events публикуются:
  - [ ] `deltran.obligation.create`
  - [ ] `deltran.bank.camt054`
- [ ] Compliance Engine получает события
- [ ] Obligation Engine получает события
- [ ] PostgreSQL содержит записи в таблице `payments`

### Тестирование end-to-end flow:

```bash
# 1. Submit pain.001
curl -X POST http://localhost:8080/iso20022/pain.001 \
  -H "Content-Type: application/xml" \
  --data @sample_pain001.xml

# 2. Проверить, что событие опубликовано в NATS
# (Смотреть логи Compliance Engine и Obligation Engine)

# 3. Submit camt.054 (funding confirmation)
curl -X POST http://localhost:8080/iso20022/camt.054 \
  -H "Content-Type: application/xml" \
  --data @sample_camt054.xml

# 4. Проверить, что Account Monitor получил событие
# 5. Проверить, что Token Engine заминтил токены
```

---

## 🎯 Рекомендуемый план действий (Recommended Action Plan)

### Немедленно (Immediate):

1. ✅ **Добавить Rust Gateway в docker-compose.yml** (Вариант 1 или 2)
2. ✅ **Запустить миграции базы данных**
3. ✅ **Пересобрать и запустить docker-compose**
4. ✅ **Протестировать ISO 20022 endpoints**

### Краткосрочно (Short-term):

1. ✅ **Проверить end-to-end flow** (pain.001 → Obligation → Clearing → Settlement → camt.054 → Token Engine)
2. ✅ **Убедиться, что все NATS события работают**
3. ✅ **Провести нагрузочное тестирование** (K6 stress tests)

### Долгосрочно (Long-term):

1. ✅ **Удалить Go Gateway** (если больше не нужен для UI)
2. ✅ **Добавить метрики Prometheus** в Rust Gateway
3. ✅ **Добавить authentication/authorization**
4. ✅ **Настроить TLS/HTTPS**

---

## 🔧 Пример обновлённого docker-compose.yml

```yaml
version: '3.9'

services:
  # ═════════════════════════════════════════════════════════
  # INFRASTRUCTURE
  # ═════════════════════════════════════════════════════════

  postgres:
    image: postgres:15-alpine
    container_name: deltran-postgres
    environment:
      POSTGRES_DB: deltran
      POSTGRES_USER: deltran
      POSTGRES_PASSWORD: deltran_secure_pass_2024
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data
    networks:
      - deltran-network

  nats:
    image: nats:latest
    container_name: deltran-nats
    command: ["-js", "-m", "8222"]
    ports:
      - "4222:4222"  # NATS client
      - "8222:8222"  # HTTP monitoring
    networks:
      - deltran-network

  redis:
    image: redis:7-alpine
    container_name: deltran-redis
    ports:
      - "6379:6379"
    networks:
      - deltran-network

  # ═════════════════════════════════════════════════════════
  # GATEWAY - ISO 20022 Entry Point
  # ═════════════════════════════════════════════════════════

  gateway:
    build:
      context: ./services/gateway-rust  # ← RUST VERSION
      dockerfile: Dockerfile
    container_name: deltran-gateway
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran
      - NATS_URL=nats://nats:4222
      - BIND_ADDR=0.0.0.0:8080
      - RUST_LOG=info,deltran_gateway=debug
    depends_on:
      - postgres
      - nats
    networks:
      - deltran-network

  # ═════════════════════════════════════════════════════════
  # MICROSERVICES
  # ═════════════════════════════════════════════════════════

  compliance-engine:
    build:
      context: ./services/compliance-engine
      dockerfile: Dockerfile
    container_name: deltran-compliance-engine
    environment:
      - DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran
      - NATS_URL=nats://nats:4222
    depends_on:
      - postgres
      - nats
      - gateway
    networks:
      - deltran-network

  obligation-engine:
    build:
      context: ./services/obligation-engine
      dockerfile: Dockerfile
    container_name: deltran-obligation-engine
    environment:
      - DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran
      - NATS_URL=nats://nats:4222
    depends_on:
      - postgres
      - nats
      - compliance-engine
    networks:
      - deltran-network

  clearing-engine:
    build:
      context: ./services/clearing-engine
      dockerfile: Dockerfile
    container_name: deltran-clearing-engine
    environment:
      - DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran
      - NATS_URL=nats://nats:4222
    depends_on:
      - postgres
      - nats
      - obligation-engine
    networks:
      - deltran-network

  account-monitor:
    build:
      context: ./services/account-monitor
      dockerfile: Dockerfile
    container_name: deltran-account-monitor
    ports:
      - "8090:8090"
    environment:
      - DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran
      - NATS_URL=nats://nats:4222
      - MONITORED_ACCOUNTS=${MONITORED_ACCOUNTS}
    depends_on:
      - postgres
      - nats
    networks:
      - deltran-network

  token-engine:
    build:
      context: ./services/token-engine
      dockerfile: Dockerfile
    container_name: deltran-token-engine
    environment:
      - DATABASE_URL=postgresql://deltran:deltran_secure_pass_2024@postgres:5432/deltran
      - NATS_URL=nats://nats:4222
    depends_on:
      - postgres
      - nats
      - account-monitor
    networks:
      - deltran-network

networks:
  deltran-network:
    driver: bridge

volumes:
  postgres_data:
```

---

## 📊 Ожидаемый результат (Expected Outcome)

### До миграции:
```
pain.001 → Go Gateway → ❌ Не обрабатывается
                       ❌ Нет NATS
                       ❌ DelTran flow НЕ работает
```

### После миграции:
```
pain.001 → Rust Gateway → ✅ Parse XML
                       → ✅ Save to PostgreSQL
                       → ✅ Publish to NATS
                       → ✅ Compliance Engine
                       → ✅ Obligation Engine
                       → ✅ Clearing Engine
                       → ✅ ... (весь DelTran flow)
                       → ✅ Token Engine (после camt.054)
```

---

## ✅ Резюме (Summary)

### Проблема:
- В docker-compose.yml используется **Go Gateway** (без ISO 20022, без NATS)
- **Rust Gateway** (production-ready) НЕ развёрнут

### Решение:
- **Вариант 1**: Заменить Go Gateway на Rust Gateway (рекомендуется)
- **Вариант 2**: Запустить оба на разных портах (переходный период)

### Критичность:
🔴 **ВЫСОКАЯ** - Без Rust Gateway DelTran MVP НЕ работает end-to-end!

### Время на исправление:
⏱️ **15-30 минут** (обновить docker-compose.yml, rebuild, restart)

---

**Статус**: 🚨 Требуется немедленное исправление

**Приоритет**: P0 (критический)
