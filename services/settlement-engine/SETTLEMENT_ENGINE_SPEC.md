# Settlement Engine Technical Specification

## Purpose

**Settlement Engine is responsible for converting virtual net obligations into actual bank payments.**

It is the "physical execution layer" that ensures every netting result from the Clearing Engine becomes a real money movement in the banking system.

---

## Core Responsibilities

### 1. 📥 Accept Settlement Instructions

**From:** Clearing Engine (via NATS)

**Input Format:**
```rust
struct SettlementInstruction {
    instruction_id: Uuid,
    clearing_batch_id: Uuid,
    clearing_window: ClearingWindow,  // e.g., "2025-11-18T12:00:00Z"
    settlement_mode: SettlementMode,  // NORMAL / EMERGENCY

    // Net obligation details
    debtor_bank_id: Uuid,
    creditor_bank_id: Uuid,
    currency: Currency,
    net_amount: Decimal,

    // Original transaction references
    constituent_transactions: Vec<TransactionReference>,

    // Token reservation
    token_reservation_id: Uuid,
    reserved_tokens: Decimal,

    // Routing preferences
    preferred_payout_channel: PayoutChannel,  // API / SWIFT / FILE
    fallback_channels: Vec<PayoutChannel>,

    // Compliance & risk
    risk_score: f64,
    compliance_status: ComplianceStatus,

    // Timing
    requested_execution_time: DateTime<Utc>,
    latest_execution_time: DateTime<Utc>,  // SLA deadline
}

enum SettlementMode {
    NORMAL,        // Standard clearing window settlement
    EMERGENCY,     // Out-of-window urgent settlement
}
```

**Validations:**
1. ✅ Tokens are actually reserved in Token Engine (query `token_reservations` table)
2. ✅ Risk limits not exceeded (check Risk Engine)
3. ✅ Payout channels are available (check bank integration status)
4. ✅ Instruction is idempotent (not already processed)
5. ✅ Timing is feasible (not past cut-off time, bank is open)

**Actions:**
- Create `settlement_instructions` record with status = `PENDING`
- Publish event: `settlement.instruction.received`
- Start execution workflow

---

### 2. 🏗️ Build Outbound Payment Messages

**Responsibility:** Construct ISO 20022 or local format messages for bank APIs.

#### For ISO 20022 Banks (pacs.008)

**Message Structure:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.13">
  <FIToFICstmrCdtTrf>
    <GrpHdr>
      <MsgId>DELTRAN-STTLM-{instruction_id}</MsgId>
      <CreDtTm>{timestamp}</CreDtTm>
      <NbOfTxs>1</NbOfTxs>
      <TtlIntrBkSttlmAmt Ccy="{currency}">{net_amount}</TtlIntrBkSttlmAmt>
      <SttlmInf>
        <SttlmMtd>CLRG</SttlmMtd>  <!-- Clearing -->
        <ClrSys>
          <Prtry>DELTRAN</Prtry>
        </ClrSys>
      </SttlmInf>
      <InstgAgt>
        <FinInstnId>
          <Othr>
            <Id>DELTRAN</Id>
          </Othr>
        </FinInstnId>
      </InstgAgt>
      <InstdAgt>
        <FinInstnId>
          <BICFI>{creditor_bank_bic}</BICFI>
        </FinInstnId>
      </InstdAgt>
    </GrpHdr>
    <CdtTrfTxInf>
      <PmtId>
        <InstrId>{instruction_id}</InstrId>
        <EndToEndId>{end_to_end_id}</EndToEndId>
        <UETR>{uetr}</UETR>
      </PmtId>
      <IntrBkSttlmAmt Ccy="{currency}">{net_amount}</IntrBkSttlmAmt>
      <IntrBkSttlmDt>{settlement_date}</IntrBkSttlmDt>
      <ChrgBr>SHAR</ChrgBr>  <!-- Shared charges -->
      <Dbtr>
        <Nm>{debtor_bank_name}</Nm>
      </Dbtr>
      <DbtrAgt>
        <FinInstnId>
          <BICFI>{debtor_bank_bic}</BICFI>
        </FinInstnId>
      </DbtrAgt>
      <CdtrAgt>
        <FinInstnId>
          <BICFI>{creditor_bank_bic}</BICFI>
        </FinInstnId>
      </CdtrAgt>
      <Cdtr>
        <Nm>{creditor_bank_name}</Nm>
      </Nm>
      <RmtInf>
        <Ustrd>DelTran netting settlement for batch {clearing_batch_id}</Ustrd>
      </RmtInf>
    </CdtTrfTxInf>
  </FIToFICstmrCdtTrf>
</Document>
```

**Field Mappings:**
- `MsgId` = `DELTRAN-STTLM-{instruction_id}` (unique, idempotent)
- `InstrId` = `instruction_id` (UUID)
- `UETR` = Preserve from original pain.001 if single transaction, else generate new
- `EndToEndId` = Preserve from original, or use `DELTRAN-NET-{batch_id}`
- `IntrBkSttlmAmt` = `net_amount` (from Clearing Engine)
- `IntrBkSttlmDt` = Settlement date (usually T+0 or T+1)
- `DbtrAgt` / `CdtrAgt` = BIC codes from `banks` table

**Validation:**
- XSD schema validation before sending
- Business rules: amount > 0, valid BICs, date not in past
- Idempotency check: never send same `MsgId` twice

#### For Local Format Banks (SEPA, ACH, Faster Payments, etc.)

**Adapter Pattern:**
```rust
trait PaymentFormatAdapter {
    fn build_message(&self, instruction: &SettlementInstruction) -> Result<Vec<u8>, Error>;
    fn parse_response(&self, response: &[u8]) -> Result<SettlementResponse, Error>;
}

struct SepaAdapter;
impl PaymentFormatAdapter for SepaAdapter {
    fn build_message(&self, instruction: &SettlementInstruction) -> Result<Vec<u8>, Error> {
        // Build SEPA XML (pain.001 variant)
        // ...
    }
}

struct FasterPaymentsAdapter;
impl PaymentFormatAdapter for FasterPaymentsAdapter {
    fn build_message(&self, instruction: &SettlementInstruction) -> Result<Vec<u8>, Error> {
        // Build UK Faster Payments ISO 20022 message
        // ...
    }
}

struct UaeRailAdapter;
impl PaymentFormatAdapter for UaeRailAdapter {
    fn build_message(&self, instruction: &SettlementInstruction) -> Result<Vec<u8>, Error> {
        // Build UAE domestic format (potentially WPS or other)
        // ...
    }
}
```

**Selection Logic:**
```rust
fn select_adapter(bank: &Bank, currency: &Currency) -> Box<dyn PaymentFormatAdapter> {
    match (bank.country_code.as_str(), currency) {
        ("GB", Currency::GBP) => Box::new(FasterPaymentsAdapter),
        ("AE", Currency::AED) => Box::new(UaeRailAdapter),
        (country, Currency::EUR) if is_sepa_country(country) => Box::new(SepaAdapter),
        _ => Box::new(Pacs008Adapter),  // Default ISO 20022 pacs.008
    }
}
```

---

### 3. 🔌 Bank Integration & Execution

**Integration Channels:**

| Channel | Protocol | Use Case | Latency | Reliability |
|---------|----------|----------|---------|-------------|
| **REST API** | HTTPS + JSON/XML | Modern banks, real-time | <1s | High |
| **SWIFT** | MT or ISO 20022 via SWIFT network | Traditional banks | 5-30s | Very High |
| **Host-to-Host** | SFTP/AS2 + XML files | Large volume batch | 1-5 min | Medium |
| **Webhook** | HTTPS callback | Async status updates | N/A | Medium |

**Execution Logic:**

```rust
async fn execute_settlement(instruction: SettlementInstruction) -> Result<SettlementOutcome, Error> {
    // 1. Resolve bank and channel
    let bank = get_bank(instruction.creditor_bank_id).await?;
    let channel = select_channel(&bank, &instruction.preferred_payout_channel)?;

    // 2. Build message
    let adapter = select_adapter(&bank, &instruction.currency);
    let message = adapter.build_message(&instruction)?;

    // 3. Execute with retry
    let mut attempt = 0;
    let max_attempts = 3;
    let mut last_error = None;

    while attempt < max_attempts {
        attempt += 1;

        match channel.send(&message).await {
            Ok(response) => {
                // Parse response
                let settlement_response = adapter.parse_response(&response)?;

                // Update status
                update_instruction_status(
                    instruction.instruction_id,
                    SettlementStatus::EXECUTED,
                    Some(settlement_response.reference)
                ).await?;

                // Publish success event
                publish_event(SettlementEvent::Executed {
                    instruction_id: instruction.instruction_id,
                    bank_reference: settlement_response.reference,
                    executed_at: Utc::now(),
                }).await?;

                return Ok(SettlementOutcome::Success {
                    bank_reference: settlement_response.reference,
                });
            }

            Err(e) if e.is_retryable() => {
                last_error = Some(e);

                // Exponential backoff
                let backoff_ms = 1000 * 2_u64.pow(attempt - 1);  // 1s, 2s, 4s
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;

                continue;
            }

            Err(e) => {
                // Non-retryable error (e.g., invalid account, compliance reject)
                return Err(e);
            }
        }
    }

    // All retries exhausted
    Err(last_error.unwrap())
}
```

**Error Classification:**

```rust
enum SettlementError {
    // Technical errors (retryable)
    NetworkTimeout,
    BankApiUnavailable,
    TemporarySystemError,

    // Business errors (non-retryable)
    InvalidAccount,
    ClosedAccount,
    InsufficientFunds,
    ComplianceReject,
    AmountLimitExceeded,
    InvalidBIC,
    InvalidCurrency,

    // Configuration errors (non-retryable)
    ChannelNotConfigured,
    UnsupportedCurrency,
    BankNotOnboarded,
}

impl SettlementError {
    fn is_retryable(&self) -> bool {
        matches!(self,
            Self::NetworkTimeout |
            Self::BankApiUnavailable |
            Self::TemporarySystemError
        )
    }

    fn to_iso_reason_code(&self) -> &'static str {
        match self {
            Self::InvalidAccount => "AC01",  // IncorrectAccountNumber
            Self::ClosedAccount => "AC04",   // ClosedAccountNumber
            Self::InsufficientFunds => "AM04", // InsufficientFunds
            Self::ComplianceReject => "LEGL", // LegalDecision
            Self::AmountLimitExceeded => "AM09", // WrongAmount
            Self::InvalidBIC => "AG01",      // TransactionForbidden
            _ => "TECH",  // TechnicalProblem
        }
    }
}
```

---

### 4. 🔄 Retry & Fallback Logic

**Retry Strategy (Technical Failures):**

```rust
struct RetryPolicy {
    max_attempts: u32,
    backoff_strategy: BackoffStrategy,
    retry_window: Duration,
}

enum BackoffStrategy {
    Exponential { base_ms: u64 },     // 1s, 2s, 4s, 8s...
    Linear { interval_ms: u64 },      // 5s, 10s, 15s...
    Fixed { interval_ms: u64 },       // 5s, 5s, 5s...
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_attempts: 3,
            backoff_strategy: BackoffStrategy::Exponential { base_ms: 1000 },
            retry_window: Duration::from_secs(300),  // 5 minutes
        }
    }
}
```

**Fallback Strategy (Channel Failures):**

```rust
async fn execute_with_fallback(instruction: SettlementInstruction) -> Result<SettlementOutcome, Error> {
    let channels = [
        instruction.preferred_payout_channel,
        ...instruction.fallback_channels,
    ];

    for channel in channels {
        match execute_via_channel(&instruction, channel).await {
            Ok(outcome) => {
                // Log if fallback was used
                if channel != instruction.preferred_payout_channel {
                    log_fallback_usage(instruction.instruction_id, channel).await;
                }
                return Ok(outcome);
            }
            Err(e) if e.is_retryable() => {
                // Try next channel
                continue;
            }
            Err(e) => {
                // Non-retryable error, don't try other channels
                return Err(e);
            }
        }
    }

    Err(Error::AllChannelsFailed)
}
```

**Retry-in-Next-Window Logic:**

```rust
async fn handle_retry_exhausted(instruction: SettlementInstruction, error: SettlementError) {
    if error.is_retryable() && instruction.settlement_mode == SettlementMode::NORMAL {
        // Mark for retry in next clearing window
        update_instruction_status(
            instruction.instruction_id,
            SettlementStatus::RETRY_IN_NEXT_WINDOW,
            None
        ).await;

        // Calculate next window
        let next_window = calculate_next_clearing_window(Utc::now());

        // Republish to Clearing Engine
        publish_event(SettlementEvent::RetryScheduled {
            instruction_id: instruction.instruction_id,
            next_window,
            reason: error.to_string(),
        }).await;

    } else {
        // Hard failure
        update_instruction_status(
            instruction.instruction_id,
            SettlementStatus::FAILED_HARD,
            None
        ).await;

        // Initiate refund flow
        initiate_refund(instruction).await;
    }
}
```

---

### 5. 🚫 Handle Business Failures & Refunds

**Business Failure Flow:**

```rust
async fn handle_business_failure(
    instruction: SettlementInstruction,
    error: SettlementError
) -> Result<(), Error> {
    // 1. Update settlement status
    update_instruction_status(
        instruction.instruction_id,
        SettlementStatus::FAILED_HARD,
        Some(error.to_iso_reason_code())
    ).await?;

    // 2. Release token reservation (don't burn - we're refunding)
    release_token_reservation(instruction.token_reservation_id).await?;

    // 3. Create refund obligation
    let refund_obligation = create_refund_obligation(
        instruction.clone(),
        error.clone()
    ).await?;

    // 4. Publish to Obligation Engine
    publish_event(SettlementEvent::RefundRequired {
        original_instruction_id: instruction.instruction_id,
        refund_obligation_id: refund_obligation.obligation_id,
        reason: error.to_iso_reason_code(),
    }).await?;

    // 5. Generate pacs.002 rejection status
    let pacs002 = build_pacs002_rejection(
        &instruction,
        error.to_iso_reason_code(),
        &error.to_string()
    );

    send_to_gateway(pacs002).await?;

    // 6. Create investigation case if needed
    if error.requires_investigation() {
        create_investigation_case(instruction, error).await?;
    }

    Ok(())
}

async fn create_refund_obligation(
    original: SettlementInstruction,
    error: SettlementError
) -> Result<Obligation, Error> {
    // Create reverse obligation: creditor becomes debtor
    let refund = Obligation {
        obligation_id: Uuid::new_v4(),
        original_instruction_id: Some(original.instruction_id),
        obligation_type: ObligationType::Refund,

        // Swap debtor and creditor
        debtor_bank_id: original.creditor_bank_id,
        creditor_bank_id: original.debtor_bank_id,

        amount: original.net_amount,
        currency: original.currency,

        reason: format!("Refund for failed settlement: {}", error.to_iso_reason_code()),

        status: ObligationStatus::PENDING_FUNDING,
        created_at: Utc::now(),
    };

    insert_obligation(&refund).await?;
    Ok(refund)
}
```

**ISO Reason Codes for Common Failures:**

| Code | Name | Description | Refund? |
|------|------|-------------|---------|
| AC01 | IncorrectAccountNumber | Account number invalid | Yes |
| AC04 | ClosedAccountNumber | Account is closed | Yes |
| AC06 | BlockedAccount | Account blocked/frozen | Yes |
| AM04 | InsufficientFunds | Not enough balance | Yes |
| AM05 | Duplication | Duplicate payment | No (already processed) |
| AM09 | WrongAmount | Amount outside limits | Yes |
| LEGL | LegalDecision | Sanctions/legal block | Yes |
| TECH | TechnicalProblem | System error | Retry |

---

### 6. ✅ Reconciliation of Payouts

**Reconciliation Sources:**

1. **Synchronous Response (REST API)**
   - Immediate confirmation in HTTP response
   - Extract: bank reference, status, timestamp

2. **Asynchronous camt.054 (Credit/Debit Notification)**
   - Bank sends when money actually moved
   - Match by: EndToEndId, InstrId, or bank reference

3. **Daily camt.053 (Bank Statement)**
   - EOD reconciliation
   - Match by: amount, date, reference

4. **Webhook Callbacks**
   - Bank pushes status updates
   - Match by: instruction_id or bank reference

**Matching Logic:**

```rust
async fn reconcile_payout(notification: Camt054Notification) -> Result<(), Error> {
    // Extract key fields
    let bank_reference = notification.entry.account_servicer_ref;
    let end_to_end_id = notification.entry_details.tx_details.refs.end_to_end_id;
    let amount = notification.entry.amount;
    let currency = notification.entry.currency;
    let booking_date = notification.entry.booking_date;

    // Find matching settlement instruction
    let instruction = find_instruction_by_reference(
        &bank_reference,
        &end_to_end_id,
        amount,
        currency
    ).await?;

    match instruction {
        Some(instr) if instr.status == SettlementStatus::EXECUTED => {
            // Already reconciled (duplicate notification) - idempotent
            log::info!("Instruction {} already reconciled", instr.instruction_id);
            Ok(())
        }

        Some(instr) => {
            // Match found - reconcile
            update_instruction_status(
                instr.instruction_id,
                SettlementStatus::RECONCILED,
                Some(bank_reference.clone())
            ).await?;

            // Burn tokens (settlement confirmed)
            burn_tokens(
                instr.token_reservation_id,
                instr.reserved_tokens
            ).await?;

            // Update Token Engine
            publish_event(SettlementEvent::Reconciled {
                instruction_id: instr.instruction_id,
                bank_reference,
                booking_date,
            }).await?;

            // Close obligations
            close_obligations(instr.constituent_transactions).await?;

            // Generate final pacs.002 (ACSC - AcceptedSettlementCompleted)
            let pacs002 = build_pacs002_success(&instr, &bank_reference);
            send_to_gateway(pacs002).await?;

            Ok(())
        }

        None => {
            // No match found - create exception
            create_reconciliation_exception(ReconciliationException {
                notification_id: Uuid::new_v4(),
                bank_reference,
                end_to_end_id,
                amount,
                currency,
                booking_date,
                status: ReconciliationStatus::UNMATCHED,
                created_at: Utc::now(),
            }).await?;

            Err(Error::ReconciliationMismatch)
        }
    }
}

async fn find_instruction_by_reference(
    bank_ref: &str,
    end_to_end_id: &str,
    amount: Decimal,
    currency: Currency
) -> Result<Option<SettlementInstruction>, Error> {
    // Try exact match on bank reference first
    if let Some(instr) = query_instruction_by_bank_ref(bank_ref).await? {
        return Ok(Some(instr));
    }

    // Try match on end_to_end_id
    if let Some(instr) = query_instruction_by_end_to_end_id(end_to_end_id).await? {
        return Ok(Some(instr));
    }

    // Fuzzy match on amount + currency + date (last resort)
    query_instruction_by_amount_currency(amount, currency, Utc::now().date()).await
}
```

**Reconciliation Status Transitions:**

```
PENDING → EXECUTED → RECONCILED → CLOSED
   ↓          ↓
   FAILED_HARD → REFUNDED
   ↓
   RETRY_IN_NEXT_WINDOW → (back to PENDING)
```

---

### 7. 📊 Generate Status Messages

**pacs.002 Generation (FI-to-FI Status Report):**

```rust
fn build_pacs002_success(
    instruction: &SettlementInstruction,
    bank_reference: &str
) -> Pacs002 {
    Pacs002 {
        group_header: GroupHeader {
            message_id: format!("DELTRAN-STATUS-{}", Uuid::new_v4()),
            creation_date_time: Utc::now(),
        },
        transaction_info_and_status: TransactionInfoAndStatus {
            original_instruction_id: instruction.instruction_id.to_string(),
            original_end_to_end_id: instruction.constituent_transactions[0].end_to_end_id.clone(),
            transaction_status: TransactionStatus::ACSC,  // AcceptedSettlementCompleted
            status_reason_info: None,
            acceptance_date_time: Some(Utc::now()),
            account_servicer_reference: Some(bank_reference.to_string()),
        },
    }
}

fn build_pacs002_rejection(
    instruction: &SettlementInstruction,
    reason_code: &str,
    reason_text: &str
) -> Pacs002 {
    Pacs002 {
        group_header: GroupHeader {
            message_id: format!("DELTRAN-STATUS-{}", Uuid::new_v4()),
            creation_date_time: Utc::now(),
        },
        transaction_info_and_status: TransactionInfoAndStatus {
            original_instruction_id: instruction.instruction_id.to_string(),
            original_end_to_end_id: instruction.constituent_transactions[0].end_to_end_id.clone(),
            transaction_status: TransactionStatus::RJCT,  // Rejected
            status_reason_info: Some(StatusReasonInfo {
                reason: StatusReason {
                    code: reason_code.to_string(),
                },
                additional_info: Some(reason_text.to_string()),
            }),
            acceptance_date_time: None,
            account_servicer_reference: None,
        },
    }
}
```

**pain.002 Generation (Customer Status Report):**

```rust
fn build_pain002_from_pacs002(pacs002: &Pacs002) -> Pain002 {
    // Similar structure but customer-facing language
    Pain002 {
        group_header: GroupHeader {
            message_id: format!("DELTRAN-CUST-STATUS-{}", Uuid::new_v4()),
            creation_date_time: Utc::now(),
        },
        original_group_info: OriginalGroupInfo {
            original_message_id: pacs002.transaction_info_and_status.original_instruction_id.clone(),
            original_message_name_id: "pain.001".to_string(),
        },
        payment_info_and_status: PaymentInfoAndStatus {
            original_payment_info_id: pacs002.transaction_info_and_status.original_end_to_end_id.clone(),
            transaction_info_and_status: TransactionStatus {
                status_id: pacs002.transaction_info_and_status.transaction_status.clone(),
                status: map_status_to_customer_language(&pacs002.transaction_info_and_status.transaction_status),
            },
        },
    }
}

fn map_status_to_customer_language(status: &TransactionStatus) -> String {
    match status {
        TransactionStatus::ACCP => "Payment accepted and being processed".to_string(),
        TransactionStatus::ACSC => "Payment completed successfully".to_string(),
        TransactionStatus::RJCT => "Payment rejected - please contact support".to_string(),
        TransactionStatus::PDNG => "Payment pending".to_string(),
        _ => "Payment status unknown".to_string(),
    }
}
```

---

### 8. 📝 Settlement Batch Accounting

**Batch Record Structure:**

```rust
struct SettlementBatch {
    batch_id: Uuid,
    clearing_window: DateTime<Utc>,
    settlement_mode: SettlementMode,

    // Summary
    total_instructions: u32,
    total_amount_by_currency: HashMap<Currency, Decimal>,

    // Status breakdown
    executed: u32,
    reconciled: u32,
    failed: u32,
    pending: u32,

    // Timing
    batch_created_at: DateTime<Utc>,
    batch_completed_at: Option<DateTime<Utc>>,

    // Audit
    hash: String,  // Hash of all instruction IDs + amounts
    previous_batch_hash: Option<String>,  // Blockchain-style chain
}
```

**Ledger Integration:**

```rust
async fn record_settlement_in_ledger(instruction: &SettlementInstruction) {
    // Double-entry accounting
    let entries = vec![
        LedgerEntry {
            account: format!("bank:{}:tokenized_pool:{}", instruction.debtor_bank_id, instruction.currency),
            debit: Some(instruction.net_amount),
            credit: None,
            reference: instruction.instruction_id.to_string(),
        },
        LedgerEntry {
            account: format!("bank:{}:tokenized_pool:{}", instruction.creditor_bank_id, instruction.currency),
            debit: None,
            credit: Some(instruction.net_amount),
            reference: instruction.instruction_id.to_string(),
        },
    ];

    insert_ledger_entries(entries).await;
}
```

---

## State Machine

```
┌──────────┐
│ PENDING  │ ← Created from Clearing Engine
└─────┬────┘
      │
      ↓ execute_settlement()
┌──────────┐
│EXECUTING │ ← Sending to bank API/SWIFT
└─────┬────┘
      │
      ├─→ Success ─→ ┌──────────┐
      │              │ EXECUTED │ ← Bank accepted
      │              └─────┬────┘
      │                    │
      │                    ↓ reconcile_payout()
      │              ┌─────────────┐
      │              │ RECONCILED  │ ← camt.054 received
      │              └─────┬───────┘
      │                    │
      │                    ↓ close_obligations()
      │              ┌─────────┐
      │              │ CLOSED  │ ← Final state
      │              └─────────┘
      │
      ├─→ Technical Error (retryable) ─→ Retry (up to 3x)
      │                                      │
      │                                      ↓ If retries exhausted
      │                                ┌──────────────────────┐
      │                                │RETRY_IN_NEXT_WINDOW  │
      │                                └──────────────────────┘
      │
      └─→ Business Error (non-retryable) ─→ ┌─────────────┐
                                             │ FAILED_HARD │
                                             └──────┬──────┘
                                                    │
                                                    ↓ initiate_refund()
                                             ┌─────────────┐
                                             │  REFUNDED   │
                                             └─────────────┘
```

---

## Performance & SLAs

### Target Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Settlement Latency** | <5 seconds (P95) | Time from instruction received to bank API call |
| **Reconciliation Time** | <30 seconds (P95) | Time from camt.054 received to status update |
| **Success Rate** | >99.5% | Successful settlements / total attempts |
| **Retry Rate** | <5% | Instructions requiring retry / total |
| **Refund Rate** | <1% | Business failures requiring refund / total |

### Monitoring

**Key Metrics:**
```rust
struct SettlementMetrics {
    // Throughput
    instructions_per_second: f64,
    settlements_per_clearing_window: u32,

    // Latency
    p50_settlement_latency_ms: u64,
    p95_settlement_latency_ms: u64,
    p99_settlement_latency_ms: u64,

    // Success rates
    success_rate_overall: f64,
    success_rate_by_bank: HashMap<Uuid, f64>,
    success_rate_by_currency: HashMap<Currency, f64>,

    // Errors
    retry_rate: f64,
    hard_failure_rate: f64,
    reconciliation_mismatch_rate: f64,

    // Channel health
    channel_availability: HashMap<PayoutChannel, f64>,
    fallback_usage_rate: f64,
}
```

**Alerts:**
- Settlement success rate < 99% → PagerDuty
- P95 latency > 10s → Slack warning
- Reconciliation mismatch rate > 0.5% → Investigation required
- Any channel availability < 90% → Escalate to bank

---

## Security & Compliance

### Idempotency

**Guarantees:**
- Same `instruction_id` → never sent to bank twice
- Use `message_id` tracking in `sent_messages` table
- On retry: check if already sent before attempting

**Implementation:**
```rust
async fn ensure_idempotency(instruction_id: Uuid) -> Result<bool, Error> {
    let existing = query_one!(
        "SELECT status FROM settlement_instructions WHERE instruction_id = $1",
        instruction_id
    ).await?;

    match existing.status.as_str() {
        "EXECUTED" | "RECONCILED" | "CLOSED" => {
            // Already processed
            log::warn!("Duplicate settlement instruction: {}", instruction_id);
            Ok(false)  // Do not process
        }
        "PENDING" | "EXECUTING" => {
            // In progress - check sent_messages
            let sent = query_one!(
                "SELECT COUNT(*) as count FROM sent_messages WHERE instruction_id = $1",
                instruction_id
            ).await?;

            Ok(sent.count == 0)  // Process only if not sent
        }
        _ => Ok(true)  // Process
    }
}
```

### Audit Trail

**Immutable Log:**
```rust
struct SettlementAuditEntry {
    entry_id: Uuid,
    instruction_id: Uuid,
    event_type: SettlementEventType,
    timestamp: DateTime<Utc>,
    actor: String,  // System / user / bank
    details: serde_json::Value,

    // Hash chain
    entry_hash: String,
    previous_hash: String,
}

enum SettlementEventType {
    InstructionReceived,
    ValidationPassed,
    ExecutionStarted,
    MessageSent,
    ResponseReceived,
    RetryAttempted,
    FallbackUsed,
    ReconciliationMatched,
    StatusSent,
    RefundInitiated,
    Closed,
}
```

### Data Encryption

**At Rest:**
- Sensitive fields (bank credentials, account numbers) encrypted with AES-256
- Encryption keys managed via AWS KMS / HashiCorp Vault

**In Transit:**
- TLS 1.3 for all HTTP APIs
- SWIFT network security for SWIFT channels
- SFTP/AS2 with encryption for file-based integrations

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pacs008_message_building() {
        let instruction = create_test_instruction();
        let pacs008 = build_pacs008(&instruction).unwrap();

        // Validate structure
        assert_eq!(pacs008.group_header.msg_id, format!("DELTRAN-STTLM-{}", instruction.instruction_id));
        assert_eq!(pacs008.credit_transfer_tx_info.intr_bk_sttlm_amt.value, instruction.net_amount);

        // Validate XSD
        assert!(validate_against_xsd(&pacs008.to_xml(), "pacs.008.001.13").is_ok());
    }

    #[tokio::test]
    async fn test_retry_logic() {
        let instruction = create_test_instruction();

        // Mock bank API with 2 failures then success
        let mock_bank = MockBankApi::new()
            .fail_times(2)
            .then_succeed();

        let result = execute_settlement_with_mock(instruction, mock_bank).await;

        assert!(result.is_ok());
        assert_eq!(mock_bank.call_count(), 3);
    }

    #[tokio::test]
    async fn test_reconciliation_matching() {
        let instruction = create_and_execute_instruction().await;

        let camt054 = create_camt054_for_instruction(&instruction);
        let result = reconcile_payout(camt054).await;

        assert!(result.is_ok());

        let updated = get_instruction(instruction.instruction_id).await;
        assert_eq!(updated.status, SettlementStatus::RECONCILED);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_end_to_end_settlement_flow() {
    // Setup
    let test_db = TestDatabase::new().await;
    let settlement_engine = SettlementEngine::new_test(test_db.clone()).await;

    // 1. Create instruction from Clearing Engine
    let instruction = create_test_instruction();
    let result = settlement_engine.handle_instruction(instruction.clone()).await;
    assert!(result.is_ok());

    // 2. Simulate bank API response
    let bank_response = simulate_bank_success(&instruction);
    settlement_engine.handle_bank_response(bank_response).await.unwrap();

    // 3. Check status
    let instr = test_db.get_instruction(instruction.instruction_id).await.unwrap();
    assert_eq!(instr.status, SettlementStatus::EXECUTED);

    // 4. Simulate camt.054
    let camt054 = create_camt054(&instruction);
    settlement_engine.handle_camt054(camt054).await.unwrap();

    // 5. Verify reconciliation
    let instr = test_db.get_instruction(instruction.instruction_id).await.unwrap();
    assert_eq!(instr.status, SettlementStatus::RECONCILED);

    // 6. Verify tokens burned
    let reservation = test_db.get_token_reservation(instruction.token_reservation_id).await.unwrap();
    assert_eq!(reservation.status, ReservationStatus::BURNED);
}
```

---

## Configuration

```yaml
settlement_engine:
  retry_policy:
    max_attempts: 3
    backoff_strategy: exponential
    backoff_base_ms: 1000
    retry_window_seconds: 300

  channels:
    - name: rest_api
      enabled: true
      timeout_seconds: 30
      priority: 1

    - name: swift
      enabled: true
      timeout_seconds: 60
      priority: 2

    - name: host_to_host
      enabled: true
      timeout_seconds: 300
      priority: 3

  reconciliation:
    auto_match_threshold_seconds: 30
    create_exception_after_hours: 24

  sla:
    target_latency_p95_ms: 5000
    target_success_rate: 0.995

  monitoring:
    metrics_interval_seconds: 60
    alert_on_success_rate_below: 0.99
    alert_on_latency_p95_above_ms: 10000
```

---

## Summary

**Settlement Engine = "Money Movement Execution Layer"**

**Core Responsibilities:**
1. ✅ Accept net settlement instructions from Clearing Engine
2. ✅ Build ISO 20022 or local format payment messages
3. ✅ Execute via bank APIs/SWIFT with retry logic
4. ✅ Handle business failures and initiate refunds
5. ✅ Reconcile payouts against bank confirmations
6. ✅ Generate status messages (pacs.002, pain.002)
7. ✅ Maintain audit trail and batch accounting

**Key Guarantees:**
- **Idempotency:** Same instruction never executed twice
- **Atomicity:** Either full settlement or full refund
- **Auditability:** Immutable hash-chained log
- **Reliability:** Retry + fallback + reconciliation

**One Sentence:**
*"Settlement Engine converts virtual clearing results into actual bank payments, with bulletproof retry logic, reconciliation, and refund handling, ensuring every netted obligation becomes a real money movement."*

---

*Document Version: 1.0*
*Last Updated: 2025-11-18*
