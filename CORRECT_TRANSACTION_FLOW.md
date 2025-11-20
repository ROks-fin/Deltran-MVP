# Правильный поток транзакции DelTran
# Correct DelTran Transaction Flow

## 🎯 1:1 Token Backing Guarantee

**Ключевой принцип**: Токены минтятся **ТОЛЬКО** после подтверждения реального FIAT на EMI-счёте.

**Key Principle**: Tokens are minted **ONLY** after real FIAT confirmation on EMI account.

---

## 📊 Международный платёж (International Payment)

```
┌─────────────────────────────────────────────────────────────────────┐
│                     INTERNATIONAL PAYMENT FLOW                      │
└─────────────────────────────────────────────────────────────────────┘

1️⃣  CLIENT
    │
    │ pain.001 (payment initiation)
    ↓
2️⃣  GATEWAY SERVICE
    │
    │ deltran.payment.received
    ↓
3️⃣  COMPLIANCE ENGINE
    │ - AML/KYC check
    │ - Sanctions screening
    │
    │ deltran.compliance.approved
    ↓
4️⃣  OBLIGATION ENGINE
    │ - Create obligation record
    │ - Check cross-border vs local
    │
    │ deltran.obligation.created
    ↓
5️⃣  CLEARING ENGINE (Multilateral Netting)
    │ - Find matching obligations
    │ - Calculate net positions
    │ - 40-60% liquidity savings
    │
    │ deltran.clearing.completed
    ↓
6️⃣  LIQUIDITY ROUTER
    │ - Select payout bank
    │ - Find optimal FX rate
    │
    │ deltran.liquidity.routed
    ↓
7️⃣  RISK ENGINE
    │ - FX volatility assessment
    │ - Exposure limit check
    │ - Recommended action
    │
    │ deltran.risk.assessed
    ↓
8️⃣  SETTLEMENT ENGINE
    │ - Initiate bank transfer (pacs.008)
    │ - Wait for confirmation
    │
    │ ⏳ WAITING FOR REAL FIAT...
    │
    │ camt.054 (bank notification: CREDIT received)
    ↓
9️⃣  ACCOUNT MONITOR ⭐ NEW SERVICE
    │ - Poll bank accounts (every 30s)
    │ - Listen for camt.054 push notifications
    │ - Match transaction with payment
    │   ├─ Primary: by end_to_end_id
    │   └─ Fallback: by amount + currency + time
    │
    │ ✅ FIAT CONFIRMED ON EMI ACCOUNT
    │
    │ deltran.funding.confirmed
    ↓
🔟  TOKEN ENGINE
    │ - Receive funding confirmation
    │ - Validate currency
    │ - Mint tokens (1:1 backing)
    │   USD → xUSD
    │   AED → xAED
    │   ILS → xILS
    │
    │ deltran.token.minted
    ↓
1️⃣1️⃣  NOTIFICATION ENGINE
    │ - Notify client: payment completed
    │
    └─→ ✅ DONE
```

---

## 📊 Локальный платёж (Local Payment)

```
┌─────────────────────────────────────────────────────────────────────┐
│                       LOCAL PAYMENT FLOW                             │
└─────────────────────────────────────────────────────────────────────┘

1️⃣  CLIENT
    │
    │ pain.001 (payment initiation)
    ↓
2️⃣  GATEWAY SERVICE
    │
    │ deltran.payment.received
    ↓
3️⃣  COMPLIANCE ENGINE
    │ - AML/KYC check
    │ - Sanctions screening
    │
    │ deltran.compliance.approved
    ↓
4️⃣  OBLIGATION ENGINE
    │ - Create obligation record
    │ - Detect LOCAL payment (same country)
    │
    │ deltran.obligation.created
    ↓
    │ ⚠️  SKIP Clearing Engine (no netting for local)
    ↓
5️⃣  LIQUIDITY ROUTER
    │ - Select local payout bank
    │ - Same currency (no FX)
    │
    │ deltran.liquidity.routed
    ↓
6️⃣  SETTLEMENT ENGINE
    │ - Initiate local bank transfer
    │ - Wait for confirmation
    │
    │ ⏳ WAITING FOR REAL FIAT...
    │
    │ camt.054 (bank notification: CREDIT received)
    ↓
7️⃣  ACCOUNT MONITOR ⭐ NEW SERVICE
    │ - Poll bank accounts (every 30s)
    │ - Listen for camt.054 push notifications
    │ - Match transaction with payment
    │
    │ ✅ FIAT CONFIRMED ON EMI ACCOUNT
    │
    │ deltran.funding.confirmed
    ↓
8️⃣  TOKEN ENGINE
    │ - Receive funding confirmation
    │ - Mint tokens (1:1 backing)
    │
    │ deltran.token.minted
    ↓
9️⃣  NOTIFICATION ENGINE
    │ - Notify client: payment completed
    │
    └─→ ✅ DONE
```

---

## 🔐 1:1 Backing Guarantee Flow

```
┌──────────────────────────────────────────────────────────────────┐
│             HOW 1:1 BACKING IS GUARANTEED                        │
└──────────────────────────────────────────────────────────────────┘

STEP 1: CLIENT INITIATES PAYMENT
────────────────────────────────
Client sends $100,000 USD payment request
├─ Gateway receives request
├─ Compliance approves
├─ Obligation created
└─ Settlement initiated

                    ⏳ NO TOKENS MINTED YET

STEP 2: BANK TRANSFER IN PROGRESS
──────────────────────────────────
Settlement Engine sends pacs.008 to bank
├─ Transfer initiated: $100,000 USD
└─ Waiting for bank confirmation...

                    ⏳ NO TOKENS MINTED YET

STEP 3: BANK SENDS camt.054 NOTIFICATION
─────────────────────────────────────────
Bank Account: +$100,000 USD CREDIT ✅
├─ camt.054 XML message received
├─ Account Monitor listens on NATS topic
└─ Transaction detected: TXN123456

                    ⏳ NO TOKENS MINTED YET

STEP 4: ACCOUNT MONITOR MATCHES TRANSACTION
────────────────────────────────────────────
Match by end_to_end_id: "E2E987654" ✅
├─ payment_id: uuid-123
├─ amount: $100,000.00
├─ currency: USD
├─ account_id: US12345678901234567890
└─ confirmed_at: 2025-01-19T14:30:00Z

Publish: deltran.funding.confirmed

STEP 5: TOKEN ENGINE MINTS TOKENS
──────────────────────────────────
✅ REAL FIAT CONFIRMED: $100,000 USD on EMI account
├─ Mint: 100,000 xUSD tokens
├─ Link to funding_event_id
├─ Link to payment_id
└─ Publish: deltran.token.minted

                    ✅ 100,000 xUSD = $100,000 USD (1:1)

STEP 6: TOKENS AVAILABLE FOR USE
─────────────────────────────────
Recipient can now:
├─ Use xUSD for payments
├─ Trade xUSD on exchange
├─ Redeem xUSD for real USD
└─ All backed by REAL $100,000 USD in EMI account
```

---

## ❌ Что было неправильно (What Was Wrong)

### До исправления (Before Fix)

```
OBLIGATION ENGINE
│
│ ❌ IMMEDIATE CALL: deltran.token.mint
↓
TOKEN ENGINE
│
└─ ❌ Mints tokens WITHOUT real FIAT confirmation
```

**Проблема**: Токены минтились ДО получения реального FIAT.

**Риск**: Fractional reserve (больше токенов, чем реального FIAT).

**Problem**: Tokens were minted BEFORE real FIAT confirmation.

**Risk**: Fractional reserve (more tokens than real FIAT).

---

## ✅ Что правильно (What Is Correct)

### После исправления (After Fix)

```
SETTLEMENT ENGINE
│
│ Receives camt.054 from bank
↓
ACCOUNT MONITOR
│
│ ✅ Matches transaction with payment
│ ✅ Confirms REAL FIAT on EMI account
│
│ deltran.funding.confirmed
↓
TOKEN ENGINE
│
└─ ✅ Mints tokens ONLY after confirmation (1:1 backing)
```

**Гарантия**: Токены минтятся ТОЛЬКО после подтверждения реального FIAT.

**Результат**: Полная прозрачность 1:1 backing.

**Guarantee**: Tokens are minted ONLY after real FIAT confirmation.

**Result**: Full transparency of 1:1 backing.

---

## 📋 NATS Topics Reference

| Topic | Publisher | Subscriber | Payload |
|-------|-----------|------------|---------|
| `deltran.payment.received` | Gateway | Compliance | PaymentRequest |
| `deltran.compliance.approved` | Compliance | Obligation | ComplianceResult |
| `deltran.obligation.created` | Obligation | Clearing/Liquidity | Obligation |
| `deltran.clearing.completed` | Clearing | Liquidity | NetPosition |
| `deltran.liquidity.routed` | Liquidity | Risk | LiquidityRoute |
| `deltran.risk.assessed` | Risk | Settlement | RiskAssessment |
| `deltran.settlement.initiated` | Settlement | Bank | pacs.008 |
| `deltran.bank.camt054` | Bank | Account Monitor | camt.054 XML |
| **`deltran.funding.confirmed`** ⭐ | **Account Monitor** | **Token Engine** | **FundingEvent** |
| `deltran.token.minted` | Token Engine | Notification | TokenMintedEvent |

---

## 🎯 Key Takeaways

1. **Token Engine is LAST** in the flow (not first!)
2. **Account Monitor is CRITICAL** for 1:1 backing guarantee
3. **camt.054 is the trigger** for token minting
4. **Unmatched transactions** are stored for manual review
5. **Full audit trail** from payment to token minting

---

**Status**: ✅ Architecture Corrected

**Date**: 2025-01-19

**1:1 Backing**: ✅ Guaranteed
