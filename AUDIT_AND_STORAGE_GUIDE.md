# 📁 DelTran Audit & Storage System - Big Four Compliance Level

## 🎯 Где хранятся транзакции и логи

### 1. **ТРАНЗАКЦИИ** (Immutable Financial Records)

#### 📍 Основная таблица: `deltran.transaction_ledger`
**Расположение:** PostgreSQL база данных `deltran`
**Файл схемы:** [`infra/sql/004_audit_and_logging_system.sql`](infra/sql/004_audit_and_logging_system.sql)

**Что хранится:**
- ✅ Все финансовые транзакции с криптографическими хешами (SHA-256)
- ✅ Цифровые подписи Ed25519 для каждой транзакции
- ✅ Балансы до и после транзакции (для reconciliation)
- ✅ FX курсы и settlement данные
- ✅ Blockchain-style chaining (previous_hash → current_hash)
- ✅ **ИММУТАБЕЛЬНОСТЬ:** После posting транзакции нельзя изменить/удалить

**Физическое расположение:**
```
Docker Volume: /var/lib/docker/volumes/infra_postgres-primary-data/_data
PostgreSQL Path: /var/lib/postgresql/data/base/16384/17259
```

**Как посмотреть:**
```sql
-- Все транзакции
SELECT * FROM deltran.transaction_ledger ORDER BY booking_date DESC LIMIT 100;

-- Экспорт в CSV
\copy (SELECT * FROM deltran.v_transaction_ledger_export) TO '/tmp/transactions.csv' CSV HEADER;
```

**Экспорт через API:**
```bash
curl -X POST http://localhost:8080/api/v1/audit/export/ledger \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "start_date": "2025-10-01T00:00:00Z",
    "end_date": "2025-10-14T23:59:59Z",
    "format": "xlsx",
    "include_metadata": true
  }'
```

---

### 2. **СИСТЕМНЫЕ ЛОГИ** (Application Events)

#### 📍 Таблица: `deltran.system_logs` (партиционированная по месяцам)
**Файл схемы:** [`infra/sql/004_audit_and_logging_system.sql`](infra/sql/004_audit_and_logging_system.sql)

**Что хранится:**
- 🔍 DEBUG, INFO, WARN, ERROR, FATAL события
- 🔍 Gateway, Settlement Engine, Risk Engine логи
- 🔍 Stack traces и контекстная информация
- 🔍 Request ID и Correlation ID для трейсинга

**Партиции:**
- `deltran.system_logs_2025_10` - Октябрь 2025
- `deltran.system_logs_2025_11` - Ноябрь 2025
- `deltran.system_logs_2025_12` - Декабрь 2025

**Как посмотреть:**
```sql
-- Ошибки за последние 24 часа
SELECT * FROM deltran.system_logs
WHERE log_level IN ('ERROR', 'FATAL')
  AND timestamp > NOW() - INTERVAL '24 hours'
ORDER BY timestamp DESC;

-- Логи по конкретному payment
SELECT * FROM deltran.system_logs
WHERE payment_id = 'YOUR_PAYMENT_UUID'
ORDER BY timestamp;
```

---

### 3. **AUDIT TRAIL** (Big Four Compliance)

#### 📍 Таблица: `deltran.audit_trail` (партиционированная)
**Файл схемы:** [`infra/sql/004_audit_and_logging_system.sql`](infra/sql/004_audit_and_logging_system.sql)

**Compliance Level:**
- ✅ SOX (Sarbanes-Oxley)
- ✅ IFRS 9 (Financial Instruments)
- ✅ Basel III (Banking Regulation)
- ✅ PCI DSS Level 1

**Что хранится:**
- 📝 Все действия пользователей (CREATE, READ, UPDATE, DELETE)
- 📝 Login/Logout события с IP адресами
- 📝 Изменения конфигурации и прав доступа
- 📝 MFA verification статусы
- 📝 Old Values vs New Values (полный diff)
- 📝 Regulatory impact assessment (LOW/MEDIUM/HIGH/CRITICAL)
- 📝 Sign-off tracking для критических операций

**Retention Policy:**
- По умолчанию: **7 лет** (Big Four standard)
- Автоматическое удаление после `purge_after_date`
- Legal Hold опция для судебных процессов

**Как посмотреть:**
```sql
-- Audit trail по конкретному пользователю
SELECT * FROM deltran.audit_trail
WHERE actor_email = 'user@example.com'
ORDER BY timestamp DESC;

-- Критические события за период
SELECT * FROM deltran.audit_trail
WHERE regulatory_impact IN ('HIGH', 'CRITICAL')
  AND timestamp BETWEEN '2025-10-01' AND '2025-10-14'
ORDER BY timestamp DESC;
```

**Экспорт через API:**
```bash
curl -X POST http://localhost:8080/api/v1/audit/export/trail \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "start_date": "2025-10-01T00:00:00Z",
    "end_date": "2025-10-14T23:59:59Z",
    "compliance_type": "SOX",
    "format": "xlsx",
    "include_metadata": true
  }'
```

---

### 4. **RECONCILIATION LOG** (External Audit)

#### 📍 Таблица: `deltran.reconciliation_log`
**Файл схемы:** [`infra/sql/004_audit_and_logging_system.sql`](infra/sql/004_audit_and_logging_system.sql)

**Что хранится:**
- 💰 Daily settlement reconciliation
- 💰 Nostro account balances
- 💰 Inter-bank reconciliation
- 💰 Month-end и Year-end closing
- 💰 Variance analysis
- 💰 External audit references

**Reconciliation Types:**
- `DAILY_SETTLEMENT` - Ежедневная сверка
- `NOSTRO_ACCOUNT` - Сверка Nostro счетов
- `INTER_BANK` - Межбанковская сверка
- `MONTH_END` - Месячное закрытие
- `YEAR_END` - Годовое закрытие
- `EXTERNAL_AUDIT` - Внешний аудит

**Как посмотреть:**
```sql
-- Reconciliation за дату
SELECT * FROM deltran.v_reconciliation_export
WHERE reconciliation_date = '2025-10-14';

-- Variance report
SELECT bank_name, currency, variance, status
FROM deltran.reconciliation_log
WHERE ABS(variance) > 100
  AND status != 'resolved'
ORDER BY ABS(variance) DESC;
```

**Экспорт через API:**
```bash
curl -X POST http://localhost:8080/api/v1/audit/export/reconciliation \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "start_date": "2025-10-01T00:00:00Z",
    "end_date": "2025-10-14T23:59:59Z",
    "format": "csv"
  }'
```

---

### 5. **COMPLIANCE EVIDENCE** (Cryptographic Proof)

#### 📍 Таблица: `deltran.compliance_evidence`
**Файл схемы:** [`infra/sql/004_audit_and_logging_system.sql`](infra/sql/004_audit_and_logging_system.sql)

**Что хранится:**
- 🔐 Цифровые подписи транзакций
- 🔐 Сертификаты и certificate chains
- 🔐 Timestamp Authority proofs
- 🔐 AML и Sanctions screening результаты
- 🔐 KYC verification документы
- 🔐 SHA-256 хеши для integrity verification

**Evidence Types:**
- `TRANSACTION_PROOF` - Доказательство транзакции
- `AML_CHECK` - Anti-Money Laundering проверка
- `SANCTIONS_SCREENING` - Проверка санкционных списков
- `KYC_VERIFICATION` - Know Your Customer верификация
- `AUTHORIZATION_RECORD` - Запись авторизации
- `SYSTEM_CONFIGURATION` - Системная конфигурация
- `RECONCILIATION_PROOF` - Доказательство сверки

**Retention:**
- По умолчанию: **10 лет**
- Legal Hold флаг для блокировки удаления

---

## 📊 Big Four Export Formats

### Поддерживаемые форматы экспорта:

1. **CSV** - Comma-Separated Values
   - ✅ Excel compatible
   - ✅ Легкий импорт в аналитические системы
   - ✅ Малый размер файла

2. **XLSX** - Microsoft Excel
   - ✅ Форматированные таблицы
   - ✅ Множественные sheets
   - ✅ Готово для Big Four аудиторов

3. **JSON** - JavaScript Object Notation
   - ✅ Структурированные данные
   - ✅ API integration ready
   - ✅ Полная метадата

---

## 🔧 API Endpoints для экспорта

### 1. Audit Trail Export
```http
POST /api/v1/audit/export/trail
Authorization: Bearer {jwt_token}
Content-Type: application/json

{
  "start_date": "2025-10-01T00:00:00Z",
  "end_date": "2025-10-14T23:59:59Z",
  "entity_type": "payment",
  "compliance_type": "SOX",
  "format": "xlsx",
  "include_metadata": true
}
```

### 2. Transaction Ledger Export
```http
POST /api/v1/audit/export/ledger
Authorization: Bearer {jwt_token}
Content-Type: application/json

{
  "start_date": "2025-10-01T00:00:00Z",
  "end_date": "2025-10-14T23:59:59Z",
  "format": "csv",
  "include_metadata": false
}
```

### 3. Reconciliation Export
```http
POST /api/v1/audit/export/reconciliation
Authorization: Bearer {jwt_token}
Content-Type: application/json

{
  "start_date": "2025-10-01T00:00:00Z",
  "end_date": "2025-10-14T23:59:59Z",
  "format": "xlsx"
}
```

**Response:**
```json
{
  "file_path": "audit_trail_SOX_20251014_093000.xlsx",
  "record_count": 15234,
  "generated_at": "2025-10-14T09:30:00Z",
  "exported_by": "admin@deltran.io",
  "report_type": "audit_trail",
  "compliance_ref": "BIG4-AUDIT-20251014-093000"
}
```

---

## 📂 Файловая структура

```
MVP DelTran/
│
├── infra/sql/
│   └── 004_audit_and_logging_system.sql      # 🔑 ГЛАВНЫЙ ФАЙЛ СХЕМЫ
│
├── gateway-go/
│   ├── internal/audit/
│   │   └── exporter.go                        # 🔑 ЭКСПОРТ ЛОГИКА
│   │
│   └── cmd/gateway/
│       └── main.go                            # API endpoints
│
├── ledger-core/src/
│   └── crypto.rs                              # Ed25519 подписи, SHA-256
│
├── AUDIT_AND_STORAGE_GUIDE.md                 # 🔑 ЭТОТ ФАЙЛ
│
└── all_payments_export.csv                    # Текущий экспорт транзакций
```

---

## 🚀 Quick Start - Экспорт данных

### Вариант 1: Через PostgreSQL (прямой доступ)
```bash
# Подключиться к базе данных
docker exec -it deltran-postgres-primary psql -U deltran_app -d deltran

# Экспорт audit trail в CSV
\copy (SELECT * FROM deltran.v_big_four_audit_export WHERE audit_timestamp >= '2025-10-01') TO '/tmp/audit_trail.csv' CSV HEADER;

# Экспорт transaction ledger в CSV
\copy (SELECT * FROM deltran.v_transaction_ledger_export WHERE booking_date >= '2025-10-01') TO '/tmp/transactions.csv' CSV HEADER;

# Копировать файл на host
docker cp deltran-postgres-primary:/tmp/audit_trail.csv ./audit_trail.csv
```

### Вариант 2: Через API (рекомендуется для production)
```bash
# Получить JWT token
TOKEN=$(curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@deltran.io","password":"your_password"}' | jq -r .access_token)

# Экспорт audit trail
curl -X POST http://localhost:8080/api/v1/audit/export/trail \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "start_date": "2025-10-01T00:00:00Z",
    "end_date": "2025-10-14T23:59:59Z",
    "compliance_type": "SOX",
    "format": "xlsx"
  }' | jq .
```

---

## 🔒 Криптографические ключи

**⚠️ ВАЖНО:** Криптографические ключи **НЕ хранятся** в базе данных в открытом виде!

### Где используются ключи:

1. **Runtime Memory** (Gateway Service)
   - Ed25519 signing keys для SWIFT сообщений
   - Жизненный цикл: только во время работы процесса

2. **Redis Cache** (временное хранение)
   - Расположение: `deltran-redis-master` контейнер
   - TTL: 15 минут для session keys
   - TTL: 24 часа для idempotency keys

3. **ledger-core/src/crypto.rs**
   - Модуль генерации и проверки подписей
   - SHA-256 хеширование
   - Merkle tree construction

### Криптографические данные в БД:

```sql
-- Хеши транзакций
SELECT transaction_reference, transaction_hash, digital_signature
FROM deltran.transaction_ledger
LIMIT 10;

-- Compliance evidence с цифровыми подписями
SELECT evidence_reference, evidence_hash, digital_signature, certificate_chain
FROM deltran.compliance_evidence
WHERE digital_signature IS NOT NULL;
```

---

## 📈 Статистика хранилища

### Проверить размер таблиц:
```sql
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size,
    pg_total_relation_size(schemaname||'.'||tablename) AS size_bytes
FROM pg_tables
WHERE schemaname = 'deltran'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

### Количество записей:
```sql
SELECT
    'payments' AS table_name, COUNT(*) FROM deltran.payments
UNION ALL SELECT
    'transaction_ledger', COUNT(*) FROM deltran.transaction_ledger
UNION ALL SELECT
    'audit_trail', COUNT(*) FROM deltran.audit_trail
UNION ALL SELECT
    'system_logs', COUNT(*) FROM deltran.system_logs
UNION ALL SELECT
    'reconciliation_log', COUNT(*) FROM deltran.reconciliation_log;
```

---

## 🎯 Big Four Compliance Checklist

- ✅ **Immutable Ledger** - Транзакции нельзя изменить после posting
- ✅ **Complete Audit Trail** - Все действия пользователей логируются
- ✅ **Cryptographic Proof** - SHA-256 хеши + Ed25519 подписи
- ✅ **Blockchain Chaining** - Previous hash → Current hash
- ✅ **Retention Policy** - 7-10 лет автоматическое хранение
- ✅ **Reconciliation** - Daily/Monthly/Yearly сверки
- ✅ **Export Formats** - CSV, XLSX, JSON для аудиторов
- ✅ **Regulatory Tags** - SOX, IFRS-9, Basel-III, PCI-DSS
- ✅ **MFA Tracking** - Multi-Factor Authentication logs
- ✅ **Variance Analysis** - Автоматическое обнаружение расхождений

---

## 📞 Дополнительная информация

**Документация:**
- [Schema File](infra/sql/004_audit_and_logging_system.sql) - Полная схема БД
- [Exporter Code](gateway-go/internal/audit/exporter.go) - Go код экспорта
- [Crypto Module](ledger-core/src/crypto.rs) - Rust криптография

**Контакты:**
- GitHub Issues: https://github.com/deltran/mvp/issues
- Email: compliance@deltran.io
