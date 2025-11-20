# DelTran - Правильный Поток с Обязательствами и Токенизацией

**Дата**: 2025-01-20
**Приоритет**: P0 (КРИТИЧНО для Compliance)

---

## 🎯 КРИТИЧЕСКИЕ ПРИНЦИПЫ

### 1. Токенизация ТОЛЬКО после реального FIAT
```
❌ НЕПРАВИЛЬНО:
   Obligation создано → Token сразу минтится

✅ ПРАВИЛЬНО:
   Obligation создано → Settlement → Bank payout → camt.054 BOOKED → Token минтится
```

### 2. Obligation - это "обещание", НЕ гарантия
```
Obligation = "Мы ОБЯЗАНЫ выплатить, но еще НЕ получили FIAT"
           ↓
         Статус: PENDING (ждем funding)
```

### 3. Settlement - финальный аккорд
```
Settlement = "Закрываем обязательство ПОСЛЕ подтверждения"
           ↓
         Obligation: PENDING → SETTLED
         Token: MINT (1:1 backing)
```

---

## 📊 ПРАВИЛЬНАЯ АРХИТЕКТУРА

### Международный Поток (Cross-Border):

```
┌─────────────────────────────────────────────────────────────────┐
│ ЭТАП 1: ИНИЦИАЦИЯ (без токенов, без гарантий)                   │
└─────────────────────────────────────────────────────────────────┘

1. Gateway (pain.001)
   └─> Compliance Engine (AML/KYC)
       └─> Obligation Engine

           ┌──────────────────────────────────────────────────┐
           │ Obligation = "Обещание выплатить"               │
           │ Статус: PENDING                                  │
           │ ❌ НЕТ токенов (FIAT еще не поступил)            │
           │ ❌ НЕТ гарантии (может быть отменено)            │
           └──────────────────────────────────────────────────┘

           └─> Clearing Engine (multilateral netting)
               └─> Liquidity Router (выбор банка)
                   └─> Settlement Engine

┌─────────────────────────────────────────────────────────────────┐
│ ЭТАП 2: ИСПОЛНЕНИЕ (Settlement отправляет payout)                │
└─────────────────────────────────────────────────────────────────┘

2. Settlement Engine
   ├─> Формирует pacs.008 (ISO 20022)
   ├─> Отправляет в банк получателя
   └─> Ждет подтверждения...

       Obligation Статус: PENDING → EXECUTING

┌─────────────────────────────────────────────────────────────────┐
│ ЭТАП 3: ПОДТВЕРЖДЕНИЕ БАНКОМ (Реальный FIAT зачислен!)          │
└─────────────────────────────────────────────────────────────────┘

3. Bank → camt.054 BOOKED
   └─> Gateway получает
       └─> Проверяет: CREDIT + BOOKED

           ┌──────────────────────────────────────────────────┐
           │ ✅ РЕАЛЬНЫЙ FIAT ЗАЧИСЛЕН НА СЧЕТ                │
           │ ✅ Bank confirmed (не может быть отменено)       │
           │ ✅ ГОТОВО для токенизации                        │
           └──────────────────────────────────────────────────┘

           ├─> Settlement Engine → Close Obligation
           │
           │   ┌──────────────────────────────────────────────┐
           │   │ Obligation: EXECUTING → SETTLED              │
           │   │ ✅ Обязательство ВЫПОЛНЕНО                   │
           │   └──────────────────────────────────────────────┘
           │
           └─> Token Engine → Mint Token

               ┌──────────────────────────────────────────────┐
               │ Token создан                                 │
               │ Backing: 1:1 (привязан к camt.054 BOOKED)   │
               │ ✅ Полностью обеспечен FIAT                  │
               └──────────────────────────────────────────────┘
```

### Локальный Поток (Same Country):

```
┌─────────────────────────────────────────────────────────────────┐
│ ЭТАП 1: ИНИЦИАЦИЯ                                                │
└─────────────────────────────────────────────────────────────────┘

1. Gateway → Compliance → Obligation Engine

   Obligation: PENDING
   ├─> ❌ Минует Clearing Engine (локальный платеж)
   └─> Liquidity Router (выбор локального банка)
       └─> Settlement Engine

┌─────────────────────────────────────────────────────────────────┐
│ ЭТАП 2: ИСПОЛНЕНИЕ                                               │
└─────────────────────────────────────────────────────────────────┘

2. Settlement Engine → Local payout
   Obligation: PENDING → EXECUTING

┌─────────────────────────────────────────────────────────────────┐
│ ЭТАП 3: ПОДТВЕРЖДЕНИЕ                                            │
└─────────────────────────────────────────────────────────────────┘

3. Bank → camt.054 BOOKED
   ├─> Settlement Engine → Close Obligation (SETTLED)
   └─> Token Engine → Mint Token (1:1 backing)
```

---

## 🔒 СТАТУСЫ OBLIGATION

### ObligationStatus Enum:

```rust
pub enum ObligationStatus {
    // Начальные состояния
    Pending,        // Создано, ждем funding от клиента
    Validated,      // Прошло все проверки

    // В процессе
    Clearing,       // В клиринговом окне (только международные)
    ReadyForSettle, // Готово к settlement
    Executing,      // Settlement отправил payout в банк

    // Финальные состояния
    Settled,        // ✅ Банк подтвердил (camt.054 BOOKED), obligation закрыто
    Failed,         // ❌ Не удалось выполнить
    Cancelled,      // Отменено
    Expired,        // Истекло время
}
```

### Переходы статусов:

```
Pending → Validated → Clearing → ReadyForSettle → Executing → Settled ✅
                                                             ↓
                                                          Failed ❌
```

---

## 💰 ТОКЕНИЗАЦИЯ - ТОЛЬКО ПОСЛЕ SETTLED

### Правило:

```rust
// Token Engine слушает события ТОЛЬКО от Settlement Engine
// после успешного закрытия obligation

if obligation_status == ObligationStatus::Settled {
    // ✅ Obligation выполнено
    // ✅ camt.054 BOOKED получен
    // ✅ Реальный FIAT на счету

    mint_token(payment, camt054_reference);
}
```

### Гарантии:

| Проверка | Статус |
|----------|--------|
| Obligation.status = SETTLED | ✅ |
| camt.054 BOOKED received | ✅ |
| Real FIAT on EMI account | ✅ |
| Bank confirmation reference | ✅ |
| Cannot be reversed | ✅ |

---

## 🎯 SETTLEMENT ENGINE - Финальный Аккорд

### Функции Settlement Engine:

#### 1. Исполнение Payout

```rust
async fn execute_settlement(instruction: SettlementInstruction) -> Result<()> {
    // 1. Формируем pacs.008 (ISO 20022)
    let pacs008 = build_pacs008_message(&instruction)?;

    // 2. Отправляем в банк
    send_to_bank(&pacs008).await?;

    // 3. Обновляем статус obligation
    update_obligation_status(
        instruction.obligation_id,
        ObligationStatus::Executing
    ).await?;

    Ok(())
}
```

#### 2. Обработка Подтверждения от Банка

```rust
async fn handle_bank_confirmation(camt054: Camt054Notification) -> Result<()> {
    // 1. Проверяем: CREDIT + BOOKED
    if !is_credit_event(&camt054) || !is_booked(&camt054) {
        return Ok(()); // Skip DEBIT or PENDING
    }

    // 2. Находим obligation по end_to_end_id
    let obligation = find_obligation_by_e2e(&camt054.end_to_end_id).await?;

    // 3. КРИТИЧНО: Закрываем obligation
    close_obligation(
        obligation.id,
        ObligationStatus::Settled,
        camt054.bank_reference
    ).await?;

    info!("✅ Obligation {} SETTLED - банк подтвердил зачисление", obligation.id);

    // 4. ТОЛЬКО ТЕПЕРЬ отправляем в Token Engine
    publish_to_token_engine(TokenMintRequest {
        payment_id: obligation.payment_id,
        amount: camt054.amount,
        currency: camt054.currency,
        obligation_id: obligation.id,
        bank_reference: camt054.bank_reference,
        booked_at: camt054.booking_date,
    }).await?;

    Ok(())
}
```

#### 3. Закрытие Obligation

```rust
async fn close_obligation(
    obligation_id: Uuid,
    final_status: ObligationStatus,
    bank_reference: String,
) -> Result<()> {
    sqlx::query!(
        r#"
        UPDATE obligations
        SET
            status = $1,
            settled_at = NOW(),
            bank_confirmation_reference = $2,
            updated_at = NOW()
        WHERE obligation_id = $3
        "#,
        final_status.to_string(),
        bank_reference,
        obligation_id
    )
    .execute(&db)
    .await?;

    info!("🔒 Obligation {} закрыто со статусом: {:?}", obligation_id, final_status);

    // Publish event для analytics
    publish_obligation_closed_event(obligation_id, final_status).await?;

    Ok(())
}
```

---

## 📋 NATS TOPICS - Обновленный Flow

### События:

| Topic | Publisher | Subscriber | Trigger | Payload |
|-------|-----------|------------|---------|---------|
| `deltran.obligation.create` | Gateway, Compliance | Obligation Engine | pain.001 received | CanonicalPayment |
| `deltran.obligation.validated` | Obligation Engine | Clearing/Liquidity | Obligation VALIDATED | Obligation |
| `deltran.clearing.submit` | Obligation Engine | Clearing Engine | International payment | Payment + Obligation |
| `deltran.liquidity.select` | Clearing Engine | Liquidity Router | After netting | Net Positions |
| `deltran.liquidity.select.local` | Obligation Engine | Liquidity Router | Local payment | Payment + Obligation |
| `deltran.settlement.execute` | Liquidity Router | Settlement Engine | Bank selected | Settlement Instruction |
| `deltran.settlement.executed` | Settlement Engine | Analytics | pacs.008 sent | Settlement ID |
| `deltran.bank.camt054` | Gateway | Settlement Engine | Bank confirmation | camt.054 BOOKED |
| **`deltran.obligation.settled`** | **Settlement Engine** | **Analytics, Reporting** | **Obligation closed** | **Obligation + camt.054 ref** |
| **`deltran.token.mint`** | **Settlement Engine** | **Token Engine** | **AFTER obligation settled** | **TokenMintRequest** |
| `deltran.token.minted` | Token Engine | Analytics | Token created | Token + Obligation ID |

---

## 🔐 DATABASE SCHEMA - Obligation Table

```sql
CREATE TABLE obligations (
    obligation_id UUID PRIMARY KEY,
    payment_id UUID NOT NULL,
    deltran_tx_id UUID NOT NULL,

    -- Amounts
    amount DECIMAL(20, 4) NOT NULL,
    currency VARCHAR(3) NOT NULL,

    -- Parties
    debtor_country VARCHAR(2) NOT NULL,
    creditor_country VARCHAR(2) NOT NULL,

    -- Status Tracking
    status VARCHAR(50) NOT NULL DEFAULT 'PENDING',
    -- PENDING, VALIDATED, CLEARING, READY_FOR_SETTLE, EXECUTING, SETTLED, FAILED

    -- Settlement Info
    settlement_id UUID,
    settlement_instruction_id UUID,

    -- Bank Confirmation
    bank_confirmation_reference VARCHAR(255),
    camt054_entry_reference VARCHAR(255),

    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    validated_at TIMESTAMP,
    clearing_started_at TIMESTAMP,
    settlement_started_at TIMESTAMP,
    settled_at TIMESTAMP,  -- ✅ КРИТИЧНО: когда obligation закрыто
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),

    -- Indexes
    INDEX idx_obligation_payment (payment_id),
    INDEX idx_obligation_status (status),
    INDEX idx_obligation_settled (settled_at)
);

-- Комментарии
COMMENT ON COLUMN obligations.status IS 'Статус обязательства: PENDING → SETTLED';
COMMENT ON COLUMN obligations.settled_at IS 'КРИТИЧНО: Время подтверждения банком (camt.054 BOOKED)';
COMMENT ON COLUMN obligations.bank_confirmation_reference IS 'Ссылка на bank statement entry';
```

---

## 🎯 TOKEN MINT REQUEST

### Структура:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenMintRequest {
    // Payment Info
    pub payment_id: Uuid,
    pub deltran_tx_id: Uuid,
    pub end_to_end_id: String,

    // Obligation Info (КРИТИЧНО!)
    pub obligation_id: Uuid,
    pub obligation_status: String,  // Must be "SETTLED"
    pub obligation_settled_at: DateTime<Utc>,

    // Amount
    pub amount: Decimal,
    pub currency: String,  // Will create xUSD, xAED, etc.

    // Bank Confirmation (1:1 Backing Proof!)
    pub bank_reference: String,          // camt.054 reference
    pub bank_statement_entry: String,    // Entry ID from bank
    pub booked_at: DateTime<Utc>,       // Bank booking timestamp

    // Audit Trail
    pub camt054_message_id: String,
    pub account_id: String,              // Which EMI account received FIAT
}
```

### Validation в Token Engine:

```rust
async fn validate_mint_request(request: &TokenMintRequest) -> Result<()> {
    // 1. Проверяем, что obligation SETTLED
    if request.obligation_status != "SETTLED" {
        return Err(TokenError::ObligationNotSettled(
            format!("Cannot mint token - obligation {} status is {}, expected SETTLED",
                    request.obligation_id, request.obligation_status)
        ));
    }

    // 2. Проверяем наличие bank reference
    if request.bank_reference.is_empty() {
        return Err(TokenError::MissingBankConfirmation);
    }

    // 3. Проверяем, что не пытаемся дважды сминтить токен
    if token_already_minted_for_obligation(request.obligation_id).await? {
        return Err(TokenError::DuplicateMint(request.obligation_id));
    }

    // 4. Проверяем, что FIAT действительно на счету (reconciliation)
    verify_fiat_on_account(
        request.account_id,
        request.amount,
        request.currency,
        request.booked_at
    ).await?;

    Ok(())
}
```

---

## 🚀 IMPLEMENTATION PLAN

### Файлы для Изменения:

#### 1. Settlement Engine (`services/settlement-engine/src/`)

**Новые функции**:
- ✅ `handle_bank_confirmation()` - обработка camt.054
- ✅ `close_obligation()` - закрытие obligation в БД
- ✅ `publish_obligation_settled()` - событие закрытия
- ✅ `publish_token_mint_request()` - запрос на минтинг

**Файлы**:
```
src/
├─ nats_consumer.rs        (подписка на deltran.bank.camt054)
├─ obligation_closer.rs    (NEW - логика закрытия obligations)
├─ token_mint_publisher.rs (NEW - публикация в Token Engine)
└─ database.rs             (queries для update obligation)
```

#### 2. Obligation Engine (`services/obligation-engine/src/`)

**Изменения**:
- ✅ Добавить поле `settled_at` в ObligationCreatedEvent
- ✅ НЕ вызывать Token Engine (удалить неиспользуемый код)
- ✅ Публиковать статус: PENDING, VALIDATED, EXECUTING

**Убрать**:
```rust
// ❌ УДАЛИТЬ эту функцию (не используется)
async fn publish_to_token_engine(nats_client: &Client, payment: &CanonicalPayment)
```

#### 3. Token Engine (`services/token-engine/src/`)

**Изменения**:
- ✅ Слушать `deltran.token.mint` (уже делает)
- ✅ Добавить validation: obligation_status == SETTLED
- ✅ Проверять bank_reference
- ✅ Предотвращать duplicate minting

#### 4. Gateway (`services/gateway-rust/src/`)

**Изменения**:
- ✅ camt.054 handler должен публиковать в `deltran.bank.camt054`
- ✅ Settlement Engine будет слушать и закрывать obligation
- ✅ Settlement Engine потом вызовет Token Engine

---

## 📊 SEQUENCE DIAGRAM

```
┌─────────┐  ┌────────────┐  ┌──────────┐  ┌─────────┐  ┌──────────┐  ┌──────┐  ┌────────┐
│ Gateway │  │ Compliance │  │ Obligat. │  │ Clearing│  │ Liquidity│  │Settle│  │ Token  │
└────┬────┘  └─────┬──────┘  └────┬─────┘  └────┬────┘  └────┬─────┘  └──┬───┘  └───┬────┘
     │             │              │              │             │           │          │
     │ pain.001    │              │              │             │           │          │
     ├────────────>│              │              │             │           │          │
     │             │ AML/KYC      │              │             │           │          │
     │             ├─────────────>│              │             │           │          │
     │             │              │ Create       │             │           │          │
     │             │              │ Obligation   │             │           │          │
     │             │              │ Status:PENDING│            │           │          │
     │             │              │              │             │           │          │
     │             │              ├─────────────>│ Netting     │           │          │
     │             │              │              ├────────────>│ Select    │          │
     │             │              │              │             │ Bank      │          │
     │             │              │              │             ├──────────>│ Execute  │
     │             │              │              │             │           │ pacs.008 │
     │             │              │              │             │           ├────>BANK │
     │             │              │              │             │           │          │
     │             │              │ Status: EXECUTING          │           │          │
     │             │              │<────────────────────────────────────────          │
     │             │              │              │             │           │          │
     │  camt.054   │              │              │             │           │          │
     │  BOOKED     │              │              │             │           │          │
     │<────────────────────────────────────────────────────────────────────          │
     │             │              │              │             │           │          │
     │ deltran.bank.camt054       │              │             │           │          │
     ├──────────────────────────────────────────────────────────────────>│          │
     │             │              │              │             │           │          │
     │             │              │              │             │   Close   │          │
     │             │              │              │             │ Obligation│          │
     │             │              │<──────────────────────────────────────│          │
     │             │              │ Status: SETTLED            │           │          │
     │             │              │              │             │           │          │
     │             │              │              │             │    deltran.token.mint│
     │             │              │              │             │           ├─────────>│
     │             │              │              │             │           │   Mint   │
     │             │              │              │             │           │   Token  │
     │             │              │              │             │           │   (1:1)  │
```

---

## ✅ ИТОГОВЫЕ ГАРАНТИИ

### 1. Токенизация ТОЛЬКО после реального FIAT

```
Token создается ТОЛЬКО если:
├─ ✅ Obligation.status == SETTLED
├─ ✅ camt.054 BOOKED received
├─ ✅ Bank confirmation reference exists
└─ ✅ FIAT verified on EMI account
```

### 2. Obligation - обещание с отслеживанием

```
Obligation lifecycle:
PENDING → VALIDATED → CLEARING → EXECUTING → SETTLED ✅
                                            ↓
                                          Token Mint
```

### 3. Settlement - финальный аккорд

```
Settlement Engine closes obligation ONLY after:
├─ ✅ pacs.008 sent to bank
├─ ✅ camt.054 BOOKED received
├─ ✅ Bank confirmed money transfer
└─ ✅ THEN: close obligation + trigger token mint
```

---

**Статус**: 🔴 ТРЕБУЕТСЯ РЕАЛИЗАЦИЯ
**Приоритет**: P0 (CRITICAL для compliance)
**Следующий шаг**: Реализовать obligation closing в Settlement Engine
