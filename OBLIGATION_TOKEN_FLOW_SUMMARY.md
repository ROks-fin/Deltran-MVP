# DelTran - Правильный Поток: Obligations → Settlement → Tokenization

**Дата**: 2025-01-20
**Статус**: 🎯 **КРИТИЧЕСКАЯ АРХИТЕКТУРА**

---

## 🔐 ТРИ КЛЮЧЕВЫХ ПРИНЦИПА

### 1️⃣ Токенизация ТОЛЬКО после реального FIAT

```
❌ WRONG:
   pain.001 → Obligation → Token (немедленно)
                          ↑
                    БЕЗ подтверждения банка!

✅ CORRECT:
   pain.001 → Obligation (PENDING) → Settlement → camt.054 BOOKED → Token
                                                   ↑
                                          РЕАЛЬНЫЙ FIAT на счету!
```

### 2️⃣ Obligation = "Обещание", НЕ гарантия

```
Obligation создано:
├─ Статус: PENDING
├─ Означает: "Мы ОБЯЗАНЫ заплатить"
├─ НО: Еще НЕ получили FIAT от клиента
└─ ❌ НЕТ токенов (недостаточно гарантий)
```

### 3️⃣ Settlement = Финальный аккорд

```
Settlement Engine:
├─ Отправляет pacs.008 в банк
├─ Получает camt.054 BOOKED
├─ ✅ Закрывает Obligation (SETTLED)
└─ ✅ Триггерит Token Engine (1:1 backing)
```

---

## 📊 ПОЛНЫЙ ПОТОК (Cross-Border Example)

```
┌──────────────────────────────────────────────────────────────────┐
│ ФАЗА 1: ИНИЦИАЦИЯ (Обязательство создано, токенов НЕТ)           │
└──────────────────────────────────────────────────────────────────┘

Client → pain.001 → Gateway
                    │
                    ├─> Compliance (AML/KYC) ✅
                    │
                    └─> Obligation Engine
                        │
                        │  ╔══════════════════════════════════════╗
                        └─>║ Obligation Created                   ║
                           ║ ID: 8f4a2b...                        ║
                           ║ Status: PENDING                      ║
                           ║ Amount: 100,000 AED                  ║
                           ║ ❌ NO TOKENS YET                     ║
                           ╚══════════════════════════════════════╝
                           │
                           ├─> Clearing Engine (multilateral netting)
                           │   └─> Net: 60,000 AED (40% savings)
                           │
                           └─> Liquidity Router
                               └─> Settlement Engine

┌──────────────────────────────────────────────────────────────────┐
│ ФАЗА 2: ИСПОЛНЕНИЕ (Settlement отправляет деньги)                 │
└──────────────────────────────────────────────────────────────────┘

Settlement Engine:
├─> Формирует pacs.008 (ISO 20022)
├─> Отправляет в Bank IL (Beneficiary)
│
│  ╔══════════════════════════════════════╗
└─>║ Obligation Status Update             ║
   ║ PENDING → EXECUTING                  ║
   ║ Settlement ID: 3d7c...               ║
   ║ Bank: Leumi (LUMIILIT)               ║
   ║ ❌ STILL NO TOKENS                   ║
   ╚══════════════════════════════════════╝

Bank IL processes payout...
⏳ Waiting for confirmation...

┌──────────────────────────────────────────────────────────────────┐
│ ФАЗА 3: ПОДТВЕРЖДЕНИЕ (РЕАЛЬНЫЙ FIAT зачислен!)                  │
└──────────────────────────────────────────────────────────────────┘

Bank IL → camt.054 BOOKED
          │
          │  ╔══════════════════════════════════════╗
          └─>║ camt.054 Notification                ║
             ║ Type: CREDIT (money IN)              ║
             ║ Status: BOOKED (final)               ║
             ║ Amount: 100,000 AED                  ║
             ║ Account: IL-EMI-001                  ║
             ║ Reference: BNK-2025-001234           ║
             ║ ✅ REAL FIAT ON ACCOUNT              ║
             ╚══════════════════════════════════════╝
             │
             └─> Gateway → Settlement Engine

Settlement Engine:
│
├─> 1. Close Obligation
│      ╔══════════════════════════════════════╗
│      ║ Obligation Status Update             ║
│      ║ EXECUTING → SETTLED ✅               ║
│      ║ settled_at: 2025-01-20 10:35:42 UTC  ║
│      ║ bank_ref: BNK-2025-001234            ║
│      ╚══════════════════════════════════════╝
│
└─> 2. Trigger Token Mint
       │
       │  ╔══════════════════════════════════════╗
       └─>║ TokenMintRequest                     ║
          ║ obligation_id: 8f4a2b...             ║
          ║ obligation_status: SETTLED ✅        ║
          ║ amount: 100,000                      ║
          ║ currency: AED → xAED token           ║
          ║ bank_reference: BNK-2025-001234      ║
          ║ booked_at: 2025-01-20 10:35:42       ║
          ╚══════════════════════════════════════╝
          │
          └─> Token Engine

Token Engine:
│
├─> Validate:
│   ✅ obligation_status == SETTLED
│   ✅ bank_reference exists
│   ✅ not duplicate mint
│   ✅ FIAT verified on account
│
└─> Mint Token:
    ╔══════════════════════════════════════╗
    ║ Token Created ✅                     ║
    ║ Token ID: 7b3e...                    ║
    ║ Type: xAED                           ║
    ║ Amount: 100,000                      ║
    ║ Backing: 1:1 (BNK-2025-001234)      ║
    ║ Backed by: REAL FIAT on IL-EMI-001  ║
    ║ Cannot be reversed                   ║
    ╚══════════════════════════════════════╝
```

---

## 🔒 OBLIGATION LIFECYCLE

```
┌─────────┐
│ PENDING │  ← Obligation created (payment initiated)
└────┬────┘    ❌ NO tokens yet
     │         ❌ NO guarantees
     ↓
┌──────────┐
│VALIDATED │  ← Passed all checks (Compliance, Risk)
└────┬─────┘    ❌ STILL no tokens
     │
     ↓
┌──────────┐
│ CLEARING │  ← In clearing window (international only)
└────┬─────┘    Multilateral netting in progress
     │
     ↓
┌───────────────┐
│READY_FOR_SETTLE│ ← Liquidity Router selected bank
└──────┬────────┘
       │
       ↓
┌───────────┐
│ EXECUTING │  ← Settlement Engine sent pacs.008 to bank
└─────┬─────┘    Waiting for bank confirmation...
      │
      ↓
   camt.054 BOOKED received ✅
      │
      ↓
┌─────────┐
│ SETTLED │  ← ✅ OBLIGATION CLOSED
└────┬────┘    ✅ Bank confirmed
     │         ✅ FIAT on account
     │         ✅ NOW mint token!
     │
     └────────> Token Engine
```

---

## 💰 TOKEN MINT CONDITIONS

### MUST be TRUE:

```rust
fn can_mint_token(request: &TokenMintRequest) -> bool {
    // 1. Obligation MUST be SETTLED
    request.obligation_status == "SETTLED"

    // 2. Bank reference MUST exist (camt.054 proof)
    && !request.bank_reference.is_empty()

    // 3. Must NOT be duplicate
    && !token_already_minted(request.obligation_id)

    // 4. FIAT MUST be on account
    && fiat_verified_on_account(
        request.account_id,
        request.amount,
        request.currency
    )
}
```

### If ANY condition fails:

```
❌ TokenError::ObligationNotSettled
❌ TokenError::MissingBankConfirmation
❌ TokenError::DuplicateMint
❌ TokenError::FiatNotVerified

→ NO TOKEN MINTED
→ LOG SECURITY ALERT
→ NOTIFY OPS TEAM
```

---

## 🎯 NATS TOPICS FLOW

```
pain.001 received
    ↓
deltran.compliance.check
    ↓
deltran.obligation.create
    │
    ├─ Cross-border? → deltran.clearing.submit
    │                   ↓
    │               deltran.liquidity.select
    │
    └─ Local? → deltran.liquidity.select.local
                    ↓
            deltran.settlement.execute
                    ↓
            pacs.008 sent to bank
                    ↓
            camt.054 BOOKED received
                    ↓
            deltran.bank.camt054 ← Settlement Engine listens
                    ↓
            ┌──────────────────────────┐
            │ Settlement Engine:       │
            │ 1. Close obligation      │
            │ 2. Publish token mint    │
            └──────────────────────────┘
                    ↓
            deltran.token.mint ← Token Engine listens
                    ↓
            ┌──────────────────────────┐
            │ Token Engine:            │
            │ 1. Validate conditions   │
            │ 2. Mint xAED/xUSD token  │
            │ 3. Update ledger         │
            └──────────────────────────┘
                    ↓
            deltran.token.minted
```

---

## 📋 КРИТИЧЕСКИЕ ИЗМЕНЕНИЯ

### Что ЕСТЬ сейчас (НЕПРАВИЛЬНО):

```rust
// services/gateway-rust/src/main.rs:241
// camt.054 handler
state.router.route_to_token_engine(&payment).await?;
    ↑
    ❌ Gateway НАПРЯМУЮ вызывает Token Engine
    ❌ НЕТ проверки obligation status
    ❌ НЕТ закрытия obligation
```

### Что ДОЛЖНО быть (ПРАВИЛЬНО):

```rust
// services/settlement-engine/src/obligation_closer.rs (NEW)
async fn handle_bank_confirmation(camt054: Camt054) -> Result<()> {
    // 1. Find obligation by end_to_end_id
    let obligation = find_obligation(&camt054.end_to_end_id).await?;

    // 2. CRITICAL: Close obligation
    close_obligation(
        obligation.id,
        ObligationStatus::Settled,
        camt054.bank_reference
    ).await?;

    // 3. ONLY NOW: trigger token mint
    publish_token_mint_request(TokenMintRequest {
        obligation_id: obligation.id,
        obligation_status: "SETTLED",  // ← MUST be SETTLED
        bank_reference: camt054.bank_reference,
        amount: camt054.amount,
        currency: camt054.currency,
        booked_at: camt054.booking_date,
    }).await?;

    Ok(())
}
```

---

## 🚀 IMPLEMENTATION CHECKLIST

### Priority P0 (CRITICAL):

- [ ] **Settlement Engine**
  - [ ] Создать `obligation_closer.rs`
  - [ ] Реализовать `handle_bank_confirmation()`
  - [ ] Реализовать `close_obligation()`
  - [ ] Реализовать `publish_token_mint_request()`
  - [ ] Подписаться на `deltran.bank.camt054`

- [ ] **Obligation Engine**
  - [ ] Добавить поле `settled_at` в БД
  - [ ] Добавить `bank_confirmation_reference`
  - [ ] Убрать неиспользуемый `publish_to_token_engine()`

- [ ] **Token Engine**
  - [ ] Добавить validation: `obligation_status == SETTLED`
  - [ ] Проверять `bank_reference` exists
  - [ ] Предотвращать duplicate minting
  - [ ] Обновить TokenMintRequest struct

- [ ] **Gateway**
  - [ ] Изменить camt.054 handler
  - [ ] Публиковать в `deltran.bank.camt054`
  - [ ] Убрать прямой вызов Token Engine

### Priority P1 (Important):

- [ ] Добавить integration tests
- [ ] Обновить документацию
- [ ] Добавить monitoring для obligation closing
- [ ] Добавить alerts для failed settlements

---

## ✅ ОЖИДАЕМЫЕ РЕЗУЛЬТАТЫ

### После Реализации:

```
✅ Tokens created ONLY after real FIAT on account
✅ Obligations properly tracked: PENDING → SETTLED
✅ Settlement Engine = final decision maker
✅ 1:1 backing GUARANTEED (camt.054 BOOKED proof)
✅ Fraud-proof (cannot mint without bank confirmation)
✅ Audit trail complete (obligation → settlement → token)
✅ Regulatory compliant (E-Money License requirements)
```

### Гарантии:

| Гарантия | Механизм |
|----------|----------|
| **1:1 Backing** | Token mint ONLY after camt.054 BOOKED |
| **No Speculation** | Obligation SETTLED before token creation |
| **Fraud Protection** | Bank reference validation |
| **Audit Trail** | Obligation → Settlement → Token chain |
| **Reconciliation** | camt.054 reference in token metadata |
| **Cannot Reverse** | BOOKED status = final confirmation |

---

**Статус**: 🔴 ТРЕБУЕТСЯ РЕАЛИЗАЦИЯ
**Приоритет**: P0 (CRITICAL)
**Estimated**: 8-16 hours
**Risk**: HIGH if not implemented (regulatory non-compliance)
