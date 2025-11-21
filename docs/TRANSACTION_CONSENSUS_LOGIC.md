# DelTran MVP - Transaction Consensus Logic & State Aggregation

## Overview

DelTran использует **распределенную архитектуру микросервисов** где каждый сервис отвечает за свою часть бизнес-логики. Для обеспечения консистентности решений по транзакциям используется **консенсус-логика с агрегацией состояний**.

---

## Архитектура принятия решений

### 1. Lifecycle транзакции

```
INITIATED → RISK_EVALUATED → COMPLIANCE_CHECKED → TOKEN_LOCKED →
CLEARING_QUEUED → SETTLED → CONFIRMED
```

### 2. Участники принятия решений

| Сервис | Ответственность | База данных | Решение |
|--------|----------------|-------------|---------|
| **Token Engine** | Эмиссия и управление токенами | `tokens`, `token_balances` | `BALANCE_SUFFICIENT` / `INSUFFICIENT` |
| **Risk Engine** | Оценка рисков транзакции | `risk_scores` | `APPROVE` / `REVIEW` / `REJECT` |
| **Compliance Engine** | AML/KYC/Sanctions | `compliance_checks` | `APPROVED` / `HOLD` / `REJECTED` |
| **Liquidity Router** | Прогноз ликвидности | `liquidity_predictions` | `INSTANT_SETTLE` / `QUEUE` |
| **Clearing Engine** | Multilateral netting | `clearing_batches`, `netting_results` | `NETTED` / `GROSS` |
| **Settlement Engine** | Окончательные расчеты | `settlement_instructions` | `SETTLED` / `FAILED` |

---

## Консенсус-логика

### Модель голосования (Voting-Based Consensus)

Каждая транзакция должна получить **одобрение всех критичных сервисов**:

#### Критичные сервисы (MUST APPROVE):
1. **Compliance Engine** - Отклоняет санкционированные транзакции
2. **Risk Engine** - Блокирует высокорисковые операции
3. **Token Engine** - Проверяет достаточность баланса

#### Некритичные сервисы (ADVISORY):
1. **Liquidity Router** - Рекомендует оптимальный путь
2. **Clearing Engine** - Оптимизирует через netting
3. **Settlement Engine** - Подтверждает финализацию

### Алгоритм принятия решения

```rust
fn determine_transaction_status(
    compliance: ComplianceStatus,
    risk: RiskDecision,
    token_balance: BalanceCheck,
    liquidity: LiquidityPrediction,
) -> TransactionStatus {
    // CRITICAL: Any rejection blocks transaction
    if compliance == ComplianceStatus::Rejected {
        return TransactionStatus::Rejected("Compliance violation");
    }

    if risk == RiskDecision::Reject {
        return TransactionStatus::Rejected("Risk too high");
    }

    if !token_balance.is_sufficient() {
        return TransactionStatus::Rejected("Insufficient balance");
    }

    // HOLDS: Require manual review
    if compliance == ComplianceStatus::Hold ||
       compliance == ComplianceStatus::ReviewRequired {
        return TransactionStatus::PendingReview("Compliance review required");
    }

    if risk == RiskDecision::Review {
        return TransactionStatus::PendingReview("Risk review required");
    }

    // APPROVED: Route based on liquidity
    if liquidity.can_instant_settle {
        return TransactionStatus::Approved("Ready for instant settlement");
    } else {
        return TransactionStatus::Approved("Queued for batch clearing");
    }
}
```

---

## Агрегация состояния транзакции

### 1. Таблица агрегации (Transaction State View)

Создается **материализованное представление** (`materialized view`) для агрегации:

```sql
CREATE MATERIALIZED VIEW transaction_state_aggregation AS
SELECT
    t.id AS transaction_id,
    t.sender_bank_id,
    t.receiver_bank_id,
    t.amount,
    t.sent_currency,
    t.received_currency,
    t.status AS transaction_status,

    -- Compliance Decision
    cc.overall_status AS compliance_status,
    cc.risk_rating AS compliance_risk,
    cc.required_actions AS compliance_actions,

    -- Risk Assessment
    rs.decision AS risk_decision,
    rs.overall_score AS risk_score,
    rs.confidence AS risk_confidence,

    -- Token Availability
    (SELECT available_balance >= t.amount
     FROM token_balances
     WHERE bank_id = t.sender_bank_id
       AND currency = CONCAT('x', t.sent_currency)
    ) AS has_sufficient_balance,

    -- Clearing Status
    cb.status AS clearing_status,
    cb.netting_amount,

    -- Settlement Status
    si.status AS settlement_status,
    si.settled_at,

    -- Aggregated Decision
    CASE
        WHEN cc.overall_status = 'Rejected' THEN 'REJECTED_COMPLIANCE'
        WHEN rs.decision = 'Reject' THEN 'REJECTED_RISK'
        WHEN NOT (SELECT available_balance >= t.amount FROM token_balances WHERE bank_id = t.sender_bank_id AND currency = CONCAT('x', t.sent_currency))
            THEN 'REJECTED_INSUFFICIENT_FUNDS'
        WHEN cc.overall_status IN ('Hold', 'ReviewRequired') OR rs.decision = 'Review'
            THEN 'PENDING_REVIEW'
        WHEN si.status = 'Settled' THEN 'SETTLED'
        WHEN cb.status = 'Completed' THEN 'CLEARING_COMPLETED'
        WHEN cc.overall_status = 'Approved' AND rs.decision = 'Approve'
            THEN 'APPROVED_PENDING_SETTLEMENT'
        ELSE 'PROCESSING'
    END AS aggregated_status,

    -- Timestamps
    t.created_at,
    cc.checked_at AS compliance_checked_at,
    rs.calculated_at AS risk_evaluated_at,
    si.settled_at AS settlement_completed_at,

    NOW() AS last_updated

FROM transactions t
LEFT JOIN compliance_checks cc ON t.id = cc.transaction_id
LEFT JOIN risk_scores rs ON t.id = rs.transaction_id
LEFT JOIN clearing_batches cb ON t.clearing_batch_id = cb.id
LEFT JOIN settlement_instructions si ON t.id = si.transaction_id;

-- Index for fast lookups
CREATE INDEX idx_transaction_aggregation_id ON transaction_state_aggregation(transaction_id);
CREATE INDEX idx_transaction_aggregation_status ON transaction_state_aggregation(aggregated_status);
CREATE INDEX idx_transaction_aggregation_updated ON transaction_state_aggregation(last_updated DESC);
```

### 2. Периодическое обновление

```sql
-- Refresh every 5 seconds
REFRESH MATERIALIZED VIEW CONCURRENTLY transaction_state_aggregation;
```

---

## Event-Driven Coordination через NATS JetStream

### 1. Публикация событий

Каждый сервис публикует решение в NATS:

```rust
// Compliance Engine
async fn publish_compliance_decision(nats: &NatsClient, result: &ComplianceCheckResult) {
    let event = ComplianceEvent {
        transaction_id: result.transaction_id,
        status: result.overall_status,
        risk_rating: result.risk_rating,
        required_actions: result.required_actions,
        timestamp: Utc::now(),
    };

    nats.publish("transaction.compliance.decision", event).await;
}

// Risk Engine
async fn publish_risk_decision(nats: &NatsClient, risk: &RiskScore) {
    let event = RiskEvent {
        transaction_id: risk.transaction_id,
        decision: risk.decision,
        score: risk.overall_score,
        confidence: risk.confidence,
        timestamp: Utc::now(),
    };

    nats.publish("transaction.risk.decision", event).await;
}
```

### 2. Orchestration Service (Gateway)

Gateway подписывается на все события и координирует workflow:

```rust
struct TransactionOrchestrator {
    nats: NatsClient,
    db: PgPool,
}

impl TransactionOrchestrator {
    async fn handle_compliance_decision(&self, event: ComplianceEvent) {
        if event.status == ComplianceStatus::Rejected {
            // Immediately reject transaction
            self.update_transaction_status(
                event.transaction_id,
                "REJECTED_COMPLIANCE"
            ).await;

            self.publish_transaction_rejected(event.transaction_id).await;
        } else if event.status == ComplianceStatus::Approved {
            // Check if all other services approved
            if self.check_all_approvals(event.transaction_id).await {
                self.trigger_clearing(event.transaction_id).await;
            }
        }
    }

    async fn check_all_approvals(&self, txn_id: Uuid) -> bool {
        let state = self.get_transaction_state(txn_id).await;

        state.compliance_status == ComplianceStatus::Approved &&
        state.risk_decision == RiskDecision::Approve &&
        state.has_sufficient_balance
    }
}
```

---

## Conflict Resolution

### Priority System для противоречивых решений:

1. **Compliance REJECT** > All other decisions
2. **Risk REJECT** > All except Compliance
3. **Insufficient Balance** > Advisory services
4. **Manual Review Required** > Auto-approval

### Пример сценария:

| Сервис | Решение | Приоритет |
|--------|---------|-----------|
| Compliance | `APPROVED` | ✓ |
| Risk | `REVIEW` (50% score) | ⚠️ |
| Token | `SUFFICIENT` | ✓ |
| Liquidity | `INSTANT_SETTLE` | Advisory |

**Итоговое решение**: `PENDING_REVIEW` (Risk требует ручного одобрения)

---

## Database Schema для координации

### Transaction Event Log

```sql
CREATE TABLE transaction_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_id UUID NOT NULL REFERENCES transactions(id),
    service_name VARCHAR(50) NOT NULL, -- 'compliance', 'risk', 'token', etc.
    event_type VARCHAR(50) NOT NULL, -- 'DECISION', 'STATUS_CHANGE', 'ERROR'
    decision VARCHAR(20), -- 'APPROVE', 'REJECT', 'REVIEW'
    payload JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_events_transaction (transaction_id, occurred_at DESC),
    INDEX idx_events_service (service_name, occurred_at DESC)
);
```

### Transaction Decisions Aggregate

```sql
CREATE TABLE transaction_decisions (
    transaction_id UUID PRIMARY KEY REFERENCES transactions(id),

    -- Individual service decisions
    compliance_status VARCHAR(20),
    compliance_checked_at TIMESTAMPTZ,

    risk_decision VARCHAR(20),
    risk_score NUMERIC(5,2),
    risk_evaluated_at TIMESTAMPTZ,

    token_balance_sufficient BOOLEAN,
    token_checked_at TIMESTAMPTZ,

    liquidity_recommendation VARCHAR(50),
    liquidity_predicted_at TIMESTAMPTZ,

    -- Aggregated decision
    final_decision VARCHAR(30) NOT NULL DEFAULT 'PENDING',
    decision_reason TEXT,
    decided_at TIMESTAMPTZ,

    -- Tracking
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Function to auto-update aggregated decision
CREATE OR REPLACE FUNCTION update_transaction_decision()
RETURNS TRIGGER AS $$
BEGIN
    NEW.final_decision := determine_final_decision(
        NEW.compliance_status,
        NEW.risk_decision,
        NEW.token_balance_sufficient
    );
    NEW.updated_at := NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_decision
    BEFORE UPDATE ON transaction_decisions
    FOR EACH ROW
    EXECUTE FUNCTION update_transaction_decision();
```

---

## Real-time Dashboard Query

Для получения полной информации о транзакции:

```sql
SELECT
    tsa.transaction_id,
    tsa.sender_bank_id,
    tsa.receiver_bank_id,
    tsa.amount,
    tsa.aggregated_status,

    -- Decisions breakdown
    jsonb_build_object(
        'compliance', jsonb_build_object(
            'status', tsa.compliance_status,
            'risk_rating', tsa.compliance_risk,
            'actions', tsa.compliance_actions
        ),
        'risk', jsonb_build_object(
            'decision', tsa.risk_decision,
            'score', tsa.risk_score,
            'confidence', tsa.risk_confidence
        ),
        'token', jsonb_build_object(
            'sufficient_balance', tsa.has_sufficient_balance
        ),
        'clearing', jsonb_build_object(
            'status', tsa.clearing_status,
            'netting_amount', tsa.netting_amount
        ),
        'settlement', jsonb_build_object(
            'status', tsa.settlement_status,
            'settled_at', tsa.settled_at
        )
    ) AS decisions,

    -- Timeline
    jsonb_build_object(
        'created', tsa.created_at,
        'compliance_checked', tsa.compliance_checked_at,
        'risk_evaluated', tsa.risk_evaluated_at,
        'settled', tsa.settlement_completed_at
    ) AS timeline

FROM transaction_state_aggregation tsa
WHERE tsa.transaction_id = $1;
```

---

## Conclusion

Эта архитектура обеспечивает:

✅ **Consistency** - Все решения записываются в БД
✅ **Availability** - Каждый сервис работает независимо
✅ **Partition Tolerance** - NATS обеспечивает асинхронную координацию
✅ **Auditability** - Полный event log для регуляторов
✅ **Real-time monitoring** - Materialized view обновляется каждые 5 сек

Система реализует **Eventually Consistent** модель с **Strong Consistency** для критичных операций (compliance, risk).
