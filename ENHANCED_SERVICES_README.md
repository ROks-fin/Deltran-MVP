# DelTran MVP - Enhanced Services Guide

## 🎯 Обзор улучшений

Все сервисы DelTran MVP были улучшены с использованием актуальной документации из Context7. Добавлены:

- ✅ JWT авторизация в Gateway
- ✅ Многоуровневый Rate Limiting
- ✅ Интеграция с Analytics Collector
- ✅ Audit logging middleware
- ✅ Security headers
- ✅ Real-time метрики

## 📋 Новые сервисы

### 1. Analytics Collector (Python FastAPI)
**Порт**: 8093
**Описание**: Собирает и анализирует метрики производительности транзакций

**Запуск**:
```bash
cd services/analytics-collector
pip install -r requirements.txt
python main.py
```

**API Endpoints**:
- `POST /events/transaction` - Записать событие транзакции
- `POST /transactions` - Создать транзакцию
- `GET /metrics/dashboard` - Получить метрики (последние 5 мин)
- `GET /metrics/performance/{test_run_id}` - Метрики тестового запуска
- `GET /transactions/{id}` - Детали транзакции

**Документация**: http://localhost:8093/docs

### 2. Enhanced Gateway (Go)
**Порт**: 8080
**Описание**: Улучшенный Gateway с JWT auth, rate limiting и аналитикой

**Запуск**:
```bash
cd services/gateway

# С базовой конфигурацией
go run main_enhanced.go

# С полной конфигурацией (auth + rate limiting + analytics)
ENABLE_AUTH=true ENABLE_RATE_LIMIT=true ENABLE_ANALYTICS=true \
JWT_SECRET=your-secret-key \
ANALYTICS_URL=http://localhost:8093 \
go run main_enhanced.go
```

**Environment Variables**:
- `GATEWAY_PORT` - Порт (default: 8080)
- `ENABLE_AUTH` - Включить JWT авторизацию (default: false)
- `ENABLE_RATE_LIMIT` - Включить rate limiting (default: true)
- `ENABLE_ANALYTICS` - Включить аналитику (default: true)
- `JWT_SECRET` - Секретный ключ для JWT
- `ANALYTICS_URL` - URL Analytics Collector

**Rate Limiting Tiers**:
- `anonymous`: 10 req/min
- `basic`: 100 req/min
- `premium`: 1000 req/min
- `admin`: 10000 req/min

## 🗄️ База данных для аналитики

### Создание базы данных:
```bash
# Windows (PowerShell)
createdb deltran_analytics

# Применить миграции
psql -U postgres -d deltran_analytics -f migrations/001_create_analytics_db.sql
```

### Структура БД:
- `transaction_analytics` - Все транзакции с метриками производительности
- `tokens` - Токенизированные активы
- `performance_metrics` - Агрегированные метрики
- `dashboard_metrics` (VIEW) - Real-time метрики
- `test_run_summary` (VIEW) - Сводка по тестовым запускам

## 🔐 JWT Authentication

### Получение токена:

Для тестирования можно использовать существующий Auth Service или создать токен вручную:

```bash
# Тестовые пользователи (если Auth Service запущен на 8094)
curl -X POST http://localhost:8094/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "admin123"
  }'
```

### Использование токена:
```bash
curl -X POST http://localhost:8080/api/v1/transfer \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "sender_bank": "ICICI",
    "receiver_bank": "ENBD",
    "amount": 10000,
    "from_currency": "INR",
    "to_currency": "AED",
    "sender_account": "ACC001",
    "receiver_account": "ACC002"
  }'
```

## 📊 Middleware Stack

Gateway middleware применяется в следующем порядке:

1. **CORS Middleware** - Обрабатывает CORS headers
2. **Logging Middleware** - Логирует все запросы
3. **Security Headers** - Добавляет security headers
4. **Analytics Middleware** - Записывает метрики (опционально)
5. **Rate Limiting** - Ограничивает частоту запросов (опционально)
6. **JWT Authentication** - Проверяет токены (опционально)
7. **Permission Check** - Проверяет права доступа (для защищенных эндпоинтов)

## 🧪 Тестирование

### 1. Проверка здоровья сервисов:
```bash
# Analytics Collector
curl http://localhost:8093/health

# Gateway
curl http://localhost:8080/health
```

### 2. Тест без авторизации:
```bash
# Получить список банков (публичный endpoint)
curl http://localhost:8080/api/v1/banks
```

### 3. Тест rate limiting:
```bash
# Отправить 15 запросов (лимит: 10/мин для anonymous)
for i in {1..15}; do
  curl http://localhost:8080/api/v1/banks
  echo ""
done

# Последние 5 запросов должны вернуть 429 (Too Many Requests)
```

### 4. Тест с авторизацией:
```bash
# 1. Получить JWT token
TOKEN=$(curl -s -X POST http://localhost:8094/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' | jq -r '.access_token')

# 2. Использовать токен
curl -X POST http://localhost:8080/api/v1/transfer \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{
    "sender_bank": "ICICI",
    "receiver_bank": "ENBD",
    "amount": 10000,
    "from_currency": "INR",
    "to_currency": "AED"
  }'
```

### 5. Проверка метрик:
```bash
# Real-time dashboard metrics
curl http://localhost:8093/metrics/dashboard | jq

# Список транзакций
curl http://localhost:8093/transactions?limit=10 | jq

# Детали транзакции
curl http://localhost:8093/transactions/TXN-12345 | jq
```

## 📈 Мониторинг и метрики

### Rate Limit Headers:
Каждый ответ включает rate limit информацию:
```
X-RateLimit-Tier: admin
X-RateLimit-Limit: 10000
X-RateLimit-Remaining: 9999
X-RateLimit-Reset: 1699999999
```

### Analytics Events:
Каждая транзакция генерирует события:
1. `gateway` - Запрос получен Gateway
2. `clearing_start` - Начало клиринга
3. `clearing_complete` - Клиринг завершен
4. `settlement_start` - Начало расчетов
5. `settlement_complete` - Расчеты завершены
6. `completed` / `failed` - Финальный статус

### Просмотр метрик в реальном времени:
```sql
-- Подключиться к БД
psql -U postgres -d deltran_analytics

-- Посмотреть метрики за последние 5 минут
SELECT * FROM dashboard_metrics;

-- Топ 10 медленных транзакций
SELECT transaction_id, total_latency, status, timestamp
FROM transaction_analytics
ORDER BY total_latency DESC NULLS LAST
LIMIT 10;

-- Статистика по тестовым запускам
SELECT * FROM test_run_summary
ORDER BY test_start DESC;
```

## 🔧 Конфигурация для production

### Gateway (.env):
```bash
GATEWAY_PORT=8080
ENABLE_AUTH=true
ENABLE_RATE_LIMIT=true
ENABLE_ANALYTICS=true
JWT_SECRET=your-very-secure-secret-key-here
ANALYTICS_URL=http://analytics-collector:8093
```

### Analytics Collector (.env):
```bash
DATABASE_URL=postgresql://postgres:password@localhost:5432/deltran_analytics
```

## 🚀 Быстрый старт (все сервисы)

```bash
# 1. Создать БД для аналитики
createdb deltran_analytics
psql -U postgres -d deltran_analytics -f migrations/001_create_analytics_db.sql

# 2. Запустить Analytics Collector
cd services/analytics-collector
pip install -r requirements.txt
python main.py &

# 3. Запустить Enhanced Gateway
cd ../gateway
ENABLE_ANALYTICS=true go run main_enhanced.go &

# 4. Проверить работу
curl http://localhost:8080/health
curl http://localhost:8093/health

# 5. Отправить тестовую транзакцию
curl -X POST http://localhost:8080/api/v1/transfer \
  -H "Content-Type: application/json" \
  -d '{
    "sender_bank": "ICICI",
    "receiver_bank": "ENBD",
    "amount": 10000,
    "from_currency": "INR",
    "to_currency": "AED",
    "sender_account": "ACC001",
    "receiver_account": "ACC002"
  }'

# 6. Проверить метрики
curl http://localhost:8093/metrics/dashboard | jq
```

## 📚 Дополнительные ресурсы

### Документация API:
- Analytics Collector: http://localhost:8093/docs
- Gateway Swagger (если настроен): http://localhost:8080/swagger

### Middleware:
- `services/gateway/middleware/auth.go` - JWT authentication
- `services/gateway/middleware/ratelimit.go` - Rate limiting
- `services/gateway/middleware/analytics.go` - Analytics integration

### Миграции БД:
- `migrations/001_create_analytics_db.sql` - Основная схема

## 🔍 Troubleshooting

### Analytics Collector не запускается:
```bash
# Проверить доступность БД
psql -U postgres -d deltran_analytics -c "SELECT 1;"

# Проверить логи
python main.py
```

### Gateway не может подключиться к Analytics:
```bash
# Проверить что Analytics Collector запущен
curl http://localhost:8093/health

# Проверить переменную окружения
echo $ANALYTICS_URL
```

### Rate limiting не работает:
```bash
# Убедитесь что ENABLE_RATE_LIMIT=true
ENABLE_RATE_LIMIT=true go run main_enhanced.go

# Проверить headers в ответе
curl -v http://localhost:8080/api/v1/banks
```

### JWT authentication не работает:
```bash
# Проверить что Auth Service запущен (если используется)
curl http://localhost:8094/health

# Проверить формат токена
curl -H "Authorization: Bearer YOUR_TOKEN" http://localhost:8080/api/v1/transfer
```

## 💡 Советы по использованию

1. **Для локальной разработки**: Отключите auth (`ENABLE_AUTH=false`)
2. **Для тестирования производительности**: Включите analytics (`ENABLE_ANALYTICS=true`)
3. **Для демонстрации**: Используйте все middleware с логами
4. **Для production**: Обязательно измените `JWT_SECRET` и включите HTTPS

## 🎓 Обучение и примеры

Подробные примеры использования Context7 для улучшения сервисов см. в:
- `.claude/agents/Agent-Security.md` - JWT и security patterns
- `.claude/agents/Agent-Integration.md` - Интеграция сервисов
- `.claude/agents/Agent-Analytics.md` - Метрики и мониторинг
- `.claude/agents/Agent-Performance.md` - K6 тесты

---

**Версия**: 2.0
**Дата**: 2025-11-10
**Статус**: ✅ Production Ready для локального MVP
