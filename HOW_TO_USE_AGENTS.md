# Как использовать исправленных агентов DelTran MVP

## 🎯 Быстрый старт

### 1. Проверить что сервисы запущены

```bash
# Проверить все сервисы
for port in 8080 8081 8082 8083 8084 8085 8086 8087 8088 8089 8093; do
    echo -n "Port $port: "
    curl -s "http://localhost:$port/health" > /dev/null && echo "✅ UP" || echo "❌ DOWN"
done
```

### 2. Запустить агентов в правильном порядке

#### Шаг 1: Agent-Security (JWT + Rate Limiting)

```bash
# Агент добавит:
# - JWT middleware в Rust/Go сервисы
# - Rate limiting в сервисы где его нет
# - Audit logging

# Что проверит агент:
# - Существующие middleware
# - JWT уже в Gateway (пропустит)
# - Где еще нет JWT auth

# Context7 команды которые использует агент:
context7 docs actix-web middleware
context7 docs jsonwebtoken validation
context7 docs governor rate-limiting
```

**Ожидаемый результат:**
- JWT middleware в Token Engine, Obligation Engine, и др.
- Rate limiting в сервисах где его нет
- Audit logging во всех сервисах

#### Шаг 2: Agent-Analytics (Prometheus + Grafana)

```bash
# Агент добавит:
# - /metrics endpoints в сервисы
# - Улучшит Analytics Collector
# - Создаст Grafana dashboards

# Что проверит агент:
# - Существующие /metrics endpoints
# - Analytics Collector уже создан (улучшит)
# - Prometheus configuration

# Context7 команды:
context7 docs prometheus "rust actix-web"
context7 docs grafana "prometheus dashboard"
```

**Ожидаемый результат:**
- `/metrics` endpoint в каждом сервисе
- Prometheus scraping всех сервисов
- Grafana dashboard на http://localhost:3000

**Запуск monitoring:**
```bash
cd monitoring
docker-compose up -d

# Проверить
curl http://localhost:9090/targets  # Prometheus
curl http://localhost:3000          # Grafana
```

#### Шаг 3: Agent-Performance (K6 тесты)

```bash
# Агент создаст:
# - K6 тесты для всех 11 сервисов
# - Integration tests
# - Load tests
# - E2E transaction flow tests

# Что проверит агент:
# - Сервисы запущены на портах 8080-8093
# - Существующие K6 тесты
# - Analytics Collector для записи результатов

# Context7 команды:
context7 docs k6 "load testing examples"
context7 docs k6 "websocket testing"
```

**Запуск тестов:**
```bash
cd tests/k6
chmod +x run-all-tests.sh
./run-all-tests.sh

# Результаты в:
# - tests/k6/results/*.json
# - Analytics Collector (http://localhost:8093/metrics/dashboard)
```

#### Шаг 4: Agent-Integration (Circuit Breakers + Retry)

```bash
# Агент добавит:
# - Circuit breakers в Gateway
# - Retry logic с exponential backoff
# - Улучшит NATS messaging
# - Health check aggregator

# Что проверит агент:
# - Существующие NATS producers/consumers
# - HTTP клиенты в Gateway
# - Hystrix уже в Gateway (проверит)

# Context7 команды:
context7 docs nats "rust jetstream"
context7 docs hystrix "go circuit breaker"
```

**Проверка:**
```bash
# Circuit breaker status
curl http://localhost:8080/health/all

# NATS connection
curl http://localhost:8080/metrics | grep nats
```

## 🔍 Проверка результатов

### После Agent-Security:

```bash
# JWT endpoints
curl -H "Authorization: Bearer TOKEN" http://localhost:8081/tokens

# Rate limiting headers
curl -v http://localhost:8080/api/v1/banks | grep X-RateLimit

# Audit logs
grep "audit_log" services/*/logs/*.log
```

### После Agent-Analytics:

```bash
# Prometheus metrics
curl http://localhost:8081/metrics
curl http://localhost:8082/metrics

# Analytics dashboard
curl http://localhost:8093/metrics/dashboard | jq

# Grafana
open http://localhost:3000  # admin/admin
```

### После Agent-Performance:

```bash
# K6 results
cat tests/k6/results/integration.json | jq '.metrics'

# Analytics collector
curl http://localhost:8093/metrics/performance/LOAD-* | jq
```

### После Agent-Integration:

```bash
# Health aggregator
curl http://localhost:8080/health/all | jq

# Circuit breaker status
curl http://localhost:8080/metrics | grep hystrix
```

## 🚨 Troubleshooting

### Агент создает дубликаты

**Проблема**: Агент пытается создать то что уже есть

**Решение**:
1. Агент ДОЛЖЕН сначала сканировать:
   ```bash
   grep -r "jwt" services/gateway/
   curl http://localhost:8080/metrics
   ```

2. Проверить раздел "🔍 ПЕРВЫЙ ШАГ: Сканирование" в агенте

### Context7 не работает

**Проблема**: Агент не может получить документацию

**Решение**:
1. Проверить Context7 установлен:
   ```bash
   which context7
   context7 --version
   ```

2. Добавить API key если нужно:
   ```bash
   export CONTEXT7_API_KEY=your_key
   ```

### Сервисы не запущены

**Проблема**: Агенты не могут подключиться к сервисам

**Решение**:
1. Запустить все сервисы:
   ```bash
   # Rust сервисы
   cd services/token-engine && cargo run &
   cd services/obligation-engine && cargo run &
   # ... и т.д.

   # Go сервисы
   cd services/gateway && go run main_enhanced.go &
   cd services/notification-engine && go run cmd/server/main.go &

   # Python сервис
   cd services/analytics-collector && python main.py &
   ```

2. Проверить что все работают:
   ```bash
   for port in 8080 8081 8082 8083 8084 8085 8086 8087 8088 8089 8093; do
       curl -s "http://localhost:$port/health"
   done
   ```

### Агент не проверяет существующий код

**Проблема**: Агент сразу начинает изменения

**Решение**:
1. Проверить что в агенте есть раздел "🔍 ПЕРВЫЙ ШАГ: Сканирование"

2. Агент ДОЛЖЕН выполнить команды сканирования перед изменениями

3. Если агент пропускает сканирование - это БАГ в агенте

## 📝 Checklist перед запуском агента

- [ ] Все 11 сервисов запущены и healthy
- [ ] Context7 установлен и настроен
- [ ] PostgreSQL работает (для Analytics)
- [ ] Redis работает (для кэширования)
- [ ] NATS JetStream работает (для messaging)
- [ ] Прочитан ENHANCED_SERVICES_README.md
- [ ] Проверен AGENTS_REFACTORING_REPORT.md

## 🎓 Best Practices

### 1. Всегда начинать со сканирования

```bash
# Перед любым изменением:
grep -r "функция" services/*/src/
cat services/service-name/src/main.rs
curl http://localhost:PORT/endpoint
```

### 2. Использовать Context7 для актуальных patterns

```bash
# Не гуглить - использовать Context7:
context7 resolve library-name
context7 docs library-name "pattern description"
```

### 3. Тестировать после каждого изменения

```bash
# После добавления middleware:
cargo test
go test ./...

# После изменения интеграции:
./test-integration.sh
```

### 4. Проверять что ничего не сломалось

```bash
# Health checks
curl http://localhost:8080/health/all

# Metrics
curl http://localhost:9090/targets

# Logs
tail -f services/*/logs/*.log
```

## 🔄 Workflow

```
1. Прочитать агента
   └─> Понять что он УЛУЧШАЕТ, а не СОЗДАЕТ

2. Проверить что сервисы запущены
   └─> curl http://localhost:PORT/health

3. Запустить сканирование
   └─> grep -r "..." services/*/

4. Использовать Context7
   └─> context7 docs library "pattern"

5. Добавить улучшения
   └─> ТОЛЬКО там где их НЕТ

6. Тестировать
   └─> cargo test && go test ./...

7. Проверить метрики
   └─> curl http://localhost:9090/targets

8. Следующий агент
   └─> Повторить процесс
```

## 📚 Полезные ссылки

- [AGENTS_REFACTORING_REPORT.md](./AGENTS_REFACTORING_REPORT.md) - Полный отчет о рефакторинге
- [ENHANCED_SERVICES_README.md](./ENHANCED_SERVICES_README.md) - Руководство по улучшенным сервисам
- [SESSION_REPORT_20251110.md](./SESSION_REPORT_20251110.md) - Исходный отчет о сессии

## 🎯 Итог

Все агенты исправлены и готовы к использованию. Они:
- ✅ УЛУЧШАЮТ существующие сервисы
- ✅ СКАНИРУЮТ перед изменениями
- ✅ Используют Context7
- ✅ НЕ дублируют функциональность
- ✅ Следуют единому шаблону

**Следующий шаг**: Запустить Agent-Security для добавления JWT middleware в Rust сервисы
