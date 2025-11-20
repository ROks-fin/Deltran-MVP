# DelTran ISO 20022 Message Mapping

## Purpose

This document maps ISO 20022 messages to specific DelTran microservices, defining:
- **Which messages** each service consumes/produces
- **Direction** (inbound from bank, outbound to bank, internal)
- **Priority** (P0/P1/P2/P3 for MVP implementation)
- **Integration points** with DelTran internal Canonical Model

---

## 1. GATEWAY SERVICE

**Role:** Entry/exit point for all external ISO 20022 messages

### Inbound Messages (Bank → DelTran)

| Message | Name | Priority | Purpose | Next Service |
|---------|------|----------|---------|--------------|
| `pain.001` | CustomerCreditTransferInitiation | **P0** | Bank/customer submits payment request | → Obligation Engine |
| `pain.007` | CustomerPaymentReversal | P2 | Customer requests payment reversal | → Settlement Engine |
| `pain.008` | CustomerDirectDebitInitiation | P3 | Direct debit instruction (future) | → Obligation Engine |
| `pacs.008` | FIToFICustomerCreditTransfer | **P0** | Bank sends incoming payment | → Clearing Engine |
| `pacs.004` | PaymentReturn | **P1** | Bank returns rejected payment | → Settlement Engine |
| `pacs.007` | FIToFIPaymentReversal | P2 | Bank requests reversal | → Settlement Engine |
| `pacs.028` | FIToFIPaymentStatusRequest | P2 | Bank queries payment status | → Reporting Engine |
| `camt.054` | BankToCustomerDebitCreditNotification | **P0** | FUNDING EVENT - real money arrived | → Token Engine |
| `camt.053` | BankToCustomerStatement | **P1** | EOD statement for reconciliation | → Token Engine (Reconciliation) |
| `camt.055` | CustomerPaymentCancellationRequest | **P1** | Customer cancels payment before settlement | → Settlement Engine |
| `camt.056` | FIToFIPaymentCancellationRequest | **P1** | Bank cancels payment after settlement | → Settlement Engine |
| `camt.027` | ClaimNonReceipt | P2 | Investigation: payment not received | → Compliance Engine |
| `camt.087` | RequestToModifyPayment | P2 | Modify payment in flight | → Settlement Engine |
| `acmt.007` | AccountOpeningInstruction | P2 | Open new EMI account | → Token Engine |
| `acmt.009` | AccountModificationRequest | P2 | Modify account details | → Token Engine |
| `acmt.010` | AccountClosureRequest | P2 | Close EMI account | → Token Engine |
| `acmt.023` | IdentificationVerificationRequest | P2 | Verify account IBAN/BBAN | → Compliance Engine |
| `trck.001` | PaymentTrackingRequest | P3 | Query payment tracking status | → Analytics/Tracking |
| `remt.001` | RemittanceAdvice | P3 | Detailed remittance data | → Reporting Engine |

### Outbound Messages (DelTran → Bank)

| Message | Name | Priority | Purpose | Triggered By |
|---------|------|----------|---------|--------------|
| `pain.002` | CustomerPaymentStatusReport | **P0** | Status of customer payment | Obligation/Settlement Engine |
| `pacs.002` | FIToFIPaymentStatusReport | **P0** | Status to bank after settlement | Settlement Engine |
| `pacs.008` | FIToFICustomerCreditTransfer | **P0** | DelTran sends payment to bank | Settlement Engine |
| `pacs.004` | PaymentReturn | **P1** | DelTran returns failed payment | Settlement Engine |
| `camt.057` | NotificationToReceive | P2 | Pre-advice: expect incoming payment | Settlement Engine |
| `camt.029` | ResolutionOfInvestigation | P2 | Close investigation case | Compliance Engine |
| `camt.105` | ChargesReport | P2 | Report fees charged | Reporting Engine |
| `acmt.011` | AccountStatusReport | P2 | Account status update | Token Engine |
| `acmt.024` | IdentificationVerificationReport | P2 | Result of IBAN verification | Compliance Engine |
| `trck.002` | PaymentTrackingStatusReport | P3 | Payment tracking response | Analytics/Tracking |

### Validation & Transformation

**Gateway Responsibilities:**
1. **XML Parsing:** Parse ISO 20022 XML, validate against XSD schemas
2. **Business Validation:** Apply ISO business rules from MDR Part2
3. **Canonical Mapping:** Convert ISO → DelTran Canonical Model
4. **Routing:** Route to appropriate internal service via NATS
5. **Response Mapping:** Convert Canonical → ISO 20022 for outbound
6. **Error Handling:** Generate ISO-compliant error responses (pacs.002, pain.002)

**Key Canonical Fields:**
```rust
struct CanonicalPayment {
    deltran_tx_id: Uuid,          // Internal ID
    uetr: Option<Uuid>,           // ISO UETR from pain.001/pacs.008
    end_to_end_id: String,        // EndToEndId from ISO
    instruction_id: String,       // InstrId from ISO
    amount: Decimal,              // InstdAmt or IntrBkSttlmAmt
    currency: Currency,           // Ccy
    debtor: Party,                // Debtor info
    creditor: Party,              // Creditor info
    debtor_agent: FinancialInst,  // DebtorAgent BIC
    creditor_agent: FinancialInst,// CreditorAgent BIC
    remittance_info: String,      // RmtInf
    ...
}
```

---

## 2. OBLIGATION ENGINE

**Role:** Tracks payment obligations, holds until funding confirmed

### Inbound Messages

| Message | Source | Priority | Action |
|---------|--------|----------|--------|
| `pain.001` | Gateway | **P0** | Create obligation record, status=PENDING_FUNDING |
| `camt.054` (from Token) | Token Engine | **P0** | Match funding to obligation → READY_FOR_CLEARING |

### Outbound Messages

| Message | Destination | Priority | Trigger |
|---------|-------------|----------|---------|
| Internal event | Clearing Engine | **P0** | Obligation ready for settlement |
| `pain.002` (via Gateway) | Bank | **P0** | Status: "Accepted but pending funding" |

### ISO Field Mapping

**pain.001 → Obligation:**
- `GrpHdr.MsgId` → `obligation.instruction_id`
- `PmtInf.PmtInfId` → `obligation.payment_info_id`
- `CdtTrfTxInf.PmtId.EndToEndId` → `obligation.end_to_end_id`
- `CdtTrfTxInf.Amt.InstdAmt` → `obligation.amount`
- `CdtTrfTxInf.Cdtr` → `obligation.creditor_info`

**camt.054 → Funding Match:**
- `Ntfctn.Ntry.Amt` → match against `obligation.amount`
- `Ntfctn.Ntry.AcctSvcrRef` → lookup by reference
- `Ntfctn.Ntry.CdtDbtInd=CRDT` → must be credit

---

## 3. TOKEN ENGINE

**Role:** Manages EMI accounts, mints/burns internal tokens based on real funding

### Inbound Messages

| Message | Source | Priority | Action |
|---------|--------|----------|--------|
| `camt.054` | Gateway | **P0** | FUNDING EVENT → Update `emi_accounts.balance` |
| `camt.053` | Gateway | **P1** | EOD reconciliation → Verify balances |
| `acmt.007` | Gateway | P2 | Create new `emi_accounts` record |
| `acmt.009` | Gateway | P2 | Update `emi_accounts` metadata |
| `acmt.010` | Gateway | P2 | Close account (mark inactive) |

### Outbound Messages

| Message | Destination | Priority | Trigger |
|---------|-------------|----------|---------|
| Internal event | Obligation Engine | **P0** | Funding received → match obligations |
| `acmt.011` (via Gateway) | Bank | P2 | Account status changed |

### ISO Field Mapping

**camt.054 → Token Mint:**
```rust
// Incoming camt.054 structure (simplified)
Notification {
    Ntry: Entry {
        Amt: 10000.00,
        Ccy: "EUR",
        CdtDbtInd: CRDT,    // Must be credit
        Sts: BOOK,          // Booked (not pending)
        BookgDt: 2025-11-18,
        ValDt: 2025-11-18,
        AcctSvcrRef: "REF123", // Bank reference
        BkTxCd: ...,
        NtryDtls: {
            TxDtls: {
                Refs: {
                    EndToEndId: "UETR123", // Match to obligation
                    InstrId: "INSTR456"
                },
                Dbtr: {...},
                DbtrAgt: {...}
            }
        }
    }
}
```

**Mapping:**
1. Extract `Ntry.Amt` + `Ntry.Ccy`
2. Lookup `emi_accounts` by `AcctSvcrRef` or bank account mapping
3. **Update balance:** `balance += Ntry.Amt`
4. Create `funding_events` record:
   - `event_id` = UUID
   - `emi_account_id` = matched account
   - `amount` = `Ntry.Amt`
   - `currency` = `Ntry.Ccy`
   - `reference` = `Ntry.AcctSvcrRef`
   - `end_to_end_id` = `NtryDtls.TxDtls.Refs.EndToEndId`
   - `booking_date` = `Ntry.BookgDt`
   - `value_date` = `Ntry.ValDt`
5. Publish event to Obligation Engine for matching

**camt.053 → Reconciliation:**
- Extract `Bal.TpCd.CdOrPrtry.Cd=CLBD` (closing booked balance)
- Compare against internal `emi_accounts.balance`
- Flag discrepancies for investigation

**acmt.007 → Account Creation:**
- `AcctId.Othr.Id` → `emi_accounts.account_number`
- `AcctId.IBAN` → `emi_accounts.iban`
- `Acct.Ccy` → `emi_accounts.currency`
- `AcctOwnr.Nm` → `emi_accounts.account_holder`

---

## 4. CLEARING ENGINE

**Role:** Multilateral netting, generates net settlement obligations

### Inbound Messages

| Message | Source | Priority | Action |
|---------|--------|----------|--------|
| Internal event | Obligation Engine | **P0** | Payment ready → Add to netting batch |

### Outbound Messages

| Message | Destination | Priority | Trigger |
|---------|-------------|----------|---------|
| `pacs.029` | Settlement Engine | **P1** | Multilateral netting result |
| Internal event | Settlement Engine | **P0** | Net obligations for settlement |

### ISO Field Mapping

**pacs.029 MultilateralSettlementRequest:**

```xml
<MltlSttlmReq>
  <GrpHdr>
    <MsgId>CLEARING-BATCH-001</MsgId>
    <CreDtTm>2025-11-18T14:00:00Z</CreDtTm>
    <SttlmSsnIdr>SESSION-20251118-1400</SttlmSsnIdr>
  </GrpHdr>
  <SttlmInf>
    <SttlmMtd>CLRG</SttlmMtd> <!-- Clearing -->
    <SttlmAcct>
      <Id>
        <Othr>
          <Id>DELTRAN-CLEARING-ACCOUNT</Id>
        </Othr>
      </Id>
    </SttlmAcct>
  </SttlmInf>
  <Ptcpt> <!-- For each participating bank -->
    <PtcptId>
      <FinInstnId>
        <BICFI>BANKAEBBXXX</BICFI>
      </FinInstnId>
    </PtcptId>
    <NtSttlmAmt Ccy="EUR">5000.00</NtSttlmAmt> <!-- Net position -->
    <CdtDbtInd>CRDT</CdtDbtInd> <!-- Bank receives -->
  </Ptcpt>
  <Ptcpt>
    <PtcptId>
      <FinInstnId>
        <BICFI>BANKBEBBXXX</BICFI>
      </FinInstnId>
    </PtcptId>
    <NtSttlmAmt Ccy="EUR">5000.00</NtSttlmAmt>
    <CdtDbtInd>DBIT</CdtDbtInd> <!-- Bank pays -->
  </Ptcpt>
</MltlSttlmReq>
```

**Mapping:**
- Calculate bilateral nets between all banks
- Apply multilateral netting algorithm
- Generate `pacs.029` with net positions for each `Ptcpt`
- Settlement Engine executes actual settlements based on this

---

## 5. SETTLEMENT ENGINE

**Role:** Executes final settlements with banks

### Inbound Messages

| Message | Source | Priority | Action |
|---------|--------|----------|--------|
| Internal event | Clearing Engine | **P0** | Execute net settlements |
| `pacs.004` | Gateway | **P1** | Process returned payment |
| `pacs.007` | Gateway | P2 | Handle reversal request |
| `camt.055/056` | Gateway | **P1** | Process cancellation |

### Outbound Messages

| Message | Destination | Priority | Trigger |
|---------|-------------|----------|---------|
| `pacs.008` | Gateway → Bank | **P0** | Send payment to bank |
| `pacs.002` | Gateway → Bank | **P0** | Confirm settlement status |
| `pacs.004` | Gateway → Bank | **P1** | Return failed payment |
| `camt.057` | Gateway → Bank | P2 | Pre-advice notification |

### ISO Field Mapping

**pacs.008 Outbound (DelTran → Bank):**

```xml
<FIToFICstmrCdtTrf>
  <GrpHdr>
    <MsgId>DELTRAN-STTLM-{uuid}</MsgId>
    <CreDtTm>{timestamp}</CreDtTm>
    <NbOfTxs>1</NbOfTxs>
    <SttlmInf>
      <SttlmMtd>CLRG</SttlmMtd>
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
        <BICFI>{creditor_agent_bic}</BICFI>
      </FinInstnId>
    </InstdAgt>
  </GrpHdr>
  <CdtTrfTxInf>
    <PmtId>
      <InstrId>{instruction_id}</InstrId>
      <EndToEndId>{end_to_end_id}</EndToEndId>
      <UETR>{uetr}</UETR>
    </PmtId>
    <IntrBkSttlmAmt Ccy="{currency}">{amount}</IntrBkSttlmAmt>
    <IntrBkSttlmDt>{settlement_date}</IntrBkSttlmDt>
    <ChrgBr>SHAR</ChrgBr> <!-- Shared charges -->
    <Dbtr>
      <Nm>{debtor_name}</Nm>
    </Dbtr>
    <DbtrAgt>
      <FinInstnId>
        <BICFI>{debtor_agent_bic}</BICFI>
      </FinInstnId>
    </DbtrAgt>
    <CdtrAgt>
      <FinInstnId>
        <BICFI>{creditor_agent_bic}</BICFI>
      </FinInstnId>
    </CdtrAgt>
    <Cdtr>
      <Nm>{creditor_name}</Nm>
    </Cdtr>
    <RmtInf>
      <Ustrd>{remittance_info}</Ustrd>
    </RmtInf>
  </CdtTrfTxInf>
</FIToFICstmrCdtTrf>
```

**Key Canonical → ISO Mappings:**
- `deltran_tx_id` → `PmtId.InstrId` (as "DELTRAN-{uuid}")
- `uetr` → `PmtId.UETR`
- `end_to_end_id` → `PmtId.EndToEndId` (preserve from pain.001)
- `net_amount` (from Clearing) → `IntrBkSttlmAmt`
- `settlement_date` → `IntrBkSttlmDt`
- `debtor_agent` → `DbtrAgt.FinInstnId.BICFI`
- `creditor_agent` → `CdtrAgt.FinInstnId.BICFI`

**pacs.002 Status Report:**
```xml
<FIToFIPmtStsRpt>
  <GrpHdr>
    <MsgId>DELTRAN-STATUS-{uuid}</MsgId>
    <CreDtTm>{timestamp}</CreDtTm>
  </GrpHdr>
  <TxInfAndSts>
    <OrgnlInstrId>{original_instruction_id}</OrgnlInstrId>
    <OrgnlEndToEndId>{original_end_to_end_id}</OrgnlEndToEndId>
    <TxSts>ACCP</TxSts> <!-- ACCP/ACTC/ACSC/RJCT -->
    <StsRsnInf>
      <Rsn>
        <Cd>AC01</Cd> <!-- Reason code if failed -->
      </Rsn>
    </StsRsnInf>
  </TxInfAndSts>
</FIToFIPmtStsRpt>
```

**Status Codes:**
- `ACCP` - AcceptedCustomerProfile (accepted)
- `ACTC` - AcceptedTechnicalValidation
- `ACSC` - AcceptedSettlementCompleted ← **SUCCESS**
- `RJCT` - Rejected

---

## 6. RISK ENGINE

**Role:** Validates limits before settlement

### Inbound Messages

| Message | Source | Priority | Action |
|---------|--------|----------|--------|
| Internal event | Clearing Engine | **P0** | Validate settlement limits |

### Outbound Messages

| Message | Destination | Priority | Trigger |
|---------|-------------|----------|---------|
| Internal event | Settlement Engine | **P0** | Approve/reject settlement |
| `pacs.002` (via Gateway) | Bank | **P0** | If rejected: status=RJCT |

### ISO Integration

**No direct ISO message consumption**, but Risk Engine influences:
- `pacs.002` status codes:
  - `AM04` - InsufficientFunds
  - `AM05` - Duplicate
  - `FOCR` - FollowingCancellationRequest
  - `LEGL` - LegalDecision

---

## 7. REPORTING ENGINE

**Role:** Generates statements, reports, analytics

### Inbound Messages

| Message | Source | Priority | Action |
|---------|--------|----------|--------|
| `pacs.028` | Gateway | P2 | Payment status query |
| `camt.060` | Gateway | P2 | Account reporting request |
| `remt.001` | Gateway | P3 | Process remittance details |

### Outbound Messages

| Message | Destination | Priority | Trigger |
|---------|-------------|----------|---------|
| `camt.053` | Gateway → Bank | **P1** | Daily statement generation |
| `camt.105` | Gateway → Bank | P2 | Fee report |
| `pain.002` | Gateway → Customer | **P0** | Payment status to customer |
| `pacs.002` | Gateway → Bank | **P0** | Payment status to bank |

### ISO Field Mapping

**camt.053 Statement Generation:**
```xml
<BkToCstmrStmt>
  <Stmt>
    <Id>{statement_id}</Id>
    <CreDtTm>{timestamp}</CreDtTm>
    <FrToDt>
      <FrDtTm>{from_date}</FrDtTm>
      <ToDtTm>{to_date}</ToDtTm>
    </FrToDt>
    <Acct>
      <Id>
        <IBAN>{emi_account_iban}</IBAN>
      </Id>
      <Ccy>{currency}</Ccy>
    </Acct>
    <Bal> <!-- Opening balance -->
      <Tp>
        <CdOrPrtry>
          <Cd>OPBD</Cd>
        </CdOrPrtry>
      </Tp>
      <Amt Ccy="{currency}">{opening_balance}</Amt>
      <CdtDbtInd>CRDT</CdtDbtInd>
      <Dt>
        <Dt>{from_date}</Dt>
      </Dt>
    </Bal>
    <Bal> <!-- Closing balance -->
      <Tp>
        <CdOrPrtry>
          <Cd>CLBD</Cd>
        </CdOrPrtry>
      </Tp>
      <Amt Ccy="{currency}">{closing_balance}</Amt>
      <CdtDbtInd>CRDT</CdtDbtInd>
      <Dt>
        <Dt>{to_date}</Dt>
      </Dt>
    </Bal>
    <Ntry> <!-- For each transaction -->
      <Amt Ccy="{currency}">{amount}</Amt>
      <CdtDbtInd>{CRDT/DBIT}</CdtDbtInd>
      <Sts>BOOK</Sts>
      <BookgDt>
        <Dt>{date}</Dt>
      </BookgDt>
      <ValDt>
        <Dt>{value_date}</Dt>
      </ValDt>
      <AcctSvcrRef>{reference}</AcctSvcrRef>
      <NtryDtls>
        <TxDtls>
          <Refs>
            <EndToEndId>{end_to_end_id}</EndToEndId>
            <InstrId>{instruction_id}</InstrId>
            <UETR>{uetr}</UETR>
          </Refs>
          <RltdPties>
            <Dbtr>
              <Nm>{debtor_name}</Nm>
            </Dbtr>
            <Cdtr>
              <Nm>{creditor_name}</Nm>
            </Cdtr>
          </RltdPties>
          <RmtInf>
            <Ustrd>{remittance}</Ustrd>
          </RmtInf>
        </TxDtls>
      </NtryDtls>
    </Ntry>
  </Stmt>
</BkToCstmrStmt>
```

---

## 8. COMPLIANCE ENGINE

**Role:** AML screening, KYC, investigations

### Inbound Messages

| Message | Source | Priority | Action |
|---------|--------|----------|--------|
| Internal event | Obligation Engine | **P0** | Screen payment before clearing |
| `camt.027` | Gateway | P2 | Handle claim non-receipt |
| `acmt.023` | Gateway | P2 | Verify account identification |

### Outbound Messages

| Message | Destination | Priority | Trigger |
|---------|-------------|----------|---------|
| Internal event | Clearing Engine | **P0** | Approve/block payment |
| `camt.029` | Gateway → Bank | P2 | Investigation resolution |
| `acmt.024` | Gateway → Bank | P2 | ID verification result |

### ISO Integration

**Compliance influences pacs.002 rejection codes:**
- `FF01` - InvalidFileFormat
- `AM09` - WrongAmount (suspicious pattern)
- `NOAS` - NoAnswerFromCustomer
- `LEGL` - LegalDecision (sanctions hit)

---

## 9. NOTIFICATION ENGINE

**Role:** Sends notifications to customers/banks

### Inbound Messages

| Message | Source | Priority | Action |
|---------|--------|----------|--------|
| Internal event | Settlement Engine | **P0** | Settlement completed |
| Internal event | Obligation Engine | **P0** | Payment status changed |

### Outbound Messages

| Message | Destination | Priority | Trigger |
|---------|-------------|----------|---------|
| `pain.002` | Gateway → Customer | **P0** | Payment status notification |
| `camt.057` | Gateway → Bank | P2 | Notification to receive |
| Email/SMS | Customer | **P0** | Via external channels |

---

## 10. LIQUIDITY ROUTER

**Role:** Optimizes liquidity usage across corridors

### ISO Integration

**Indirect:** Uses data from:
- `camt.054` funding events (available balance)
- `camt.053` statements (projected balances)
- `pacs.029` net positions (settlement obligations)

**No direct ISO message production**, but influences:
- Which settlements to prioritize
- When to request additional liquidity (future: `camt.050`)

---

## 11. ANALYTICS / TRACKING SERVICE

**Role:** Payment tracking, reporting, analytics

### Inbound Messages

| Message | Source | Priority | Action |
|---------|--------|----------|--------|
| `trck.001` | Gateway | P3 | Payment tracking request |
| `trck.004` | Gateway | P3 | Tracking update |

### Outbound Messages

| Message | Destination | Priority | Trigger |
|---------|-------------|----------|---------|
| `trck.002` | Gateway → Bank | P3 | Tracking status report |

### ISO Field Mapping

**trck.001 → Query:**
- `PmtId.UETR` → Lookup transaction by UETR
- Return current status + history

**trck.002 Response:**
```xml
<PmtTrckgStsRpt>
  <OrgnlPmtId>
    <UETR>{uetr}</UETR>
  </OrgnlPmtId>
  <TxSts>ACSC</TxSts> <!-- Current status -->
  <PrcgDtTms> <!-- Processing timestamps -->
    <AccptncDtTm>{acceptance_time}</AccptncDtTm>
    <FnlSttlmDtTm>{settlement_time}</FnlSttlmDtTm>
  </PrcgDtTms>
</PmtTrckgStsRpt>
```

---

## Priority Matrix

### P0 - MVP MUST HAVE

| Service | Message In | Message Out | Purpose |
|---------|-----------|-------------|---------|
| Gateway | pain.001 | pain.002 | Customer payment in |
| Gateway | pacs.008 | pacs.002 | Bank payment in/out |
| Gateway | camt.054 | - | Funding event |
| Token Engine | camt.054 | - | Mint tokens |
| Obligation | pain.001 | - | Create obligation |
| Clearing | - | - | Netting (internal) |
| Settlement | - | pacs.008 | Send payment |
| Settlement | - | pacs.002 | Confirm status |

### P1 - HIGH VALUE

| Service | Message In | Message Out | Purpose |
|---------|-----------|-------------|---------|
| Gateway | camt.053 | - | EOD reconciliation |
| Gateway | pacs.004 | pacs.004 | Payment returns |
| Gateway | camt.055/056 | - | Cancellations |
| Token Engine | camt.053 | - | Reconcile balances |
| Clearing | - | pacs.029 | Multilateral netting |
| Reporting | - | camt.053 | Generate statements |

### P2 - IMPORTANT

| Service | Message In | Message Out | Purpose |
|---------|-----------|-------------|---------|
| Gateway | acmt.007-010 | acmt.011 | Account management |
| Gateway | camt.027 | camt.029 | Investigations |
| Gateway | camt.057 | - | Pre-advice |
| Settlement | camt.055/056 | - | Handle cancellations |
| Reporting | - | camt.105 | Fee reports |
| Compliance | camt.027 | camt.029 | Investigations |

### P3 - NICE TO HAVE

| Service | Message In | Message Out | Purpose |
|---------|-----------|-------------|---------|
| Gateway | trck.001 | trck.002 | Payment tracking |
| Gateway | remt.001 | - | Remittance data |
| Analytics | trck.001 | trck.002 | Tracking service |

---

## Message Flow Examples

### Example 1: Successful Payment (Pain → Pacs)

```
1. Bank → pain.001 → Gateway
2. Gateway → [Canonical] → Obligation Engine
3. Obligation creates record (status=PENDING_FUNDING)
4. Gateway → pain.002 (ACCP - accepted) → Bank

[Later: Funding arrives]
5. Bank → camt.054 → Gateway → Token Engine
6. Token Engine mints tokens, updates emi_accounts.balance
7. Token Engine → [event] → Obligation Engine
8. Obligation matches funding (status=READY_FOR_CLEARING)
9. Obligation → [event] → Clearing Engine
10. Clearing nets payments
11. Clearing → [net obligations] → Settlement Engine
12. Settlement → pacs.008 → Gateway → Beneficiary Bank
13. Settlement → pacs.002 (ACSC - completed) → Gateway → Originating Bank
```

### Example 2: Payment Return

```
1. Bank → pacs.004 (return) → Gateway
2. Gateway → [Canonical] → Settlement Engine
3. Settlement reverses internal balances
4. Settlement → [event] → Token Engine
5. Token Engine burns tokens, updates emi_accounts.balance
6. Settlement → pacs.002 (RJCT with reason) → Gateway → Bank
```

### Example 3: Cancellation Request

```
1. Bank → camt.056 (cancel request) → Gateway
2. Gateway → Settlement Engine
3. Settlement checks if payment already settled:
   - If not settled: Cancel, return pacs.002 (CANC)
   - If settled: Initiate pacs.004 return flow
4. Settlement → pacs.002 or pacs.004 → Gateway → Bank
```

---

## Canonical Model Core Fields

**Unified internal representation:**

```rust
pub struct CanonicalPayment {
    // DelTran IDs
    pub deltran_tx_id: Uuid,
    pub obligation_id: Option<Uuid>,
    pub clearing_batch_id: Option<Uuid>,
    pub settlement_id: Option<Uuid>,

    // ISO Identifiers
    pub uetr: Option<Uuid>,
    pub end_to_end_id: String,
    pub instruction_id: String,
    pub message_id: String,

    // Amount & Currency
    pub instructed_amount: Decimal,
    pub settlement_amount: Decimal,
    pub currency: Currency,
    pub exchange_rate: Option<Decimal>,

    // Parties
    pub debtor: Party,
    pub creditor: Party,
    pub debtor_agent: FinancialInstitution,
    pub creditor_agent: FinancialInstitution,
    pub debtor_account: AccountIdentification,
    pub creditor_account: AccountIdentification,

    // Dates
    pub creation_date: DateTime<Utc>,
    pub requested_execution_date: Option<NaiveDate>,
    pub settlement_date: Option<NaiveDate>,
    pub value_date: Option<NaiveDate>,

    // Status
    pub status: PaymentStatus,
    pub status_reason: Option<StatusReason>,

    // Charges
    pub charge_bearer: ChargeBearer, // SHAR/SLEV/DEBT/CRED
    pub charges: Vec<Charge>,

    // Remittance
    pub remittance_info: String,
    pub remittance_structured: Option<StructuredRemittance>,

    // Risk & Compliance
    pub risk_score: Option<f64>,
    pub compliance_status: ComplianceStatus,
    pub sanctions_checked: bool,

    // Corridor & Routing
    pub corridor: String,
    pub priority: Priority,
    pub liquidity_pool_id: Option<Uuid>,
}
```

---

## Next Steps

1. ✅ Created service-to-message mapping
2. 🔄 **NEXT:** Extract MDR Part1 summaries for business context
3. 🔄 Build detailed message catalog with field descriptions
4. 🔄 Generate XSD → Rust/JSON validation schemas
5. 🔄 Write Canonical ↔ ISO transformation logic
6. 🔄 Create integration test suite with ISO message examples

---

*Generated: 2025-11-18*
*Services: 11 | Messages Mapped: 40+ | Priority: P0-P3*
