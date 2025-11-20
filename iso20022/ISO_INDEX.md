# ISO 20022 Archives Index

## Overview

This document provides a complete navigation guide for all ISO 20022 archives extracted for the DelTran MVP project.

**Summary Statistics:**
- **Total Archives:** 18
- **Total XSD Schemas:** 136 message definitions
- **MDR Part1 Files:** 11 (Business Process Descriptions)
- **MDR Part2 Files:** 12 (Message Structure Specifications)
- **MDR Part3 Files:** 11 (Data Dictionaries)

---

## Archive Categories & DelTran Mapping

### 1. PAYMENTS INITIATION

**Archive:** `payments_initiation/`

**Purpose:** Customer/Bank-initiated payment instructions entering DelTran Gateway

**Contents:**
- **XSD Files:** 4
- **MDR Files:** None (schemas only)

**Message Types:**
- `pain.001.001.12` - CustomerCreditTransferInitiation
- `pain.002.001.14` - CustomerPaymentStatusReport
- `pain.007.001.12` - CustomerPaymentReversal
- `pain.008.001.11` - CustomerDirectDebitInitiation

**DelTran Services:**
- **Gateway** - Primary entry point for payment instructions
- **Obligation Engine** - Processes payment obligations from pain.001
- **Notification Engine** - Sends pain.002 status reports to customers

**Use Cases:**
- Banks/corporates submit payment requests to DelTran
- DelTran validates and converts to internal Canonical Model
- Status reports sent back via pain.002

---

### 2. PAYMENTS CLEARING AND SETTLEMENT

**Archive:** `payments_clearing_and_settlement/`

**Purpose:** Core FI-to-FI payment processing - heart of DelTran settlement

**Contents:**
- **XSD Files:** 8
- **MDR Files:** Part1, Part2, Part3 (complete documentation)

**Message Types:**
- `pacs.002.001.15` - FIToFIPaymentStatusReport (CRITICAL for settlement status)
- `pacs.003.001.11` - FIToFICustomerDirectDebit
- `pacs.004.001.14` - PaymentReturn
- `pacs.007.001.13` - FIToFIPaymentReversal
- `pacs.008.001.13` - FIToFICustomerCreditTransfer (MOST IMPORTANT)
- `pacs.009.001.12` - FinancialInstitutionCreditTransfer
- `pacs.010.001.06` - FIToFICustomerDirectDebitRequest
- `pacs.028.001.06` - FIToFIPaymentStatusRequest

**DelTran Services:**
- **Settlement Engine** - pacs.008/009 for actual settlement instructions
- **Clearing Engine** - Processes and nets pacs.008 before settlement
- **Gateway** - Receives pacs.008 from banks for incoming payments
- **Reporting Engine** - Generates pacs.002 status reports
- **Risk Engine** - Validates limits before processing pacs.008

**Critical Flows:**
1. **Incoming:** Bank → pacs.008 → DelTran Gateway → Clearing → Settlement
2. **Outgoing:** DelTran Settlement → pacs.008 → Bank
3. **Status:** DelTran → pacs.002 → Bank
4. **Returns:** Bank/DelTran → pacs.004 → counterparty

---

### 3. MULTILATERAL SETTLEMENT

**Archive:** `multilateral_settlement/`

**Purpose:** Multilateral netting calculations for Clearing Engine

**Contents:**
- **XSD Files:** 3
- **MDR Files:** Part1, Part2, Part3

**Message Types:**
- `pacs.029.001.02` - MultilateralSettlementRequest (CORE FOR NETTING)
- `pacs.002.001.12` - FIToFIPaymentStatusReport (older version)
- `admi.004.001.02` - SystemEventNotification

**DelTran Services:**
- **Clearing Engine** - Uses pacs.029 for multilateral netting algorithm
- **Settlement Engine** - Receives net settlement obligations from pacs.029

**Use Case:**
- After bilateral netting, Clearing Engine generates pacs.029
- Contains net positions for each participant
- Settlement Engine executes final settlements based on net amounts

---

### 4. CASH MANAGEMENT

**Archive:** `cash_management/`

**Purpose:** Bank-to-Bank cash operations, liquidity management, account operations

**Contents:**
- **XSD Files:** 35
- **MDR Files:** Part1, Part2, Part3

**Key Message Types:**
- `camt.003` - GetAccount
- `camt.004` - ReturnAccount
- `camt.005` - AccountReportingRequest
- `camt.006` - AccountReportingResponse
- `camt.007` - RequestToModifyPayment
- `camt.008` - CancellationStatusReport
- `camt.009` - AccountQuery
- `camt.010` - AccountResponse
- `camt.025` - Receipt
- `camt.046` - NotificationOfCancellation
- `camt.047` - CancellationRequestAcknowledgement
- `camt.048` - CancellationDenied
- Plus 23 more...

**DelTran Services:**
- **Liquidity Router** - Monitors available liquidity via camt.003/004
- **Settlement Engine** - Uses camt.025 for receipts
- **Gateway** - Handles modification/cancellation requests (camt.007/046)
- **Reporting Engine** - Account queries and responses

---

### 5. BANK-TO-CUSTOMER CASH MANAGEMENT

**Archive:** `banktocustomer_cash_management/`

**Purpose:** CRITICAL - Real funding events trigger for DelTran tokenization

**Contents:**
- **XSD Files:** 4
- **MDR Files:** Part1, Part2, Part3

**Message Types:**
- `camt.052.001.13` - BankToCustomerAccountReport (intraday statement)
- `camt.053.001.13` - BankToCustomerStatement (EOD statement) ⭐
- `camt.054.001.13` - BankToCustomerDebitCreditNotification ⭐⭐
- `camt.060.001.07` - AccountReportingRequest

**DelTran Services:**
- **Token Engine** - PRIMARY TRIGGER via camt.054 for funding events
- **Obligation Engine** - Matches camt.054 to pending obligations
- **Reconciliation** - EOD reconciliation using camt.053
- **Reporting Engine** - Customer statement generation

**Critical Flow:**
1. Bank sends camt.054 → DelTran receives real money notification
2. Token Engine mints internal tokens (updates `emi_accounts.balance`)
3. Obligation Engine releases matched payment for settlement
4. EOD: Bank sends camt.053 → DelTran reconciles daily positions

**Priority:** HIGHEST - This is how DelTran knows money has actually arrived!

---

### 6. BANK ACCOUNT MANAGEMENT

**Archive:** `bank_account_management/`

**Purpose:** Lifecycle of EMI accounts in DelTran system

**Contents:**
- **XSD Files:** 15
- **MDR Files:** Part1, Part2, Part3

**Message Types:**
- `acmt.007` - AccountOpeningInstruction
- `acmt.008` - AccountOpeningAmendmentRequest
- `acmt.009` - AccountModificationRequest
- `acmt.010` - AccountClosureRequest
- `acmt.011` - AccountStatusReport
- `acmt.012` - AccountManagementResponse
- Plus 9 more acmt.013-021

**DelTran Services:**
- **Token Engine** - Manages `emi_accounts` table lifecycle
- **Gateway** - Receives account management requests
- **Compliance Engine** - Validates account opening/KYC requirements

**Use Cases:**
- Bank opens EMI account for DelTran → acmt.007
- DelTran creates record in `emi_accounts` table
- Account modifications → acmt.009
- Account status queries → acmt.011

---

### 7. ACCOUNT MANAGEMENT (Business Area)

**Archive:** `account_management_business_area/`

**Purpose:** Extended account management message set (34 schemas)

**Contents:**
- **XSD Files:** 34 (complete acmt.* message set)
- **MDR Files:** None

**Message Types:** acmt.001 through acmt.034+
- Includes all bank_account_management messages PLUS:
- `acmt.001` - AccountDetailsConfirmation
- `acmt.002` - AccountReport
- `acmt.003` - AccountModificationRequest (extended)
- `acmt.005` - RequestForAccountManagementStatusReport
- `acmt.022-024` - IdentificationModification/Verification (see separate archive)
- Plus many more...

**DelTran Services:**
- Supplements bank_account_management archive
- Used for extended account features beyond MVP scope
- Future: Corporate account hierarchies, mandates, etc.

---

### 8. CHANGE/VERIFY ACCOUNT IDENTIFICATION

**Archive:** `changeverify_account_identification/`

**Purpose:** IBAN/BBAN verification and modification

**Contents:**
- **XSD Files:** 3
- **MDR Files:** Part2, Part3 (no Part1)

**Message Types:**
- `acmt.022.001.04` - IdentificationModificationAdvice
- `acmt.023.001.04` - IdentificationVerificationRequest
- `acmt.024.001.04` - IdentificationVerificationReport

**DelTran Services:**
- **Token Engine** - Validates account identifiers in `emi_accounts`
- **Gateway** - Handles account identifier changes
- **Compliance Engine** - Verifies account ownership

**Use Case:**
- Bank changes IBAN for existing EMI account → acmt.022
- DelTran verifies account before settlement → acmt.023/024

---

### 9. CHARGES MANAGEMENT

**Archive:** `charges_management/`

**Purpose:** Explicit fee reporting and transparency

**Contents:**
- **XSD Files:** 2
- **MDR Files:** Part1, Part2, Part3

**Message Types:**
- `camt.105.001.03` - ChargesReport
- `camt.106.001.03` - ChargesRefundRequest

**DelTran Services:**
- **Reporting Engine** - Generates camt.105 for fee transparency
- **Settlement Engine** - Includes fee details in settlements
- **Gateway** - Handles refund requests (camt.106)

**Use Case:**
- DelTran charges corridor fee → generates camt.105 to bank
- Bank disputes fee → sends camt.106 refund request

---

### 10. EXCEPTIONS AND INVESTIGATIONS

**Archive:** `exceptions_investigations/`

**Purpose:** Payment failures, returns, disputes, investigations

**Contents:**
- **XSD Files:** 17
- **MDR Files:** Part1, Part2, Part3

**Key Message Types:**
- `camt.026` - UnableToApply (payment can't be processed)
- `camt.027` - ClaimNonReceipt
- `camt.028` - AdditionalPaymentInformation
- `camt.029` - ResolutionOfInvestigation
- `camt.055` - CustomerPaymentCancellationRequest ⭐
- `camt.056` - FIToFIPaymentCancellationRequest ⭐
- `camt.087` - RequestToModifyPayment
- Plus 10 more...

**DelTran Services:**
- **Gateway** - Receives cancellation requests (camt.055/056)
- **Settlement Engine** - Handles payment returns and reversals
- **Compliance Engine** - Manages investigations (camt.027/029)
- **Reporting Engine** - Generates investigation reports

**Critical Flows:**
1. **Cancel before settlement:** Customer → camt.055 → DelTran stops payment
2. **Cancel after settlement:** Bank → camt.056 → DelTran initiates return
3. **Investigation:** Bank → camt.027 → DelTran → camt.029 resolution

---

### 11. EXCEPTIONS MODERNISATION

**Archive:** `exceptions_investigations_modernisation/`

**Purpose:** New-generation exception handling (ISO 20022 2024+ edition)

**Contents:**
- **XSD Files:** 2
- **MDR Files:** Part1, Part2, Part3

**Message Types:**
- `camt.110.001.01` - RequestForPaymentStatusUpdate (NEW)
- `camt.111.001.02` - PaymentStatusUpdate (NEW)

**DelTran Services:**
- **Tracking Service** - Real-time status updates
- **Gateway** - Responds to status requests
- **Notification Engine** - Push updates via camt.111

**Use Case:**
- Modern alternative to legacy exception messages
- Used for real-time payment tracking (complement to trck.*)

---

### 12. NOTIFICATION TO RECEIVE

**Archive:** `notification_to_receive/`

**Purpose:** Pre-advice notifications - "expect incoming payment"

**Contents:**
- **XSD Files:** 3
- **MDR Files:** Part1, Part2, Part3

**Message Types:**
- `camt.057.001.08` - NotificationToReceive (pre-advice)
- `camt.058.001.09` - NotificationToReceiveCancellationAdvice
- `camt.059.001.08` - NotificationToReceiveStatusReport

**DelTran Services:**
- **Obligation Engine** - Creates "expected funding" record from camt.057
- **Token Engine** - Prepares for incoming camt.054 based on camt.057
- **Notification Engine** - Sends pre-advice to beneficiary banks

**Use Case:**
1. DelTran settlement scheduled → sends camt.057 to beneficiary bank
2. Bank prepares to receive payment
3. Actual payment arrives via pacs.008
4. Confirmation via camt.054

---

### 13. NOTIFICATION OF CORRESPONDENCE

**Archive:** `notification_of_correspondence/`

**Purpose:** Service messages, system notifications

**Contents:**
- **XSD Files:** 1
- **MDR Files:** Part1, Part2 (no Part3)

**Message Types:**
- `admi.024.001.01` - StaticDataRequest

**DelTran Services:**
- **Gateway** - Handles service/administrative messages
- **Reporting Engine** - Static data queries

**Priority:** LOW - mainly for operational/administrative communication

---

### 14. PAYMENT TRACKING

**Archive:** `payment_tracking/`

**Purpose:** SWIFT gpi-style tracking - end-to-end transaction visibility

**Contents:**
- **XSD Files:** 3
- **MDR Files:** Part1 (as docx), Part2, Part3

**Message Types:**
- `trck.001.001.04` - PaymentTrackingRequest
- `trck.002.001.03` - PaymentTrackingStatusReport
- `trck.004.001.03` - PaymentTrackingUpdateRequest

**DelTran Services:**
- **Tracking/Analytics Service** - Primary consumer
- **Gateway** - Handles tracking API requests
- **Reporting Engine** - Generates tracking reports

**Use Case:**
- Customer queries payment status → trck.001
- DelTran returns current state → trck.002
- External status update → trck.004

**Integration:**
- Links to UETR (Universal Transaction Reference) in pacs.008
- Provides transparency like SWIFT gpi tracker

---

### 15. REMITTANCE ADVICE

**Archive:** `remittance_advice/`

**Purpose:** Detailed payment remittance information (invoices, references)

**Contents:**
- **XSD Files:** 2
- **MDR Files:** None

**Message Types:**
- `remt.001.001.06` - RemittanceAdvice
- `remt.002.001.03` - RemittanceAdviceNotification

**DelTran Services:**
- **Gateway** - Separates remittance data from payment instruction
- **Reporting Engine** - Delivers detailed remittance to beneficiary
- **Integration Layer** - Links to ERP/accounting systems

**Use Case:**
- Corporate payment with invoice details → payment via pain.001 + remittance via remt.001
- DelTran forwards remittance separately for recipient reconciliation

---

## Message Type Quick Reference

### By Prefix

| Prefix | Domain | Count | Description |
|--------|--------|-------|-------------|
| `pain.*` | Payments Initiation | 4 | Customer/Bank payment instructions |
| `pacs.*` | Payments Clearing & Settlement | 11 | FI-to-FI payment messages |
| `camt.*` | Cash Management | ~60 | Account statements, notifications, investigations |
| `acmt.*` | Account Management | 34+ | Account lifecycle operations |
| `remt.*` | Remittance | 2 | Detailed payment remittance info |
| `trck.*` | Tracking | 3 | Payment tracking/status |
| `admi.*` | Administration | 2 | System/service messages |

### Most Critical for DelTran MVP

**Priority 1 - MUST IMPLEMENT:**
1. `pacs.008` - FI-to-FI Credit Transfer (core settlement)
2. `pacs.002` - Payment Status Report
3. `camt.054` - Debit/Credit Notification (FUNDING TRIGGER)
4. `pain.001` - Customer Credit Transfer (inbound instructions)
5. `pain.002` - Payment Status Report (customer notification)

**Priority 2 - HIGH VALUE:**
6. `camt.053` - Bank Statement (EOD reconciliation)
7. `pacs.004` - Payment Return
8. `camt.055/056` - Payment Cancellation Requests
9. `acmt.007-012` - Account Management (EMI lifecycle)
10. `pacs.029` - Multilateral Settlement (netting)

**Priority 3 - POST-MVP:**
11. `trck.*` - Payment Tracking
12. `remt.*` - Remittance Advice
13. `camt.026-029` - Investigation messages
14. `camt.057` - Notification To Receive
15. Extended acmt.*, camt.* for full features

---

## Directory Structure

```
iso20022/
├── ISO_INDEX.md (this file)
├── ARCHIVE_SCAN.md (detailed file listing)
│
├── payments_initiation/
│   └── pain.*.xsd (4 files)
│
├── payments_clearing_and_settlement/
│   ├── MDR Part1/2/3
│   └── pacs.*.xsd (8 files)
│
├── multilateral_settlement/
│   ├── MDR Part1/2/3
│   └── pacs.029, admi.004 (3 files)
│
├── cash_management/
│   ├── MDR Part1/2/3
│   └── camt.*.xsd (35 files)
│
├── banktocustomer_cash_management/
│   ├── MDR Part1/2/3
│   └── camt.052-054, camt.060 (4 files)
│
├── bank_account_management/
│   ├── MDR Part1/2/3
│   └── acmt.007-021 (15 files)
│
├── account_management_business_area/
│   └── acmt.*.xsd (34 files)
│
├── changeverify_account_identification/
│   ├── MDR Part2/3
│   └── acmt.022-024 (3 files)
│
├── charges_management/
│   ├── MDR Part1/2/3
│   └── camt.105-106 (2 files)
│
├── exceptions_investigations/
│   ├── MDR Part1/2/3
│   └── camt.026-087 (17 files)
│
├── exceptions_investigations_modernisation/
│   ├── MDR Part1/2/3
│   └── camt.110-111 (2 files)
│
├── notification_to_receive/
│   ├── MDR Part1/2/3
│   └── camt.057-059 (3 files)
│
├── notification_of_correspondence/
│   ├── MDR Part1/2
│   └── admi.024 (1 file)
│
├── payment_tracking/
│   ├── MDR Part1/2/3
│   └── trck.*.xsd (3 files)
│
└── remittance_advice/
    └── remt.*.xsd (2 files)
```

---

## Next Steps

1. ✅ Archives extracted and indexed
2. 🔄 **NEXT:** Create `DELTRAN_ISO_MAPPING.md` - detailed service-by-service message mapping
3. 🔄 Extract MDR Part1 summaries → `summary/` folder
4. 🔄 Build message catalog with detailed descriptions → `iso_message_catalog.json`
5. 🔄 Extract data dictionaries from Part3 → `dictionaries/*.json`
6. 🔄 Create Canonical Model mapping → `DELTRAN_CANONICAL_MAPPING.md`
7. 🔄 Generate validation schemas and code types from XSD
8. 🔄 Write master implementation plan → `DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md`

---

## Related Documentation

- [ARCHIVE_SCAN.md](ARCHIVE_SCAN.md) - Complete file listing by archive
- [DELTRAN_ISO_MAPPING.md](DELTRAN_ISO_MAPPING.md) - Service-to-message mapping (TO BE CREATED)
- [DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md](DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md) - Master implementation guide (TO BE CREATED)

---

*Generated: 2025-11-18*
*Total Archives: 18 | Total Messages: 136*
