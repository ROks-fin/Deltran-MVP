# DelTran - Архитектура Хранения Данных

**Дата**: 2025-01-20
**Статус**: 📊 **ПОЛНАЯ КАРТА ХРАНЕНИЯ**

---

## 🗄️ ОТВЕТ: Куда сохраняются данные?

DelTran использует **PostgreSQL** как основное хранилище для всех критических данных. Каждый микросервис имеет свои собственные таблицы, но все работают с **общей базой данных**.

### Архитектура хранения:

```
┌────────────────────────────────────────────────────────────────┐
│                   PostgreSQL (Единая БД)                       │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │
│  │   Gateway    │  │  Obligation  │  │   Clearing   │        │
│  │   Tables     │  │   Tables     │  │   Tables     │        │
│  └──────────────┘  └──────────────┘  └──────────────┘        │
│                                                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │
│  │    Token     │  │     EMI      │  │   FX Rates   │        │
│  │   Tables     │  │   Accounts   │  │   Tables     │        │
│  └──────────────┘  └──────────────┘  └──────────────┘        │
└────────────────────────────────────────────────────────────────┘
           ↑                    ↑                    ↑
           │                    │                    │
    NATS Events          NATS Events          NATS Events
    (эфемерные)          (эфемерные)          (эфемерные)
```

**ВАЖНО**:
- ✅ **PostgreSQL** = постоянное хранение (persistent)
- ✅ **NATS** = транспорт событий (ephemeral, не хранит данные)
- ✅ Каждая транзакция сохраняется в БД **ДО** публикации в NATS
- ✅ Полная прослеживаемость через database audit trail

---

## 📊 ТАБЛИЦЫ ПО СЕРВИСАМ

### 1️⃣ Gateway Service

**Файл**: `services/gateway-rust/migrations/20250118_001_create_payments_table.sql`

#### Таблица: `payments`
**Назначение**: Хранение всех входящих платежей в канонической форме

```sql
CREATE TABLE payments (
    deltran_tx_id UUID PRIMARY KEY,           -- Внутренний ID DelTran
    obligation_id UUID,                       -- Связь с obligation
    uetr UUID,                                -- ISO 20022 UETR
    end_to_end_id VARCHAR(35) NOT NULL,       -- ISO 20022 E2E ID

    -- Суммы
    instructed_amount DECIMAL(18, 5) NOT NULL,
    settlement_amount DECIMAL(18, 5) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Стороны
    debtor_name VARCHAR(140),
    debtor_iban VARCHAR(34),
    creditor_name VARCHAR(140),
    creditor_iban VARCHAR(34),

    -- Банки
    debtor_agent_bic VARCHAR(11),
    creditor_agent_bic VARCHAR(11),

    -- Статус
    status VARCHAR(50) NOT NULL DEFAULT 'Received',

    -- Таймстемпы
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    funded_at TIMESTAMP,                      -- Когда получен FIAT (camt.054)
    cleared_at TIMESTAMP,
    settled_at TIMESTAMP,
    completed_at TIMESTAMP,

    -- Метаданные
    raw_iso_message TEXT,
    metadata JSONB DEFAULT '{}'
);
```

**Индексы**:
- `idx_payments_status` - быстрый поиск по статусу
- `idx_payments_end_to_end_id` - поиск по E2E ID
- `idx_payments_pending_funding` - мониторинг ожидающих FIAT

#### Таблица: `payment_events`
**Назначение**: Audit trail всех изменений платежа

```sql
CREATE TABLE payment_events (
    event_id UUID PRIMARY KEY,
    deltran_tx_id UUID NOT NULL REFERENCES payments(deltran_tx_id),
    event_type VARCHAR(50) NOT NULL,          -- 'RECEIVED', 'FUNDED', 'SETTLED'
    event_status VARCHAR(20) NOT NULL,
    event_data JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

**Файл кода**: [services/gateway-rust/src/db.rs](services/gateway-rust/src/db.rs)

---

### 2️⃣ Obligation Engine

**Файл**: Использует общую схему `infrastructure/database/migrations/001-initial-schema.sql`

#### Таблица: `obligations`
**Назначение**: Обязательства по платежам

```sql
CREATE TABLE obligations (
    id UUID PRIMARY KEY,
    window_id BIGINT REFERENCES clearing_windows(id),
    transaction_id UUID,                      -- Связь с транзакцией

    -- Стороны
    payer_id UUID NOT NULL REFERENCES banks(id),
    payee_id UUID NOT NULL REFERENCES banks(id),

    -- Сумма
    amount NUMERIC(26,8) NOT NULL CHECK (amount > 0),
    currency VARCHAR(3) NOT NULL,

    -- Статус
    status VARCHAR(20) DEFAULT 'PENDING',     -- PENDING → NETTED → SETTLED
    obligation_type VARCHAR(50) DEFAULT 'TRANSACTION',
    priority INTEGER DEFAULT 1,

    -- Метаданные
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    processed_at TIMESTAMPTZ
);
```

**Статусы Obligation**:
```
PENDING   → Создано обязательство
VALIDATED → Прошло compliance/risk
CLEARING  → В окне клиринга (international только)
NETTED    → Участвует в netting
EXECUTING → Settlement отправил pacs.008
SETTLED   → Банк подтвердил camt.054 ✅
FAILED    → Ошибка
```

**Файл кода**: [services/obligation-engine/src/database.rs](services/obligation-engine/src/database.rs) (uses shared schema)

---

### 3️⃣ Clearing Engine

**Файл**: `infrastructure/database/migrations/001-initial-schema.sql`

#### Таблица: `clearing_windows`
**Назначение**: Окна клиринга (6-часовые циклы)

```sql
CREATE TABLE clearing_windows (
    id BIGSERIAL PRIMARY KEY,
    window_name VARCHAR(100) UNIQUE NOT NULL,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    cutoff_time TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) DEFAULT 'OPEN',        -- OPEN → CLOSED → PROCESSING → COMPLETED
    region VARCHAR(50) DEFAULT 'Global',

    -- Статистика
    transactions_count INTEGER DEFAULT 0,
    obligations_count INTEGER DEFAULT 0,
    total_gross_value NUMERIC(26,8) DEFAULT 0,
    total_net_value NUMERIC(26,8) DEFAULT 0,
    saved_amount NUMERIC(26,8) DEFAULT 0,     -- Экономия от netting
    netting_efficiency NUMERIC(5,2) DEFAULT 0, -- %

    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### Таблица: `net_positions`
**Назначение**: Результаты multilateral netting

```sql
CREATE TABLE net_positions (
    id UUID PRIMARY KEY,
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    bank_pair_hash VARCHAR(100) NOT NULL,

    -- Банки
    bank_a_id UUID NOT NULL REFERENCES banks(id),
    bank_b_id UUID NOT NULL REFERENCES banks(id),
    currency VARCHAR(3) NOT NULL,

    -- Netting результаты
    gross_debit_a_to_b NUMERIC(26,8) DEFAULT 0,
    gross_credit_b_to_a NUMERIC(26,8) DEFAULT 0,
    net_amount NUMERIC(26,8) DEFAULT 0,        -- Финальная сумма
    net_direction VARCHAR(20),

    -- Метрики
    obligations_netted INTEGER DEFAULT 0,
    netting_ratio NUMERIC(5,4) DEFAULT 0,      -- Эффективность
    amount_saved NUMERIC(26,8) DEFAULT 0,      -- Сэкономлено ликвидности

    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### Таблица: `settlement_instructions`
**Назначение**: Инструкции для Settlement Engine

```sql
CREATE TABLE settlement_instructions (
    id UUID PRIMARY KEY,
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    net_position_id UUID REFERENCES net_positions(id),

    -- Платеж
    payer_bank_id UUID NOT NULL REFERENCES banks(id),
    payee_bank_id UUID NOT NULL REFERENCES banks(id),
    amount NUMERIC(26,8) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Исполнение
    instruction_type VARCHAR(50) DEFAULT 'NET_SETTLEMENT',
    status VARCHAR(20) DEFAULT 'PENDING',
    deadline TIMESTAMPTZ NOT NULL,
    sent_to_settlement_at TIMESTAMPTZ,
    settlement_id UUID,

    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**Файл кода**: [services/clearing-engine/src/database.rs](services/clearing-engine/src/database.rs)

---

### 4️⃣ Token Engine

**Файл**: Собственные таблицы (не найдена миграция, используется ORM)

#### Таблица: `tokens`
**Назначение**: Токенизированные активы (xAED, xUSD, xILS)

```sql
CREATE TABLE tokens (
    id UUID PRIMARY KEY,
    currency VARCHAR(3) NOT NULL,             -- xAED, xUSD, xILS
    amount NUMERIC(26,8) NOT NULL,
    bank_id UUID NOT NULL REFERENCES banks(id),

    -- Статус
    status VARCHAR(20) DEFAULT 'ACTIVE',      -- ACTIVE, LOCKED, BURNED

    -- Привязка к реальному FIAT
    clearing_window BIGINT,
    reference VARCHAR(255),                   -- Ссылка на camt.054 entry

    -- Таймстемпы
    created_at TIMESTAMPTZ NOT NULL,
    burned_at TIMESTAMPTZ
);
```

**Гарантия 1:1 backing**:
- Токен создается ТОЛЬКО после `camt.054 BOOKED` confirmation
- `reference` содержит bank transaction ID
- Невозможно создать токен без реального FIAT

**Файл кода**: [services/token-engine/src/database.rs](services/token-engine/src/database.rs)

---

### 5️⃣ EMI Accounts (E-Money Institution)

**Файл**: `infrastructure/database/migrations/002-emi-accounts.sql`

#### Таблица: `emi_accounts`
**Назначение**: Счета в банках-корреспондентах для хранения FIAT

```sql
CREATE TABLE emi_accounts (
    id UUID PRIMARY KEY,
    bank_id UUID NOT NULL REFERENCES banks(id),
    account_number VARCHAR(50) NOT NULL,
    iban VARCHAR(34),
    swift_bic VARCHAR(11),
    currency VARCHAR(3) NOT NULL,
    country_code VARCHAR(3) NOT NULL,

    -- Тип счета
    account_type VARCHAR(20) NOT NULL DEFAULT 'client_funds',

    -- Балансы (в валюте счета)
    ledger_balance NUMERIC(26,8) DEFAULT 0,          -- Внутренний баланс
    bank_reported_balance NUMERIC(26,8) DEFAULT 0,   -- Баланс от банка
    reserved_balance NUMERIC(26,8) DEFAULT 0,        -- Зарезервировано
    available_balance NUMERIC(26,8) AS (ledger_balance - reserved_balance),

    -- Reconciliation
    last_reconciliation_at TIMESTAMPTZ,
    reconciliation_status VARCHAR(20) DEFAULT 'PENDING',
    reconciliation_source VARCHAR(50),               -- 'camt.053', 'camt.054'
    reconciliation_difference NUMERIC(26,8) DEFAULT 0,

    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### Таблица: `emi_transactions`
**Назначение**: Все движения на EMI счетах

```sql
CREATE TABLE emi_transactions (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES emi_accounts(id),

    -- Тип транзакции
    transaction_type VARCHAR(30) NOT NULL,    -- 'funding', 'settlement', 'fee'
    direction VARCHAR(10) NOT NULL,           -- 'CREDIT', 'DEBIT'
    amount NUMERIC(26,8) NOT NULL,

    -- Балансы
    balance_before NUMERIC(26,8) NOT NULL,
    balance_after NUMERIC(26,8) NOT NULL,

    -- Связи
    related_transaction_id UUID,              -- DelTran payment
    related_settlement_id UUID,
    bank_reference VARCHAR(100),              -- Bank's txn ID
    uetr VARCHAR(36),                         -- ISO 20022 UETR

    -- ISO 20022
    iso_message_type VARCHAR(20),             -- 'pacs.008', 'camt.054'
    iso_message_id VARCHAR(35),

    -- Статус
    status VARCHAR(20) DEFAULT 'PENDING',     -- PENDING, CONFIRMED, FAILED
    confirmed_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### Таблица: `emi_account_snapshots`
**Назначение**: EOD (End-of-Day) снимки для регуляторов

```sql
CREATE TABLE emi_account_snapshots (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES emi_accounts(id),
    snapshot_date DATE NOT NULL,
    snapshot_time TIMESTAMPTZ NOT NULL,

    ledger_balance NUMERIC(26,8) NOT NULL,
    bank_reported_balance NUMERIC(26,8) NOT NULL,
    reserved_balance NUMERIC(26,8) NOT NULL,
    available_balance NUMERIC(26,8) NOT NULL,

    difference NUMERIC(26,8) DEFAULT 0,
    reconciled BOOLEAN DEFAULT FALSE,

    statement_reference VARCHAR(100),         -- camt.053 reference

    UNIQUE(account_id, snapshot_date)
);
```

---

### 6️⃣ Account Monitor Service

**Файл**: `services/account-monitor/migrations/001_create_funding_events.sql`

#### Таблица: `funding_events`
**Назначение**: Подтвержденные события поступления FIAT

```sql
CREATE TABLE funding_events (
    id UUID PRIMARY KEY,
    payment_id UUID NOT NULL,
    transaction_id VARCHAR(255) NOT NULL UNIQUE,
    account_id VARCHAR(100) NOT NULL,
    amount DECIMAL(20, 4) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    end_to_end_id VARCHAR(255),
    debtor_name VARCHAR(255),
    debtor_account VARCHAR(100),
    booking_date TIMESTAMP,
    value_date TIMESTAMP,
    confirmed_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

**Назначение**:
- Хранит события "деньги реально зачислены"
- Триггерит Token Engine для минтинга
- Используется для reconciliation

#### Таблица: `unmatched_transactions`
**Назначение**: Транзакции без соответствующего payment

```sql
CREATE TABLE unmatched_transactions (
    id UUID PRIMARY KEY,
    transaction_id VARCHAR(255) NOT NULL UNIQUE,
    account_id VARCHAR(100) NOT NULL,
    amount DECIMAL(20, 4) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    debtor_name VARCHAR(255),
    debtor_account VARCHAR(100),
    booking_date TIMESTAMP,
    value_date TIMESTAMP,
    detected_at TIMESTAMP NOT NULL DEFAULT NOW(),
    status VARCHAR(20) DEFAULT 'UNMATCHED',   -- UNMATCHED, INVESTIGATING, RESOLVED
    resolution_notes TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
```

**Файл**: `services/account-monitor/migrations/002_create_unmatched_transactions.sql`

---

### 7️⃣ Risk Engine

**Файл**: `infrastructure/database/migrations/003-fx-rates-historical.sql`

#### Таблица: `fx_rate_ticks`
**Назначение**: Intraday курсы валют для real-time мониторинга

```sql
CREATE TABLE fx_rate_ticks (
    id BIGSERIAL PRIMARY KEY,
    currency_pair VARCHAR(7) NOT NULL,        -- USD/AED, EUR/USD
    base_currency VARCHAR(3) NOT NULL,
    quote_currency VARCHAR(3) NOT NULL,

    -- Цены (8 знаков после запятой для FX)
    bid_price NUMERIC(26,8) NOT NULL,
    ask_price NUMERIC(26,8) NOT NULL,
    mid_price NUMERIC(26,8) AS ((bid_price + ask_price) / 2),
    spread NUMERIC(26,8) AS (ask_price - bid_price),

    -- Объем и ликвидность
    volume NUMERIC(26,8) DEFAULT 0,
    liquidity_score NUMERIC(5,2),             -- 0-100

    -- Время
    tick_timestamp TIMESTAMPTZ NOT NULL,
    market_session VARCHAR(20),               -- ASIAN, EUROPEAN, AMERICAN

    source VARCHAR(50),                       -- HISTORICAL, LIVE, SIMULATED

    UNIQUE (currency_pair, tick_timestamp)
);
```

#### Таблица: `fx_rate_daily`
**Назначение**: Дневные OHLC (Open, High, Low, Close) данные

```sql
CREATE TABLE fx_rate_daily (
    id BIGSERIAL PRIMARY KEY,
    currency_pair VARCHAR(7) NOT NULL,
    trade_date DATE NOT NULL,

    -- OHLC
    open_price NUMERIC(26,8) NOT NULL,
    high_price NUMERIC(26,8) NOT NULL,
    low_price NUMERIC(26,8) NOT NULL,
    close_price NUMERIC(26,8) NOT NULL,

    daily_volume NUMERIC(26,8) DEFAULT 0,

    -- Статистика
    daily_volatility NUMERIC(10,6),           -- Стандартное отклонение
    daily_return NUMERIC(10,6),               -- % изменение

    -- Moving averages (предрассчитаны)
    sma_7 NUMERIC(26,8),                      -- 7-day SMA
    sma_30 NUMERIC(26,8),                     -- 30-day SMA
    sma_90 NUMERIC(26,8),                     -- 90-day SMA

    UNIQUE (currency_pair, trade_date)
);
```

#### Таблица: `fx_rate_volatility`
**Назначение**: Метрики волатильности для расчета рисков

```sql
CREATE TABLE fx_rate_volatility (
    id BIGSERIAL PRIMARY KEY,
    currency_pair VARCHAR(7) NOT NULL,
    calculation_date DATE NOT NULL,

    -- Волатильность (annualized %)
    volatility_1d NUMERIC(10,6),              -- 1-day
    volatility_7d NUMERIC(10,6),              -- 7-day
    volatility_30d NUMERIC(10,6),             -- 30-day
    volatility_90d NUMERIC(10,6),             -- 90-day
    volatility_365d NUMERIC(10,6),            -- 1-year

    -- VaR (Value at Risk)
    var_95_1d NUMERIC(10,6),                  -- 95% confidence, 1-day
    var_99_1d NUMERIC(10,6),                  -- 99% confidence, 1-day

    -- Stress scenarios
    max_drawdown_30d NUMERIC(10,6),           -- Max падение за 30 дней
    max_surge_30d NUMERIC(10,6),              -- Max рост за 30 дней

    UNIQUE (currency_pair, calculation_date)
);
```

#### Таблица: `fx_currency_pairs`
**Назначение**: Конфигурация валютных пар

```sql
CREATE TABLE fx_currency_pairs (
    id SERIAL PRIMARY KEY,
    currency_pair VARCHAR(7) UNIQUE NOT NULL,
    base_currency VARCHAR(3) NOT NULL,
    quote_currency VARCHAR(3) NOT NULL,

    -- Trading параметры
    is_active BOOLEAN DEFAULT TRUE,
    min_trade_size NUMERIC(26,8),
    max_trade_size NUMERIC(26,8),

    -- Risk параметры
    max_exposure_usd NUMERIC(26,8),           -- Max exposure в USD
    alert_threshold NUMERIC(10,6),            -- % move → alert
    circuit_breaker_threshold NUMERIC(10,6),  -- % move → halt

    -- Характеристики рынка
    typical_spread_bps NUMERIC(10,2),         -- Spread в basis points
    average_daily_volume NUMERIC(26,8),
    market_depth_score NUMERIC(5,2)           -- 0-100
);
```

**Предзагруженные пары**:
- USD/AED, USD/INR, EUR/USD, GBP/USD (major)
- EUR/AED, GBP/AED, EUR/INR, GBP/INR (cross)
- AED/INR, SAR/INR (exotic - DelTran corridors)

**Файл кода**: [services/risk-engine/src/database.rs](services/risk-engine/src/database.rs) (simple pool only)

---

### 8️⃣ Banks Directory

**Файл**: `infrastructure/database/migrations/001-initial-schema.sql`

#### Таблица: `banks`
**Назначение**: Справочник банков-участников

```sql
CREATE TABLE banks (
    id UUID PRIMARY KEY,
    bank_code VARCHAR(20) UNIQUE NOT NULL,
    bank_name VARCHAR(255) NOT NULL,
    swift_bic VARCHAR(11),
    country_code VARCHAR(3) NOT NULL,
    region VARCHAR(50),
    status VARCHAR(20) DEFAULT 'ACTIVE',      -- ACTIVE, SUSPENDED, INACTIVE
    onboarded_at TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

---

## 🔄 ПОТОК ДАННЫХ: От Входа до Хранения

### Пример: Cross-Border Payment (AED → INR)

```
┌──────────────────────────────────────────────────────────────────┐
│ STEP 1: pain.001 Получен                                         │
└──────────────────────────────────────────────────────────────────┘
    ↓
Gateway:
├─ INSERT INTO payments (...) VALUES (...)                         ← PostgreSQL
│  status = 'Received'
│  created_at = NOW()
├─ INSERT INTO payment_events (event_type='RECEIVED', ...)        ← PostgreSQL
└─ PUBLISH to NATS: deltran.compliance.check                       ← NATS (ephemeral)

┌──────────────────────────────────────────────────────────────────┐
│ STEP 2: Compliance Check                                         │
└──────────────────────────────────────────────────────────────────┘
    ↓
Compliance Engine:
├─ SUBSCRIBE NATS: deltran.compliance.check
├─ Run AML/KYC/Sanctions check (in-memory, no DB)
└─ PUBLISH to NATS: deltran.obligation.create

┌──────────────────────────────────────────────────────────────────┐
│ STEP 3: Obligation Created                                       │
└──────────────────────────────────────────────────────────────────┘
    ↓
Obligation Engine:
├─ SUBSCRIBE NATS: deltran.obligation.create
├─ INSERT INTO obligations (...)                                   ← PostgreSQL
│  status = 'PENDING'
│  payer_id = <Bank UAE ID>
│  payee_id = <Bank IN ID>
│  amount = 100000, currency = 'AED'
├─ UPDATE payments SET obligation_id = <obligation.id>            ← PostgreSQL
└─ PUBLISH to NATS: deltran.clearing.submit (cross-border)

┌──────────────────────────────────────────────────────────────────┐
│ STEP 4: Multilateral Netting                                     │
└──────────────────────────────────────────────────────────────────┘
    ↓
Clearing Engine:
├─ SUBSCRIBE NATS: deltran.clearing.submit
├─ SELECT * FROM clearing_windows WHERE status='OPEN'             ← PostgreSQL
├─ SELECT * FROM obligations WHERE window_id=<current>            ← PostgreSQL
├─ Run netting algorithm (in-memory)
├─ INSERT INTO net_positions (...)                                ← PostgreSQL
│  gross_debit_a_to_b = 100000
│  gross_credit_b_to_a = 40000
│  net_amount = 60000 (40% saved!)
├─ UPDATE obligations SET status='NETTED' WHERE id IN (...)       ← PostgreSQL
├─ INSERT INTO settlement_instructions (...)                      ← PostgreSQL
│  amount = 60000 (net position)
└─ PUBLISH to NATS: deltran.liquidity.select

┌──────────────────────────────────────────────────────────────────┐
│ STEP 5: Settlement Execution                                     │
└──────────────────────────────────────────────────────────────────┘
    ↓
Settlement Engine:
├─ SUBSCRIBE NATS: deltran.settlement.execute
├─ SELECT * FROM settlement_instructions WHERE id=<id>            ← PostgreSQL
├─ Generate pacs.008 (ISO 20022)
├─ Send to Bank IL via SWIFT/API
├─ UPDATE settlement_instructions                                 ← PostgreSQL
│  SET status='EXECUTING', sent_to_settlement_at=NOW()
└─ UPDATE obligations SET status='EXECUTING'                      ← PostgreSQL

┌──────────────────────────────────────────────────────────────────┐
│ STEP 6: Bank Confirmation (camt.054 BOOKED)                      │
└──────────────────────────────────────────────────────────────────┘
    ↓
Gateway:
├─ Receive camt.054 from Bank IL
├─ Parse message, extract:
│  - amount: 100000 AED
│  - account: IL-EMI-001
│  - status: BOOKED (final, irreversible)
│  - bank_reference: BNK-2025-001234
│  - end_to_end_id: E2E-UAE-IN-20250120-001
├─ INSERT INTO emi_transactions (...)                             ← PostgreSQL
│  transaction_type = 'funding'
│  direction = 'CREDIT'
│  amount = 100000
│  bank_reference = 'BNK-2025-001234'
│  status = 'CONFIRMED'
├─ UPDATE emi_accounts                                            ← PostgreSQL
│  SET ledger_balance = ledger_balance + 100000
│  WHERE account_id = 'IL-EMI-001'
├─ INSERT INTO funding_events (...)                               ← PostgreSQL
│  payment_id = <payment.id>
│  transaction_id = 'BNK-2025-001234'
│  confirmed_at = NOW()
├─ UPDATE payments                                                ← PostgreSQL
│  SET status='Funded', funded_at=NOW()
│  WHERE end_to_end_id = 'E2E-UAE-IN-20250120-001'
└─ PUBLISH to NATS: deltran.token.mint                            ← NATS

┌──────────────────────────────────────────────────────────────────┐
│ STEP 7: Token Minting (1:1 Backing)                              │
└──────────────────────────────────────────────────────────────────┘
    ↓
Token Engine:
├─ SUBSCRIBE NATS: deltran.token.mint
├─ Validate:
│  ✅ obligation_status == 'SETTLED' (MUST add this!)
│  ✅ bank_reference exists ('BNK-2025-001234')
│  ✅ not duplicate mint
├─ SELECT * FROM emi_accounts WHERE account_id='IL-EMI-001'       ← PostgreSQL
│  Verify: ledger_balance >= 100000 AED
├─ INSERT INTO tokens (...)                                       ← PostgreSQL
│  currency = 'xAED'
│  amount = 100000
│  bank_id = <Bank IL ID>
│  reference = 'BNK-2025-001234' (proof of FIAT backing)
│  status = 'ACTIVE'
│  created_at = NOW()
└─ PUBLISH to NATS: deltran.token.minted
```

---

## 🔒 КРИТИЧЕСКИЕ ГАРАНТИИ

### 1️⃣ Obligation SETTLED = Token Mint Requirement

**Текущее состояние** (❌ НУЖНО ИСПРАВИТЬ):
```rust
// services/gateway-rust/src/main.rs:241
// Gateway напрямую вызывает Token Engine после camt.054
state.router.route_to_token_engine(&payment).await?;
```

**Правильная архитектура** (✅ ТРЕБУЕТСЯ):
```rust
// Settlement Engine должен:
// 1. Получить camt.054
// 2. Закрыть obligation (status='SETTLED')
// 3. Сохранить bank_reference в obligations таблице
// 4. ТОЛЬКО ТОГДА публиковать deltran.token.mint
```

**Необходимые изменения в БД**:
```sql
ALTER TABLE obligations ADD COLUMN settled_at TIMESTAMPTZ;
ALTER TABLE obligations ADD COLUMN bank_confirmation_reference VARCHAR(255);
ALTER TABLE obligations ADD COLUMN camt054_entry_reference VARCHAR(255);
```

### 2️⃣ 1:1 Backing через Reconciliation

**Ежедневная сверка**:
```sql
-- EOD Reconciliation Query
SELECT
    ea.id AS account_id,
    ea.currency,
    ea.ledger_balance AS our_balance,
    ea.bank_reported_balance AS bank_balance,
    ea.ledger_balance - ea.bank_reported_balance AS difference,
    (SELECT SUM(amount) FROM tokens WHERE currency = 'x' || ea.currency AND status='ACTIVE') AS total_tokens_minted
FROM emi_accounts ea
WHERE ea.account_type = 'client_funds'
  AND ABS(ea.ledger_balance - ea.bank_reported_balance) > 0.01;
```

**Если разница найдена**:
```sql
INSERT INTO reconciliation_discrepancies (
    account_id,
    discrepancy_type,
    expected_value,
    actual_value,
    difference,
    status
) VALUES (
    <account_id>,
    'BALANCE_MISMATCH',
    <our_balance>,
    <bank_balance>,
    <difference>,
    'OPEN'
);
```

### 3️⃣ Audit Trail

**Каждая транзакция прослеживается**:
```
payment.deltran_tx_id
  ↓
obligation.transaction_id = payment.deltran_tx_id
  ↓
settlement_instruction.net_position_id → net_positions.id
  ↓
emi_transaction.related_transaction_id = payment.deltran_tx_id
emi_transaction.bank_reference = 'BNK-2025-001234'
  ↓
token.reference = 'BNK-2025-001234'
```

**Query для полной истории**:
```sql
SELECT
    p.deltran_tx_id,
    p.end_to_end_id,
    p.status AS payment_status,
    o.status AS obligation_status,
    o.settled_at,
    et.bank_reference,
    et.confirmed_at AS fiat_confirmed,
    t.id AS token_id,
    t.amount AS token_amount,
    t.created_at AS token_minted
FROM payments p
LEFT JOIN obligations o ON o.transaction_id = p.deltran_tx_id
LEFT JOIN emi_transactions et ON et.related_transaction_id = p.deltran_tx_id
LEFT JOIN tokens t ON t.reference = et.bank_reference
WHERE p.deltran_tx_id = '<UUID>';
```

---

## 📈 МЕТРИКИ И МОНИТОРИНГ

### Database Metrics (Собираются автоматически):

```sql
-- Active connections
SELECT count(*) FROM pg_stat_activity;

-- Top 10 slowest queries
SELECT query, mean_exec_time, calls
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;

-- Table sizes
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

### Business Metrics:

```sql
-- Today's payment volume
SELECT
    currency,
    COUNT(*) AS count,
    SUM(instructed_amount) AS total_volume,
    AVG(instructed_amount) AS avg_amount
FROM payments
WHERE created_at >= CURRENT_DATE
GROUP BY currency;

-- Netting efficiency (last 7 days)
SELECT
    window_name,
    total_gross_value,
    total_net_value,
    netting_efficiency,
    saved_amount
FROM clearing_windows
WHERE created_at >= CURRENT_DATE - INTERVAL '7 days'
ORDER BY created_at DESC;

-- Token supply by currency
SELECT
    currency,
    COUNT(*) AS token_count,
    SUM(amount) AS total_supply
FROM tokens
WHERE status = 'ACTIVE'
GROUP BY currency;

-- EMI account balances
SELECT
    country_code,
    currency,
    account_type,
    SUM(ledger_balance) AS total_balance,
    SUM(reserved_balance) AS total_reserved,
    SUM(available_balance) AS total_available
FROM emi_accounts
GROUP BY country_code, currency, account_type;
```

---

## 🔐 BACKUP И DISASTER RECOVERY

### Рекомендуемая стратегия:

1. **PostgreSQL WAL Archiving**:
   ```
   archive_mode = on
   archive_command = 'cp %p /backup/wal/%f'
   ```

2. **Daily Full Backup**:
   ```bash
   pg_dump -Fc deltran_db > /backup/deltran_$(date +%Y%m%d).dump
   ```

3. **Hourly Incremental Backup**:
   ```bash
   pg_basebackup -D /backup/incremental/$(date +%Y%m%d_%H)
   ```

4. **Critical Tables - Realtime Replication**:
   - `payments`
   - `obligations`
   - `emi_transactions`
   - `tokens`

---

## ✅ ИТОГОВАЯ СХЕМА ХРАНЕНИЯ

| Сервис | Таблицы | Назначение | Критичность |
|--------|---------|------------|-------------|
| **Gateway** | `payments`, `payment_events` | Входящие ISO 20022 сообщения | 🔴 CRITICAL |
| **Obligation** | `obligations` | Обязательства по платежам | 🔴 CRITICAL |
| **Clearing** | `clearing_windows`, `net_positions`, `settlement_instructions` | Multilateral netting | 🟡 HIGH |
| **Token** | `tokens` | Tokenized assets (1:1 backed) | 🔴 CRITICAL |
| **EMI Accounts** | `emi_accounts`, `emi_transactions`, `emi_account_snapshots` | Real FIAT balances | 🔴 CRITICAL |
| **Account Monitor** | `funding_events`, `unmatched_transactions` | FIAT confirmation tracking | 🔴 CRITICAL |
| **Risk** | `fx_rate_ticks`, `fx_rate_daily`, `fx_rate_volatility`, `fx_currency_pairs` | FX risk management | 🟡 HIGH |
| **Directory** | `banks` | Bank participants registry | 🟢 MEDIUM |

---

## 🎯 РЕКОМЕНДАЦИИ

### Немедленно (P0):

1. ✅ **Добавить obligation closing в Settlement Engine**
   - Поля: `settled_at`, `bank_confirmation_reference`
   - Логика: Settlement Engine закрывает obligation при camt.054

2. ✅ **Token Engine validation**
   - Проверять `obligation_status == 'SETTLED'`
   - Проверять `bank_reference` exists

3. ✅ **Reconciliation automation**
   - Ежедневная cверка: EMI accounts vs Tokens
   - Alerts при расхождениях > 0.01%

### В ближайшее время (P1):

4. ✅ **Database monitoring**
   - Prometheus exporter для PostgreSQL
   - Grafana dashboards для business metrics

5. ✅ **Backup automation**
   - Автоматические WAL backups
   - Point-in-time recovery testing

6. ✅ **Audit logging**
   - Все INSERT/UPDATE/DELETE в critical таблицах
   - Trigger-based audit trail

---

**Статус**: ✅ Полная карта хранения данных DelTran
**Database**: PostgreSQL (единая БД, разные таблицы по сервисам)
**Messaging**: NATS (ephemeral, не хранит данные)
**Backup**: Требуется настройка (см. рекомендации)
