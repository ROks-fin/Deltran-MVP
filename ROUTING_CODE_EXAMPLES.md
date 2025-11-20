# Примеры кода маршрутизации платежей
# Payment Routing Code Examples

## 📋 Полный пример разделения потоков (Complete Routing Example)

### Файл: services/obligation-engine/src/nats_consumer.rs

---

## 1️⃣ Основная функция обработки (Main Processing Function)

```rust
// Строки 65-120

tokio::spawn(async move {
    info!("🔄 Obligation consumer task started");

    while let Some(msg) = subscriber.next().await {
        // Parse CanonicalPayment from message
        match serde_json::from_slice::<CanonicalPayment>(&msg.payload) {
            Ok(payment) => {
                info!("📋 Received obligation creation request for: {} (E2E: {}, UETR: {:?})",
                      payment.deltran_tx_id, payment.end_to_end_id, payment.uetr);

                // ┌─────────────────────────────────────────────┐
                // │  STEP 1: CREATE OBLIGATION                  │
                // └─────────────────────────────────────────────┘
                match create_obligation(&payment).await {
                    Ok(obligation) => {
                        info!("✅ Obligation created: {} for payment {}",
                              obligation.obligation_id, payment.deltran_tx_id);

                        // ┌─────────────────────────────────────────────┐
                        // │  STEP 2: ROUTING DECISION                   │
                        // │  🔍 CHECK IF CROSS-BORDER                   │
                        // └─────────────────────────────────────────────┘

                        if is_cross_border(&payment) {
                            // ═══════════════════════════════════════════
                            //  INTERNATIONAL PAYMENT ROUTE
                            // ═══════════════════════════════════════════
                            info!("🌍 Cross-border payment - routing to Clearing Engine");

                            if let Err(e) = publish_to_clearing(&nats_for_publish, &payment, &obligation).await {
                                error!("Failed to route to Clearing Engine: {}", e);
                            }

                            // Next: Clearing → Liquidity → Risk → Settlement

                        } else {
                            // ═══════════════════════════════════════════
                            //  LOCAL PAYMENT ROUTE
                            // ═══════════════════════════════════════════
                            info!("🏠 Local payment - routing to Liquidity Router");

                            if let Err(e) = publish_to_liquidity_router(&nats_for_publish, &payment, &obligation).await {
                                error!("Failed to route to Liquidity Router: {}", e);
                            }

                            // Next: Liquidity → Settlement (SKIP Clearing & Risk)
                        }

                        // Analytics event
                        if let Err(e) = publish_obligation_created(&nats_for_publish, &obligation).await {
                            error!("Failed to publish obligation created event: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("❌ Failed to create obligation for payment {}: {}",
                               payment.deltran_tx_id, e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to parse CanonicalPayment from NATS message: {}", e);
            }
        }
    }
});
```

---

## 2️⃣ Функция определения cross-border (Cross-border Detection)

```rust
// Строки 149-155

fn is_cross_border(payment: &CanonicalPayment) -> bool {
    // ┌─────────────────────────────────────────────────────────┐
    // │  АЛГОРИТМ:                                              │
    // │  1. Извлечь country code из debtor BIC                  │
    // │  2. Извлечь country code из creditor BIC                │
    // │  3. Сравнить: разные страны = international             │
    // └─────────────────────────────────────────────────────────┘

    let debtor_country = extract_country_from_bic(&payment.debtor_agent.bic);
    let creditor_country = extract_country_from_bic(&payment.creditor_agent.bic);

    // Пример:
    // debtor_country = "AE" (UAE)
    // creditor_country = "IL" (Israel)
    // Result: "AE" ≠ "IL" → TRUE (cross-border)

    debtor_country != creditor_country
}
```

**Логика**:
- ✅ `TRUE` → Разные страны → International route
- ❌ `FALSE` → Одна страна → Local route

---

## 3️⃣ Извлечение кода страны из BIC (BIC Country Extraction)

```rust
// Строки 157-169

fn extract_country_from_bic(bic: &str) -> String {
    // ┌─────────────────────────────────────────────────────────┐
    // │  BIC FORMAT: XXXXYYZZAAA                                │
    // │                                                         │
    // │  XXXX = Bank code (4 chars)                            │
    // │  YY   = Country code (2 chars) ← ИЗВЛЕКАЕМ ЭТО         │
    // │  ZZ   = Location code (2 chars)                        │
    // │  AAA  = Branch code (3 chars, optional)                │
    // └─────────────────────────────────────────────────────────┘

    // Пример BIC: "EBILAEAD001"
    //              ^^^^--^^
    //              Emir  AE
    //              Bank  ↑
    //                    Country code

    if bic.len() >= 6 {
        // Позиции 4-5 (0-indexed) = символы 5-6
        bic[4..6].to_uppercase()  // → "AE"
    } else {
        "XX".to_string() // Invalid BIC
    }
}
```

**Примеры**:

| BIC | Bank Code | Country | Result |
|-----|-----------|---------|--------|
| `EBILAEAD001` | EBIL | **AE** | `"AE"` |
| `FIRBILITXXX` | FIRB | **IL** | `"IL"` |
| `CITITRISXXX` | CITI | **TR** | `"TR"` |
| `NBADAEADXXX` | NBAD | **AE** | `"AE"` |
| `LUMIILIT123` | LUMI | **IL** | `"IL"` |

---

## 4️⃣ Публикация в Clearing Engine (International Route)

```rust
// Строки 171-188

async fn publish_to_clearing(
    nats_client: &Client,
    payment: &CanonicalPayment,
    obligation: &ObligationCreatedEvent
) -> anyhow::Result<()> {
    // ┌─────────────────────────────────────────────────────────┐
    // │  INTERNATIONAL PAYMENT ROUTE                            │
    // │  Subject: deltran.clearing.submit                       │
    // └─────────────────────────────────────────────────────────┘

    let subject = "deltran.clearing.submit";

    // Create clearing submission with obligation info
    let clearing_data = serde_json::json!({
        "payment": payment,
        "obligation": obligation,
    });

    let payload = serde_json::to_vec(&clearing_data)?;

    // Publish to NATS
    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Routed to Clearing Engine: {} (obligation: {})",
          payment.deltran_tx_id, obligation.obligation_id);

    // ┌─────────────────────────────────────────────────────────┐
    // │  NEXT STEPS (handled by Clearing Engine):              │
    // │  1. Find matching opposite obligations                  │
    // │  2. Calculate net positions                             │
    // │  3. Publish to deltran.clearing.completed               │
    // │  4. → Liquidity Router receives net positions           │
    // └─────────────────────────────────────────────────────────┘

    Ok(())
}
```

**Payload Example**:
```json
{
  "payment": {
    "deltran_tx_id": "uuid-123",
    "end_to_end_id": "E2E123456",
    "settlement_amount": "100000.00",
    "currency": "USD",
    "debtor_agent": {
      "bic": "EBILAEAD001",
      "country": "AE"
    },
    "creditor_agent": {
      "bic": "FIRBILITXXX",
      "country": "IL"
    }
  },
  "obligation": {
    "obligation_id": "uuid-456",
    "debtor_country": "AE",
    "creditor_country": "IL"
  }
}
```

---

## 5️⃣ Публикация в Liquidity Router (Local Route)

```rust
// Строки 201-221

async fn publish_to_liquidity_router(
    nats_client: &Client,
    payment: &CanonicalPayment,
    obligation: &ObligationCreatedEvent
) -> anyhow::Result<()> {
    // ┌─────────────────────────────────────────────────────────┐
    // │  LOCAL PAYMENT ROUTE                                    │
    // │  Subject: deltran.liquidity.select.local                │
    // └─────────────────────────────────────────────────────────┘

    let subject = "deltran.liquidity.select.local";

    // For local payments, Liquidity Router selects optimal local payout bank
    let liquidity_request = serde_json::json!({
        "payment": payment,
        "obligation": obligation,
        "payment_type": "LOCAL",  // ← Маркер локального платежа
        "jurisdiction": extract_country_from_bic(&payment.creditor_agent.bic),
    });

    let payload = serde_json::to_vec(&liquidity_request)?;

    // Publish to NATS
    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Routed to Liquidity Router (local): {} in {}",
          payment.deltran_tx_id,
          extract_country_from_bic(&payment.creditor_agent.bic));

    // ┌─────────────────────────────────────────────────────────┐
    // │  NEXT STEPS (handled by Liquidity Router):             │
    // │  1. Select local payout bank in same country            │
    // │  2. No FX conversion needed (same currency)             │
    // │  3. Publish to deltran.liquidity.routed                 │
    // │  4. → Settlement Engine receives routing                │
    // └─────────────────────────────────────────────────────────┘

    Ok(())
}
```

**Payload Example**:
```json
{
  "payment": {
    "deltran_tx_id": "uuid-789",
    "end_to_end_id": "E2E789012",
    "settlement_amount": "50000.00",
    "currency": "AED",
    "debtor_agent": {
      "bic": "EBILAEAD001",
      "country": "AE"
    },
    "creditor_agent": {
      "bic": "NBADAEADXXX",
      "country": "AE"
    }
  },
  "obligation": {
    "obligation_id": "uuid-101",
    "debtor_country": "AE",
    "creditor_country": "AE"
  },
  "payment_type": "LOCAL",
  "jurisdiction": "AE"
}
```

---

## 📊 Сравнение путей (Path Comparison)

### International Payment Flow

```rust
// Пример: UAE → Israel ($100,000)

let payment = CanonicalPayment {
    deltran_tx_id: Uuid::new_v4(),
    settlement_amount: Decimal::from(100000),
    currency: "USD".to_string(),
    debtor_agent: FinancialInstitution {
        bic: "EBILAEAD001".to_string(),  // UAE bank
        country: Some("AE".to_string()),
    },
    creditor_agent: FinancialInstitution {
        bic: "FIRBILITXXX".to_string(),  // Israel bank
        country: Some("IL".to_string()),
    },
    // ... other fields
};

// ✅ Routing decision:
// debtor_country = "AE"
// creditor_country = "IL"
// is_cross_border() = TRUE

// → publish_to_clearing()
// → Subject: "deltran.clearing.submit"
// → Flow: Clearing → Liquidity → Risk → Settlement
```

### Local Payment Flow

```rust
// Пример: UAE → UAE (AED 50,000)

let payment = CanonicalPayment {
    deltran_tx_id: Uuid::new_v4(),
    settlement_amount: Decimal::from(50000),
    currency: "AED".to_string(),
    debtor_agent: FinancialInstitution {
        bic: "EBILAEAD001".to_string(),  // Emirates Islamic Bank
        country: Some("AE".to_string()),
    },
    creditor_agent: FinancialInstitution {
        bic: "NBADAEADXXX".to_string(),  // National Bank of Abu Dhabi
        country: Some("AE".to_string()),
    },
    // ... other fields
};

// ✅ Routing decision:
// debtor_country = "AE"
// creditor_country = "AE"
// is_cross_border() = FALSE

// → publish_to_liquidity_router()
// → Subject: "deltran.liquidity.select.local"
// → Flow: Liquidity → Settlement (SKIP Clearing & Risk)
```

---

## 🧪 Unit Tests (Примеры тестов)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_country_from_bic() {
        assert_eq!(extract_country_from_bic("EBILAEAD001"), "AE");
        assert_eq!(extract_country_from_bic("FIRBILITXXX"), "IL");
        assert_eq!(extract_country_from_bic("CITITRISXXX"), "TR");
        assert_eq!(extract_country_from_bic("NBADAEADXXX"), "AE");
        assert_eq!(extract_country_from_bic("LUMIILIT123"), "IL");
    }

    #[test]
    fn test_is_cross_border_international() {
        let payment = CanonicalPayment {
            debtor_agent: FinancialInstitution {
                bic: "EBILAEAD001".to_string(),
                // ... other fields
            },
            creditor_agent: FinancialInstitution {
                bic: "FIRBILITXXX".to_string(),
                // ... other fields
            },
            // ... other fields
        };

        assert_eq!(is_cross_border(&payment), true);  // AE → IL
    }

    #[test]
    fn test_is_cross_border_local() {
        let payment = CanonicalPayment {
            debtor_agent: FinancialInstitution {
                bic: "EBILAEAD001".to_string(),
                // ... other fields
            },
            creditor_agent: FinancialInstitution {
                bic: "NBADAEADXXX".to_string(),
                // ... other fields
            },
            // ... other fields
        };

        assert_eq!(is_cross_border(&payment), false);  // AE → AE
    }
}
```

---

## 🔧 Конфигурация NATS Topics (NATS Topics Configuration)

### International Payment Topics

```yaml
# Obligation Engine → Clearing Engine
topic: deltran.clearing.submit
payload_type: ClearingSubmission
subscribers:
  - clearing-engine

# Clearing Engine → Liquidity Router
topic: deltran.clearing.completed
payload_type: NetPosition
subscribers:
  - liquidity-router

# Liquidity Router → Risk Engine
topic: deltran.liquidity.routed
payload_type: LiquidityRoute
subscribers:
  - risk-engine

# Risk Engine → Settlement Engine
topic: deltran.risk.assessed
payload_type: RiskAssessment
subscribers:
  - settlement-engine
```

### Local Payment Topics

```yaml
# Obligation Engine → Liquidity Router
topic: deltran.liquidity.select.local
payload_type: LocalLiquidityRequest
subscribers:
  - liquidity-router

# Liquidity Router → Settlement Engine
topic: deltran.liquidity.routed
payload_type: LiquidityRoute
subscribers:
  - settlement-engine

# Note: SKIP Clearing Engine and Risk Engine
```

---

## 📈 Метрики и мониторинг (Metrics & Monitoring)

```rust
// Prometheus metrics для отслеживания routing decisions

use prometheus::{IntCounter, register_int_counter};

lazy_static! {
    static ref PAYMENTS_INTERNATIONAL: IntCounter = register_int_counter!(
        "deltran_payments_international_total",
        "Total number of international payments routed to Clearing Engine"
    ).unwrap();

    static ref PAYMENTS_LOCAL: IntCounter = register_int_counter!(
        "deltran_payments_local_total",
        "Total number of local payments routed to Liquidity Router"
    ).unwrap();
}

// В коде:
if is_cross_border(&payment) {
    PAYMENTS_INTERNATIONAL.inc();  // Increment counter
    publish_to_clearing(&nats_for_publish, &payment, &obligation).await?;
} else {
    PAYMENTS_LOCAL.inc();  // Increment counter
    publish_to_liquidity_router(&nats_for_publish, &payment, &obligation).await?;
}
```

---

## ✅ Резюме кода (Code Summary)

| Функция | Строки | Назначение |
|---------|--------|-----------|
| `is_cross_border()` | 149-155 | Определяет international vs local |
| `extract_country_from_bic()` | 157-169 | Извлекает код страны из BIC |
| `publish_to_clearing()` | 171-188 | Route для international payments |
| `publish_to_liquidity_router()` | 201-221 | Route для local payments |

**Файл**: [services/obligation-engine/src/nats_consumer.rs](services/obligation-engine/src/nats_consumer.rs)

**Критерий разделения**: `debtor_country ≠ creditor_country`

**Метод детекции**: BIC[4..6] (позиции 5-6)

**Статус**: ✅ Работает в production
