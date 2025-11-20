# Account Monitor Service

## Назначение (Purpose)

**Account Monitor** отслеживает реальные поступления FIAT на EMI-счета DelTran и публикует события подтверждения финансирования для Token Engine.

The Account Monitor service automatically detects real FIAT arrivals on DelTran's EMI bank accounts and publishes funding confirmation events to trigger token minting.

## Ключевые функции (Key Features)

### 1. Автоматический мониторинг счетов (Automatic Account Monitoring)

- **Периодический опрос (Polling)**: Опрос банковских API каждые 30 секунд для обнаружения новых транзакций
- **Push-уведомления (Push Notifications)**: Прослушивание camt.054 сообщений в режиме реального времени

### 2. Сопоставление транзакций (Transaction Matching)

Автоматическое сопоставление входящих FIAT с ожидающими платежами:

- **Первичное сопоставление**: По `end_to_end_id` (ISO 20022 reference)
- **Резервное сопоставление**: По сумме + валюта + временной интервал (±5 минут)

### 3. Публикация событий (Event Publishing)

При успешном сопоставлении:
- Сохранение в таблицу `funding_events`
- Публикация события `deltran.funding.confirmed` в NATS
- Token Engine получает событие и минтит токены (1:1 backing)

### 4. Ручная обработка (Manual Review)

Несопоставленные транзакции сохраняются в `unmatched_transactions` для ручной проверки оператором.

## Архитектура (Architecture)

```
┌─────────────────────────────────────────────────────────────┐
│                    Account Monitor Service                   │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────┐         ┌──────────────┐                  │
│  │ Cron Jobs    │         │ NATS Listener│                  │
│  │ (every 30s)  │         │ (camt.054)   │                  │
│  └──────┬───────┘         └──────┬───────┘                  │
│         │                        │                           │
│         └────────┬───────────────┘                           │
│                  │                                           │
│         ┌────────▼────────┐                                  │
│         │ Monitor Logic   │                                  │
│         │ - Match txns    │                                  │
│         │ - Validate      │                                  │
│         └────────┬────────┘                                  │
│                  │                                           │
│         ┌────────▼────────────────────┐                      │
│         │                             │                      │
│    ┌────▼────┐              ┌─────────▼──────┐              │
│    │Database │              │ NATS Publisher │              │
│    │ - Events│              │ (funding.      │              │
│    │ - Unmtch│              │  confirmed)    │              │
│    └─────────┘              └────────────────┘              │
│                                                               │
└─────────────────────────────────────────────────────────────┘
         │                              │
         │                              │
    ┌────▼────┐                  ┌──────▼───────┐
    │Bank APIs│                  │ Token Engine │
    │(REST/   │                  │ (mints tokens│
    │ISO20022)│                  │  after conf) │
    └─────────┘                  └──────────────┘
```

## Интеграции (Integrations)

### Банковские API (Bank APIs)

Поддерживает два типа интеграций:

1. **REST API**: HTTP-запросы к банковским API для получения списка транзакций
2. **ISO 20022 camt.052**: Стандартные сообщения запроса выписки со счета

### NATS Topics

**Subscriptions (входящие)**:
- `deltran.bank.camt054` - Получение push-уведомлений о кредитах/дебетах от банков

**Publications (исходящие)**:
- `deltran.funding.confirmed` - Уведомление Token Engine о подтвержденном финансировании

## База данных (Database Schema)

### funding_events

Хранит подтвержденные события финансирования:

```sql
id               UUID PRIMARY KEY
payment_id       UUID NOT NULL          -- Связь с payment из Obligation/Settlement
transaction_id   VARCHAR(255) UNIQUE    -- ID транзакции от банка
account_id       VARCHAR(100)           -- IBAN или другой ID счета
amount           DECIMAL(20, 4)         -- Сумма поступления
currency         VARCHAR(3)             -- Валюта (AED, USD, ILS, etc.)
end_to_end_id    VARCHAR(255)           -- ISO 20022 reference для сопоставления
debtor_name      VARCHAR(255)           -- Имя плательщика
debtor_account   VARCHAR(100)           -- Счет плательщика
booking_date     TIMESTAMP              -- Дата проводки
value_date       TIMESTAMP              -- Дата валютирования
confirmed_at     TIMESTAMP              -- Когда подтверждено и отправлено в Token Engine
```

### unmatched_transactions

Хранит несопоставленные транзакции для ручной проверки:

```sql
id                    UUID PRIMARY KEY
transaction_id        VARCHAR(255) UNIQUE
account_id            VARCHAR(100)
amount                DECIMAL(20, 4)
currency              VARCHAR(3)
credit_debit_indicator VARCHAR(4)      -- CRDT или DBIT
end_to_end_id         VARCHAR(255)
detected_at           TIMESTAMP
review_status         VARCHAR(20)      -- PENDING, MATCHED, IGNORED
matched_payment_id    UUID             -- Заполняется при ручном сопоставлении
matched_at            TIMESTAMP
matched_by            VARCHAR(100)     -- Кто выполнил ручное сопоставление
notes                 TEXT
```

## Конфигурация (Configuration)

### Переменные окружения (Environment Variables)

```bash
# Server
ACCOUNT_MONITOR_PORT=8090

# Database
DATABASE_URL=postgresql://user:password@localhost/deltran

# NATS
NATS_URL=nats://localhost:4222

# Monitored Accounts (JSON array)
MONITORED_ACCOUNTS='[
  {
    "account_id": "AE070331234567890123456",
    "currency": "AED",
    "api_type": "REST",
    "api_endpoint": "https://api.bank.ae",
    "api_key": "your_api_key"
  },
  {
    "account_id": "IL123456789012345678901",
    "currency": "ILS",
    "api_type": "ISO20022",
    "api_endpoint": "https://api.bank.il",
    "api_key": "your_api_key"
  }
]'
```

## API Endpoints

### GET /health

Health check endpoint.

**Response**:
```json
{
  "status": "healthy",
  "service": "account-monitor",
  "timestamp": "2025-01-18T12:00:00Z"
}
```

### GET /api/funding-events

Получить список событий финансирования.

**Query Parameters**:
- `payment_id` (optional): Фильтр по ID платежа
- `account_id` (optional): Фильтр по ID счета
- `limit` (default: 100): Максимальное количество записей

**Response**:
```json
{
  "events": [
    {
      "id": "uuid",
      "payment_id": "uuid",
      "transaction_id": "TXN123456",
      "account_id": "AE070331234567890123456",
      "amount": "100000.00",
      "currency": "AED",
      "end_to_end_id": "E2E123456",
      "confirmed_at": "2025-01-18T12:00:00Z"
    }
  ]
}
```

### GET /api/unmatched-transactions

Получить список несопоставленных транзакций для ручной проверки.

**Query Parameters**:
- `status` (optional): Фильтр по статусу (PENDING, MATCHED, IGNORED)
- `account_id` (optional): Фильтр по ID счета
- `limit` (default: 100): Максимальное количество записей

**Response**:
```json
{
  "transactions": [
    {
      "id": "uuid",
      "transaction_id": "TXN789012",
      "account_id": "AE070331234567890123456",
      "amount": "50000.00",
      "currency": "AED",
      "credit_debit_indicator": "CRDT",
      "detected_at": "2025-01-18T11:55:00Z",
      "review_status": "PENDING"
    }
  ]
}
```

### POST /api/manual-match

Выполнить ручное сопоставление несопоставленной транзакции с платежом.

**Request**:
```json
{
  "unmatched_transaction_id": "uuid",
  "payment_id": "uuid",
  "matched_by": "operator@deltran.com",
  "notes": "Manual match - amount and timing match"
}
```

**Response**:
```json
{
  "success": true,
  "funding_event_id": "uuid",
  "message": "Transaction matched and funding confirmed"
}
```

## Логика сопоставления (Matching Logic)

### 1. Первичное сопоставление (Primary Match)

```rust
// Точное совпадение по end_to_end_id
if transaction.end_to_end_id == payment.end_to_end_id {
    return Match(payment.id);
}
```

### 2. Резервное сопоставление (Fallback Match)

```rust
// Совпадение по сумме + валюта + время (±5 минут)
if transaction.amount == payment.expected_amount
   && transaction.currency == payment.expected_currency
   && abs(transaction.booking_date - payment.created_at) <= 5 minutes {
    return Match(payment.id);
}
```

### 3. Несопоставленные (Unmatched)

Если ни одно правило не сработало:
- Сохранить в `unmatched_transactions`
- Установить статус `PENDING`
- Отправить уведомление операторам (опционально)

## Запуск (Running)

### Development

```bash
cd services/account-monitor
cargo run
```

### Docker

```bash
docker build -t deltran/account-monitor:latest .
docker run -p 8090:8090 \
  -e DATABASE_URL=postgresql://user:password@db/deltran \
  -e NATS_URL=nats://nats:4222 \
  -e MONITORED_ACCOUNTS='[...]' \
  deltran/account-monitor:latest
```

### Docker Compose

```yaml
account-monitor:
  build: ./services/account-monitor
  ports:
    - "8090:8090"
  environment:
    DATABASE_URL: postgresql://postgres:postgres@postgres:5432/deltran
    NATS_URL: nats://nats:4222
    MONITORED_ACCOUNTS: |
      [
        {
          "account_id": "AE070331234567890123456",
          "currency": "AED",
          "api_type": "REST",
          "api_endpoint": "https://api.bank.ae",
          "api_key": "${BANK_AED_API_KEY}"
        }
      ]
  depends_on:
    - postgres
    - nats
```

## Мониторинг и логи (Monitoring & Logs)

### Ключевые метрики (Key Metrics)

- `account_monitor_transactions_detected_total` - Всего обнаружено транзакций
- `account_monitor_transactions_matched_total` - Успешно сопоставлено
- `account_monitor_transactions_unmatched_total` - Не сопоставлено (требуют проверки)
- `account_monitor_funding_confirmed_total` - Подтверждено финансирований
- `account_monitor_poll_duration_seconds` - Длительность опроса банковских API

### Логи (Logs)

Важные события в логах:

```
INFO  💰 Processing CREDIT transaction: 100000.00 AED (ref: E2E123456)
INFO  ✅ Matched transaction TXN123456 with payment uuid
INFO  🚀 Funding confirmed and published for payment uuid
WARN  ⚠️  No matching payment found for transaction TXN789012. Storing for manual review.
ERROR ❌ Failed to poll account AE070331234567890123456: connection timeout
```

## Безопасность (Security)

1. **API Keys**: Банковские API ключи хранятся в переменных окружения или секретах
2. **TLS/SSL**: Все подключения к банковским API через HTTPS
3. **Authentication**: Требуется аутентификация для ручного сопоставления транзакций
4. **Audit Log**: Все ручные действия логируются с указанием оператора

## Критические зависимости (Critical Dependencies)

- **NATS**: Для публикации событий `deltran.funding.confirmed`
- **PostgreSQL**: Для хранения событий финансирования и несопоставленных транзакций
- **Bank APIs**: Для получения информации о транзакциях

## Что дальше (Next Steps)

1. **Интеграция с реальными банками**: Замена mock данных на реальные API ключи и endpoints
2. **ML-модель для сопоставления**: Улучшение точности автоматического сопоставления
3. **Dashboard для операторов**: UI для ручной обработки несопоставленных транзакций
4. **Алерты**: Уведомления при большом количестве несопоставленных транзакций
5. **Reconciliation**: Ежедневная сверка между банковскими выписками и внутренними записями
