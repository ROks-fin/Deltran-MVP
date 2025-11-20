# DelTran ISO 20022 Implementation Plan

## Executive Summary

This is the **master implementation guide** for integrating ISO 20022 messaging standards into the DelTran MVP platform.

**Status:** All 18 ISO 20022 archives extracted and mapped to DelTran services
**Total Messages:** 136 XSD schemas across 7 message families
**Documentation:** 11 MDR Part1 (business), 12 Part2 (technical), 11 Part3 (dictionaries)

**MVP Timeline:** 4-6 weeks for P0 messages, 8-12 weeks for full P0+P1 coverage

---

## What You Have

### Archive Inventory

| Archive | XSD Count | MDR Docs | Priority | Status |
|---------|-----------|----------|----------|--------|
| Payments Initiation | 4 | ❌ | **P0** | Extracted |
| Payments Clearing & Settlement | 8 | ✅ All | **P0** | Extracted |
| Multilateral Settlement | 3 | ✅ All | **P1** | Extracted |
| Cash Management | 35 | ✅ All | P1-P2 | Extracted |
| Bank-to-Customer Cash Mgmt | 4 | ✅ All | **P0** | Extracted |
| Bank Account Management | 15 | ✅ All | P2 | Extracted |
| Account Mgmt (Business Area) | 34 | ❌ | P2-P3 | Extracted |
| Change/Verify Account ID | 3 | ⚠️ P2/P3 | P2 | Extracted |
| Charges Management | 2 | ✅ All | P2 | Extracted |
| Exceptions & Investigations | 17 | ✅ All | **P1** | Extracted |
| Exceptions Modernisation | 2 | ✅ All | P3 | Extracted |
| Notification To Receive | 3 | ✅ All | P2 | Extracted |
| Notification Of Correspondence | 1 | ⚠️ P1/P2 | P3 | Extracted |
| Payment Tracking | 3 | ✅ All | P3 | Extracted |
| Remittance Advice | 2 | ❌ | P3 | Extracted |

**Legend:**
- ✅ All = MDR Part1, Part2, Part3 available
- ⚠️ = Partial MDR (missing Part1 or Part3)
- ❌ = XSD only, no MDR

---

## Implementation Phases

### PHASE 1: Foundation (Weeks 1-2) - **CRITICAL PATH**

**Goal:** Basic ISO 20022 infrastructure in Gateway

#### Tasks:

1. **Gateway: XML Parser & Validator**
   - [ ] Install XML parser (Rust: `quick-xml`, Go: `encoding/xml`)
   - [ ] Load XSD schemas into validator
   - [ ] Implement schema validation for incoming messages
   - [ ] Error handling: Generate ISO-compliant error responses
   - **Priority Messages:** pain.001, pacs.008, camt.054

2. **Canonical Model Definition**
   - [ ] Define core `CanonicalPayment` struct (see DELTRAN_ISO_MAPPING.md)
   - [ ] Implement ISO → Canonical mappers:
     - `pain.001` → `CanonicalPayment`
     - `pacs.008` → `CanonicalPayment`
     - `camt.054` → `FundingEvent`
   - [ ] Implement Canonical → ISO mappers:
     - `CanonicalPayment` → `pain.002`
     - `CanonicalPayment` → `pacs.002`
     - `CanonicalPayment` → `pacs.008`

3. **Database Schema Updates**
   - [ ] Add ISO fields to `transactions` table:
     - `uetr UUID`
     - `end_to_end_id VARCHAR(35)`
     - `instruction_id VARCHAR(35)`
     - `message_id VARCHAR(35)`
   - [ ] Add `funding_events` table for camt.054:
     ```sql
     CREATE TABLE funding_events (
       event_id UUID PRIMARY KEY,
       emi_account_id UUID REFERENCES emi_accounts(account_id),
       amount DECIMAL(18,2),
       currency VARCHAR(3),
       bank_reference VARCHAR(35),
       end_to_end_id VARCHAR(35),
       booking_date DATE,
       value_date DATE,
       status VARCHAR(20),
       camt054_xml TEXT,
       created_at TIMESTAMP
     );
     ```

4. **NATS Message Routing**
   - [ ] Define subjects for ISO message types:
     - `iso.pain.001` → Obligation Engine
     - `iso.pacs.008.inbound` → Clearing Engine
     - `iso.camt.054` → Token Engine
     - `iso.pacs.008.outbound` → Settlement Engine → Gateway
   - [ ] Implement Gateway NATS publishers/subscribers

5. **Testing Infrastructure**
   - [ ] Create ISO 20022 test message library:
     - `tests/iso_messages/pain.001.valid.xml`
     - `tests/iso_messages/pacs.008.valid.xml`
     - `tests/iso_messages/camt.054.valid.xml`
   - [ ] Integration tests: End-to-end message flow

**Deliverables:**
- ✅ Gateway can parse/validate ISO messages
- ✅ Canonical model with bi-directional ISO mapping
- ✅ Database schema supports ISO identifiers
- ✅ NATS routing for P0 messages

---

### PHASE 2: Core Payment Flow (Weeks 3-4) - **P0 MESSAGES**

**Goal:** Implement critical payment initiation & settlement flow

#### Priority P0 Messages:

| Message | Direction | Service | Complexity | Est. Hours |
|---------|-----------|---------|------------|------------|
| `pain.001` | IN | Gateway → Obligation | Medium | 16h |
| `pain.002` | OUT | Gateway ← Any | Medium | 12h |
| `pacs.008` | IN | Gateway → Clearing | High | 20h |
| `pacs.008` | OUT | Settlement → Gateway | High | 20h |
| `pacs.002` | OUT | Settlement → Gateway | Medium | 12h |
| `camt.054` | IN | Gateway → Token | **Critical** | 24h |

**Total Estimate:** ~104 hours (~2.5 weeks with 2 developers)

#### Implementation Order:

**Step 1: Payment Initiation (pain.001)**
```
1. Gateway receives pain.001 XML from bank
2. Validate against pain.001.001.12.xsd
3. Extract key fields:
   - GrpHdr.MsgId → message_id
   - PmtInf.PmtInfId → payment_info_id
   - CdtTrfTxInf.PmtId.EndToEndId → end_to_end_id
   - CdtTrfTxInf.Amt.InstdAmt → amount + currency
   - CdtTrfTxInf.Cdtr/Dbtr → parties
4. Map to CanonicalPayment
5. Publish to NATS: iso.pain.001
6. Obligation Engine receives, creates obligation (status=PENDING_FUNDING)
7. Gateway generates pain.002 response (ACCP - accepted)
```

**Reference Files:**
- XSD: `iso20022/payments_initiation/pain.001.001.12.xsd`
- Test: Create sample pain.001 with known good values

**Step 2: Funding Event (camt.054) - MOST CRITICAL**
```
1. Gateway receives camt.054 from bank (real money notification)
2. Validate against camt.054.001.13.xsd
3. Extract funding details:
   - Ntfctn.Ntry.Amt → amount
   - Ntfctn.Ntry.CdtDbtInd → must be CRDT
   - Ntfctn.Ntry.Sts → must be BOOK
   - Ntfctn.Ntry.AcctSvcrRef → bank reference
   - NtryDtls.TxDtls.Refs.EndToEndId → match to obligation
4. Publish to NATS: iso.camt.054
5. Token Engine receives:
   - Lookup emi_account by bank reference/mapping
   - UPDATE emi_accounts SET balance = balance + amount
   - INSERT INTO funding_events (...)
6. Token Engine publishes funding event
7. Obligation Engine matches funding to pending obligation:
   - Match by end_to_end_id or amount+reference
   - UPDATE obligations SET status=READY_FOR_CLEARING
8. Obligation publishes to Clearing Engine
```

**Reference Files:**
- XSD: `iso20022/banktocustomer_cash_management/camt.054.001.13.xsd`
- MDR: `iso20022/banktocustomer_cash_management/ISO20022_MDRPart2_BankToCustomerCashManagement_2024_2025_v1.pdf`
- **CRITICAL:** This is the trigger for the entire DelTran flow!

**Step 3: Settlement (pacs.008 outbound)**
```
1. Settlement Engine receives net settlement instruction from Clearing
2. Build pacs.008 message:
   - GrpHdr.MsgId → DELTRAN-STTLM-{uuid}
   - GrpHdr.InstgAgt → DelTran identifier
   - GrpHdr.InstdAgt → Beneficiary bank BIC
   - CdtTrfTxInf.PmtId.UETR → preserve original UETR
   - CdtTrfTxInf.IntrBkSttlmAmt → net settlement amount
   - CdtTrfTxInf.DbtrAgt/CdtrAgt → bank BICs
3. Publish to NATS: iso.pacs.008.outbound
4. Gateway receives, serializes to XML, validates
5. Gateway sends pacs.008 to beneficiary bank (via SWIFT/API)
```

**Reference Files:**
- XSD: `iso20022/payments_clearing_and_settlement/pacs.008.001.13.xsd`
- MDR Part2: Pages on pacs.008 structure and rules

**Step 4: Status Reporting (pacs.002, pain.002)**
```
1. Settlement Engine determines payment outcome
2. Build status message:
   - pain.002 for customer-initiated (pain.001)
   - pacs.002 for bank-initiated (pacs.008)
3. Set TxSts:
   - ACCP = Accepted (initial)
   - ACSC = AcceptedSettlementCompleted (success)
   - RJCT = Rejected (with reason code)
4. Include original identifiers (OrgnlInstrId, OrgnlEndToEndId)
5. Gateway sends to appropriate party
```

**Reference Files:**
- `pain.002.001.14.xsd`
- `pacs.002.001.15.xsd`

**Testing:**
1. End-to-end test: pain.001 → camt.054 → pacs.008 → pacs.002
2. Verify database state at each step
3. Validate all outbound XML against XSD

---

### PHASE 3: Returns & Reconciliation (Weeks 5-6) - **P1 MESSAGES**

**Goal:** Handle failures, cancellations, EOD reconciliation

#### Priority P1 Messages:

| Message | Purpose | Complexity |
|---------|---------|------------|
| `pacs.004` | Payment Return | Medium |
| `camt.053` | Daily Statement / EOD Reconciliation | High |
| `camt.055` | Customer Cancellation Request | Medium |
| `camt.056` | FI Cancellation Request | Medium |
| `pacs.029` | Multilateral Settlement (Netting) | High |

**Implementation Order:**

1. **Payment Returns (pacs.004)**
   - Implement reverse flow: Bank → Gateway → Settlement → Token Engine
   - Burn tokens, reverse obligation
   - Generate pacs.002 confirmation

2. **EOD Reconciliation (camt.053)**
   - Token Engine receives daily statement
   - Extract closing balance: `Bal.TpCd.Cd=CLBD`
   - Compare against `emi_accounts.balance`
   - Generate discrepancy report if mismatch
   - **Critical for financial integrity!**

3. **Cancellation Handling (camt.055/056)**
   - Pre-settlement: Cancel obligation, return funds
   - Post-settlement: Initiate pacs.004 return
   - Update payment status, notify parties

4. **Multilateral Netting (pacs.029)**
   - Clearing Engine generates pacs.029 after bilateral netting
   - Include net positions for each participant
   - Settlement Engine uses this for final settlements
   - Reduces settlement volume significantly

**Reference Files:**
- `pacs.004.001.14.xsd`
- `camt.053.001.13.xsd` (CRITICAL for reconciliation)
- `pacs.029.001.02.xsd` (in multilateral_settlement/)

---

### PHASE 4: Account Management & Compliance (Weeks 7-8) - **P2 MESSAGES**

**Goal:** EMI account lifecycle, investigations, fees

#### Priority P2 Messages:

| Message | Purpose | Complexity |
|---------|---------|------------|
| `acmt.007` | Account Opening | Medium |
| `acmt.009` | Account Modification | Low |
| `acmt.010` | Account Closure | Low |
| `acmt.011` | Account Status Report | Low |
| `camt.027` | Claim Non-Receipt (Investigation) | Medium |
| `camt.029` | Resolution Of Investigation | Medium |
| `camt.057` | Notification To Receive (Pre-advice) | Medium |
| `camt.105` | Charges Report | Low |

**Implementation:**

1. **Account Management (acmt.007-011)**
   - Token Engine manages `emi_accounts` lifecycle
   - acmt.007 → Create account
   - acmt.009 → Update account details
   - acmt.010 → Close account (mark inactive)
   - acmt.011 → Status report to bank

2. **Investigations (camt.027/029)**
   - Compliance Engine handles claim investigations
   - Track investigation cases in database
   - Generate camt.029 resolution after investigation

3. **Pre-Advice (camt.057)**
   - Settlement Engine sends before actual pacs.008
   - Beneficiary bank prepares to receive funds
   - Improves settlement efficiency

4. **Fee Reporting (camt.105)**
   - Reporting Engine generates after fee calculation
   - Transparency for fee structure
   - Regulatory compliance

---

### PHASE 5: Advanced Features (Weeks 9-12) - **P3 MESSAGES**

**Goal:** Tracking, remittance, extended features

#### Priority P3 Messages:

| Message | Purpose | Complexity |
|---------|---------|------------|
| `trck.001/002` | Payment Tracking (SWIFT gpi-style) | High |
| `remt.001/002` | Remittance Advice | Medium |
| `camt.110/111` | Modern Exception Handling | Medium |
| Extended `acmt.*` | Corporate account hierarchies | Low |

**Implementation:**
- Analytics/Tracking service for trck.* messages
- Remittance data separation from payment instructions
- Enhanced tracking UI/API

---

## Technical Architecture

### Component Overview

```
┌─────────────────────────────────────────────────────────────┐
│                         GATEWAY SERVICE                       │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ XML Parser  │→ │  Validator   │→ │  Canonical   │       │
│  │ (quick-xml) │  │  (XSD)       │  │  Mapper      │       │
│  └─────────────┘  └──────────────┘  └──────────────┘       │
│          ↓                                     ↓              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              NATS Publisher/Subscriber                │    │
│  └─────────────────────────────────────────────────────┘    │
└────────────────────────┬──────────────────────┬─────────────┘
                         │                      │
        ┌────────────────┴──────┐      ┌───────┴────────┐
        │  pain.001, pacs.008   │      │  camt.054      │
        │  (payments)           │      │  (funding)     │
        ↓                       ↓      ↓                │
┌──────────────┐      ┌──────────────────┐    ┌────────┴──────┐
│  OBLIGATION  │      │  CLEARING        │    │  TOKEN        │
│  ENGINE      │←────→│  ENGINE          │    │  ENGINE       │
└──────────────┘      └──────────────────┘    └───────────────┘
        │                      │                        │
        │                      ↓                        ↓
        │             ┌──────────────────┐    ┌───────────────┐
        └────────────→│  SETTLEMENT      │    │  emi_accounts │
                      │  ENGINE          │    │  (PostgreSQL) │
                      └──────────────────┘    └───────────────┘
                               │
                               ↓
                      ┌──────────────────┐
                      │  pacs.008 OUT    │
                      │  pacs.002 OUT    │
                      └──────────────────┘
```

### Message Flow Patterns

**Pattern 1: Customer Payment**
```
Customer → pain.001 → Gateway → Obligation (PENDING)
Bank → camt.054 → Gateway → Token → Obligation (READY)
Obligation → Clearing → Settlement → pacs.008 → Bank
Settlement → pacs.002/pain.002 → Gateway → Customer/Bank
```

**Pattern 2: Bank-to-Bank Payment**
```
Bank A → pacs.008 → Gateway → Clearing → Settlement
Settlement → pacs.008 → Gateway → Bank B
Settlement → pacs.002 → Gateway → Bank A
```

**Pattern 3: Payment Return**
```
Bank → pacs.004 → Gateway → Settlement
Settlement → Token (burn tokens)
Settlement → pacs.002 → Gateway → Bank
```

---

## Data Dictionary Strategy

### Extracting from MDR Part3 (Excel)

**Priority Order:**
1. **Payments Clearing & Settlement** (pacs.*)
2. **Bank-to-Customer Cash** (camt.052-054)
3. **Cash Management** (all camt.*)
4. **Account Management** (acmt.*)
5. Others as needed

**Process:**
1. Open `ISO20022_MDRPart3_*.xlsx`
2. Export relevant sheets:
   - Message definitions
   - Element dictionary
   - Code sets
3. Convert to JSON format:
   ```json
   {
     "UETR": {
       "type": "UUIDv4Identifier",
       "description": "Universally unique transaction reference",
       "pattern": "[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}",
       "occurs_in": ["pain.001", "pacs.008", "pacs.002"],
       "xml_path": "CdtTrfTxInf/PmtId/UETR"
     },
     "IntrBkSttlmAmt": {
       "type": "ActiveCurrencyAndAmount",
       "description": "Interbank settlement amount",
       "constraints": {
         "minInclusive": "0",
         "fractionDigits": "5",
         "totalDigits": "18"
       },
       "occurs_in": ["pacs.008", "pacs.009"],
       "xml_path": "CdtTrfTxInf/IntrBkSttlmAmt"
     }
   }
   ```
4. Use for:
   - Code generation (Rust structs, validation)
   - Documentation
   - API specs

---

## Code Generation from XSD

### Approach 1: Manual Parsing (Recommended for MVP)

**Rationale:** XSD → Code generators are often imperfect. Manual mapping gives control.

**Process:**
1. Read XSD, identify key elements
2. Hand-write Rust structs with appropriate types:
   ```rust
   #[derive(Debug, Serialize, Deserialize)]
   pub struct Pain001 {
       #[serde(rename = "GrpHdr")]
       pub group_header: GroupHeader,

       #[serde(rename = "PmtInf")]
       pub payment_information: Vec<PaymentInformation>,
   }

   #[derive(Debug, Serialize, Deserialize)]
   pub struct GroupHeader {
       #[serde(rename = "MsgId")]
       pub message_id: String, // Max35Text

       #[serde(rename = "CreDtTm")]
       pub creation_date_time: DateTime<Utc>, // ISODateTime

       #[serde(rename = "NbOfTxs")]
       pub number_of_transactions: String, // Max15NumericText
   }
   ```
3. Implement validation logic separately
4. Use `quick-xml` or `serde-xml-rs` for parsing

**Advantages:**
- Full control over mapping
- Better error messages
- Easier to customize for DelTran needs

### Approach 2: XSD → Code Generation (Future)

**Tools:**
- `xsd2` (Rust) - experimental
- `xjc` (Java) then translate
- Custom parser script

**Use after MVP** when structures stabilize.

---

## Validation Strategy

### 3-Layer Validation

**Layer 1: XSD Schema Validation**
```rust
use xmlschema::XMLSchema;

pub fn validate_against_xsd(xml: &str, xsd_path: &str) -> Result<(), ValidationError> {
    let schema = XMLSchema::from_file(xsd_path)?;
    schema.validate(xml)?;
    Ok(())
}
```

**Layer 2: Business Rules (MDR Part2)**
```rust
pub fn validate_pain001_business_rules(payment: &Pain001) -> Result<(), BusinessRuleError> {
    // Rule: Requested execution date must be today or future
    if payment.payment_information.requested_execution_date < Utc::now().date_naive() {
        return Err(BusinessRuleError::InvalidDate);
    }

    // Rule: Sum of transaction amounts must equal payment info control sum
    let tx_sum: Decimal = payment.payment_information.iter()
        .flat_map(|pi| &pi.credit_transfer_transactions)
        .map(|tx| tx.amount.value)
        .sum();
    if tx_sum != payment.payment_information.control_sum {
        return Err(BusinessRuleError::ControlSumMismatch);
    }

    // Rule: BIC must be valid format
    for pi in &payment.payment_information {
        validate_bic(&pi.debtor_agent.bic)?;
    }

    Ok(())
}
```

**Layer 3: DelTran-Specific Rules**
```rust
pub fn validate_deltran_rules(payment: &CanonicalPayment) -> Result<(), DelTranError> {
    // Check corridor support
    if !SUPPORTED_CORRIDORS.contains(&payment.corridor) {
        return Err(DelTranError::UnsupportedCorridor);
    }

    // Check limits
    if payment.amount > get_transaction_limit(&payment.corridor)? {
        return Err(DelTranError::LimitExceeded);
    }

    // Check EMI account exists
    let account = database::get_emi_account(&payment.debtor_account)?;
    if account.status != AccountStatus::Active {
        return Err(DelTranError::InactiveAccount);
    }

    Ok(())
}
```

---

## Testing Strategy

### Unit Tests

**Per Message Type:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pain001_valid_parsing() {
        let xml = include_str!("../tests/iso_messages/pain.001.valid.xml");
        let result = parse_pain001(xml);
        assert!(result.is_ok());

        let payment = result.unwrap();
        assert_eq!(payment.group_header.message_id, "MSG123456");
        assert_eq!(payment.payment_information.len(), 1);
    }

    #[test]
    fn test_pain001_invalid_schema() {
        let xml = include_str!("../tests/iso_messages/pain.001.invalid.xml");
        let result = parse_pain001(xml);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ParseError::SchemaValidation);
    }

    #[test]
    fn test_canonical_mapping() {
        let pain001 = create_test_pain001();
        let canonical = map_pain001_to_canonical(&pain001);

        assert_eq!(canonical.end_to_end_id, pain001.payment_information[0]
            .credit_transfer_transactions[0].payment_id.end_to_end_id);
        assert_eq!(canonical.amount, Decimal::from_str("1000.00").unwrap());
    }
}
```

### Integration Tests

**End-to-End Flows:**
```rust
#[tokio::test]
async fn test_complete_payment_flow() {
    // Setup
    let gateway = Gateway::new_test().await;
    let db = TestDatabase::new().await;

    // Step 1: Submit pain.001
    let pain001 = load_test_message("pain.001.valid.xml");
    let response = gateway.submit_payment(pain001).await.unwrap();
    assert_eq!(response.status, "ACCP");

    // Verify obligation created
    let obligation = db.get_obligation_by_end_to_end_id(&response.end_to_end_id).await.unwrap();
    assert_eq!(obligation.status, ObligationStatus::PendingFunding);

    // Step 2: Simulate funding event (camt.054)
    let camt054 = create_camt054_for_obligation(&obligation);
    gateway.submit_funding_event(camt054).await.unwrap();

    // Verify obligation status changed
    let obligation = db.get_obligation_by_id(&obligation.id).await.unwrap();
    assert_eq!(obligation.status, ObligationStatus::ReadyForClearing);

    // Verify token minted
    let account = db.get_emi_account(&obligation.debtor_account_id).await.unwrap();
    assert_eq!(account.balance, Decimal::from_str("1000.00").unwrap());

    // Step 3: Trigger clearing & settlement
    gateway.trigger_settlement().await.unwrap();

    // Verify pacs.008 sent
    let sent_messages = gateway.get_sent_messages().await;
    assert_eq!(sent_messages.len(), 1);
    assert_eq!(sent_messages[0].message_type, "pacs.008");

    // Verify pacs.002 generated
    let status_reports = db.get_status_reports_for_obligation(&obligation.id).await.unwrap();
    assert_eq!(status_reports[0].status, "ACSC");
}
```

### Message Test Library

**Create standard test messages:**
```
tests/iso_messages/
├── pain.001.valid.xml
├── pain.001.invalid_schema.xml
├── pain.001.invalid_business_rule.xml
├── pacs.008.valid.xml
├── pacs.008.valid_return.xml
├── camt.054.valid_credit.xml
├── camt.054.valid_debit.xml
├── camt.053.valid_statement.xml
├── pacs.002.success.xml
├── pacs.002.rejected.xml
└── ...
```

**Generate using:**
1. ISO 20022 message generators (if available)
2. Real examples from MDR Part2 documentation
3. Hand-crafted based on XSD

---

## Documentation Requirements

### For Each P0/P1 Message:

**Create markdown file:** `docs/messages/{message_type}.md`

**Structure:**
```markdown
# {message_type} - {MessageName}

## Overview
- **Direction:** Inbound/Outbound
- **Purpose:** {brief description}
- **DelTran Services:** {which services use it}
- **Priority:** P0/P1/P2/P3
- **XSD Location:** `iso20022/{archive}/{file}.xsd`
- **MDR Reference:** Part2, pages X-Y

## Business Context
{When is this message used? What triggers it?}

## Message Structure
{Key elements with descriptions}

## DelTran Mapping
### ISO → Canonical
{Field mappings}

### Canonical → ISO
{Reverse mappings}

## Validation Rules
### XSD Rules
{Schema constraints}

### Business Rules
{ISO 20022 business rules from MDR Part2}

### DelTran Rules
{Our specific requirements}

## Examples
### Valid Message
```xml
{example}
```

### Invalid Message (Schema)
```xml
{example with error}
```

## Testing
{How to test this message}

## Error Handling
{Common errors and responses}
```

---

## Key Milestones

### Week 2: Foundation Complete
- [ ] Gateway XML parsing working
- [ ] P0 XSD schemas loaded
- [ ] Canonical model defined
- [ ] Database schema updated
- [ ] Test infrastructure ready

### Week 4: Core Flow Working
- [ ] pain.001 → obligation flow
- [ ] camt.054 → token minting
- [ ] pacs.008 outbound settlement
- [ ] pacs.002/pain.002 status reports
- [ ] End-to-end test passing

### Week 6: Returns & Reconciliation
- [ ] pacs.004 returns working
- [ ] camt.053 EOD reconciliation
- [ ] camt.055/056 cancellations
- [ ] pacs.029 netting implemented

### Week 8: Account & Compliance
- [ ] acmt.007-011 account lifecycle
- [ ] camt.027/029 investigations
- [ ] camt.057 pre-advice
- [ ] camt.105 fee reporting

### Week 12: Advanced Features
- [ ] trck.001/002 tracking
- [ ] remt.001 remittance
- [ ] Full P0+P1+P2 coverage

---

## Resource Links

### Internal Documentation
- [ISO_INDEX.md](ISO_INDEX.md) - Complete archive navigation
- [DELTRAN_ISO_MAPPING.md](DELTRAN_ISO_MAPPING.md) - Service-message mapping
- [ARCHIVE_SCAN.md](ARCHIVE_SCAN.md) - File inventory

### ISO 20022 Archives
- All extracted to `iso20022/` directory
- XSD schemas in respective subdirectories
- MDR documentation (Part1/2/3) in archive folders

### External Resources
- ISO 20022 Official: https://www.iso20022.org/
- SWIFT ISO 20022: https://www.swift.com/standards/iso-20022
- Payments UK Implementation: https://www.wearepay.uk/iso-20022/

---

## Risk Assessment

### High Risk Areas

1. **camt.054 Funding Events** - CRITICAL PATH
   - **Risk:** Incorrect parsing leads to wrong balance updates
   - **Mitigation:** Extensive testing, reconciliation checks, audit logging

2. **pacs.008 Settlement Messages** - FINANCIAL IMPACT
   - **Risk:** Wrong amount/recipient in outbound settlement
   - **Mitigation:** Multi-stage validation, manual review for high values (MVP), settlement limits

3. **UETR Consistency** - TRACKING
   - **Risk:** UETR not preserved across message chain
   - **Mitigation:** Database constraints, validation at each hop

4. **Reconciliation Gaps** - OPERATIONAL
   - **Risk:** camt.053 reconciliation fails, balances drift
   - **Mitigation:** Daily automated checks, alerts on discrepancies

### Medium Risk Areas

5. **XSD Version Compatibility**
   - **Risk:** Bank uses different schema version
   - **Mitigation:** Support multiple versions in Gateway

6. **BIC Code Validation**
   - **Risk:** Invalid BIC causes settlement failure
   - **Mitigation:** Pre-validate against BIC directory

7. **Character Encoding**
   - **Risk:** Special characters break XML parsing
   - **Mitigation:** UTF-8 enforcement, sanitization

---

## Success Criteria

### MVP Success (End of Phase 2):
- ✅ 100% of P0 messages implemented
- ✅ End-to-end payment flow working: pain.001 → camt.054 → pacs.008 → pacs.002
- ✅ Zero critical bugs in funding/settlement
- ✅ All P0 tests passing (unit + integration)
- ✅ Production-ready error handling for P0 messages

### Full V1 Success (End of Phase 4):
- ✅ P0 + P1 + P2 messages implemented
- ✅ EOD reconciliation automated
- ✅ Account lifecycle management working
- ✅ Investigation/exception handling operational
- ✅ Comprehensive test coverage (>80%)
- ✅ Documentation complete for all implemented messages
- ✅ Bank integration tested with at least 2 partner banks

---

## Next Actions

### Immediate (This Week):
1. ✅ Review this implementation plan
2. [ ] Set up development branches: `feature/iso20022-gateway`, `feature/iso20022-token-engine`, etc.
3. [ ] Install XML parsing libraries
4. [ ] Create test message library (start with pain.001, pacs.008, camt.054)
5. [ ] Begin Phase 1: Gateway XML parser

### Short Term (Next 2 Weeks):
1. [ ] Complete Foundation (Phase 1)
2. [ ] Begin P0 message implementation
3. [ ] Daily standups to track progress
4. [ ] Weekly review of integration test results

### Medium Term (Weeks 3-6):
1. [ ] Complete Core Payment Flow (Phase 2)
2. [ ] Begin Returns & Reconciliation (Phase 3)
3. [ ] Start parallel work on account management
4. [ ] Coordinate with partner banks for testing

---

## Questions & Clarifications

**Before starting implementation, confirm:**

1. **Gateway Technology:** Rust or Go? (Affects XML library choice)
2. **Bank Connectivity:** SWIFT? REST API? File transfer?
3. **Message Volumes:** Expected TPS for planning load tests
4. **Bank Requirements:** Which specific XSD versions do partner banks require?
5. **Sandbox Environment:** Do banks provide ISO 20022 test endpoints?
6. **Regulatory:** Any specific ISO message requirements from regulators?

---

*This is your comprehensive roadmap. Follow it step-by-step, and you'll have a production-ready ISO 20022 integration.*

**Status:** Plan Ready
**Generated:** 2025-11-18
**Version:** 1.0
**Next Update:** After Phase 1 completion
