# ISO 20022 Integration for DelTran MVP

## Overview

This directory contains all ISO 20022 standard messaging schemas and documentation required for DelTran's integration with banks and financial institutions.

**Status:** ✅ All archives extracted and mapped
**Total Messages:** 136 XSD schemas across 18 business areas
**Documentation:** Complete MDR (Message Definition Reports) for 11 key areas

---

## Quick Start

### For Developers

**New to this project? Start here:**

1. **Read the master plan:** [`DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md`](DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md)
   - Complete roadmap with phases, timelines, priorities
   - Technical architecture and code examples
   - Testing strategy and success criteria

2. **Understand the mapping:** [`DELTRAN_ISO_MAPPING.md`](DELTRAN_ISO_MAPPING.md)
   - Which ISO messages each DelTran service uses
   - Message flow patterns and integration points
   - Canonical model mapping

3. **Browse the catalog:** [`iso_message_catalog.json`](iso_message_catalog.json)
   - Quick reference for all 136 messages
   - Priority levels (P0-P3)
   - Service assignments and use cases

4. **Navigate the archives:** [`ISO_INDEX.md`](ISO_INDEX.md)
   - Detailed breakdown of all 18 archives
   - File locations and MDR documentation
   - Business area descriptions

### For Business/Product Teams

**Understanding ISO 20022 for DelTran:**

- **What is ISO 20022?** International standard for financial messaging (XML-based)
- **Why do we need it?** Banks require ISO 20022 for interoperability
- **Key messages for DelTran:**
  - `pain.001` - Customer payment request
  - `camt.054` - Funding notification (CRITICAL)
  - `pacs.008` - Bank-to-bank settlement
  - `camt.053` - Daily statement (reconciliation)

**See:** [ISO_INDEX.md](ISO_INDEX.md) sections 1-15 for business context

---

## Directory Structure

```
iso20022/
│
├── README.md (this file)
├── ISO_INDEX.md (complete navigation guide)
├── DELTRAN_ISO_MAPPING.md (service-to-message mapping)
├── DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md (master implementation plan)
├── iso_message_catalog.json (quick reference catalog)
├── ARCHIVE_SCAN.md (detailed file listing)
│
├── payments_initiation/ (pain.*)
│   ├── pain.001.001.12.xsd
│   ├── pain.002.001.14.xsd
│   ├── pain.007.001.12.xsd
│   └── pain.008.001.11.xsd
│
├── payments_clearing_and_settlement/ (pacs.*)
│   ├── ISO20022_MDRPart1_*.docx (business processes)
│   ├── ISO20022_MDRPart2_*.pdf (message structures)
│   ├── ISO20022_MDRPart3_*.xlsx (data dictionary)
│   ├── pacs.002.001.15.xsd
│   ├── pacs.004.001.14.xsd
│   ├── pacs.008.001.13.xsd ⭐ CRITICAL
│   └── ... (5 more)
│
├── banktocustomer_cash_management/ (camt.052-054)
│   ├── ISO20022_MDRPart1_*.docx
│   ├── ISO20022_MDRPart2_*.pdf
│   ├── ISO20022_MDRPart3_*.xlsx
│   ├── camt.052.001.13.xsd
│   ├── camt.053.001.13.xsd ⭐ CRITICAL
│   └── camt.054.001.13.xsd ⭐⭐ MOST CRITICAL
│
├── cash_management/ (camt.003-104)
│   └── ... (35 camt.* schemas)
│
├── multilateral_settlement/
│   ├── ISO20022_MDRPart1/2/3_*.{docx,pdf,xlsx}
│   ├── pacs.029.001.02.xsd ⭐ Netting
│   └── admi.004.001.02.xsd
│
├── bank_account_management/ (acmt.007-021)
│   ├── ISO20022_MDRPart1/2/3_*.{docx,pdf,xlsx}
│   └── acmt.*.xsd (15 schemas)
│
├── exceptions_investigations/ (camt.026-087)
│   ├── ISO20022_MDRPart1/2/3_*.{docx,pdf,xlsx}
│   ├── camt.055.001.12.xsd (cancellation)
│   ├── camt.056.001.11.xsd (cancellation)
│   └── ... (15 more)
│
├── payment_tracking/ (trck.*)
│   ├── ISO20022_MDRPart1/2/3_*.{docx,pdf,xlsx}
│   └── trck.*.xsd (3 schemas)
│
├── charges_management/ (camt.105-106)
│   ├── ISO20022_MDRPart1/2/3_*.{docx,pdf,xlsx}
│   └── camt.10*.xsd (2 schemas)
│
├── remittance_advice/ (remt.*)
│   └── remt.*.xsd (2 schemas)
│
└── ... (9 more archives)
```

---

## Key Files Explained

### Master Documentation

| File | Purpose | Audience |
|------|---------|----------|
| **DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md** | Complete implementation roadmap | Dev team, PM |
| **DELTRAN_ISO_MAPPING.md** | Service-to-message mapping | Dev team |
| **ISO_INDEX.md** | Archive navigation guide | Everyone |
| **iso_message_catalog.json** | Quick reference catalog | Dev team, Tools |
| **ARCHIVE_SCAN.md** | Raw file inventory | Reference |

### XSD Schemas

**What are they?**
XML Schema Definition files that define the structure, constraints, and validation rules for each ISO 20022 message type.

**How to use:**
- Load into XML parsers for validation
- Generate code types (structs) from XSD
- Reference for understanding message structure

**Example:** `pacs.008.001.13.xsd` defines the structure of FI-to-FI credit transfer messages.

### MDR Files (Message Definition Reports)

**Part1 (`.docx`):** Business process descriptions
- Roles and participants
- Business scenarios and use cases
- When to use which messages

**Part2 (`.pdf`):** Technical message specifications
- Detailed element descriptions
- Business rules and constraints
- Code lists and allowed values
- Usage guidelines

**Part3 (`.xlsx`):** Data dictionaries
- Field definitions
- Data types and formats
- Validation constraints
- Code sets

---

## Priority Messages for MVP

### P0 - MUST IMPLEMENT FIRST (Weeks 1-4)

| Message | Name | Critical For | Service |
|---------|------|-------------|---------|
| `pain.001` | CustomerCreditTransferInitiation | Payment entry point | Gateway → Obligation |
| `pain.002` | CustomerPaymentStatusReport | Customer notifications | Notification → Gateway |
| `pacs.008` | FIToFICustomerCreditTransfer | Core settlements | Clearing/Settlement |
| `pacs.002` | FIToFIPaymentStatusReport | Settlement status | Settlement → Gateway |
| `camt.054` | DebitCreditNotification | **FUNDING TRIGGER** | Gateway → Token Engine |

**⚠️ camt.054 is THE MOST CRITICAL MESSAGE** - This is how DelTran knows real money arrived!

### P1 - HIGH VALUE (Weeks 5-6)

| Message | Name | Critical For | Service |
|---------|------|-------------|---------|
| `camt.053` | BankToCustomerStatement | EOD reconciliation | Token Engine |
| `pacs.004` | PaymentReturn | Handle failures | Settlement Engine |
| `camt.055/056` | CancellationRequest | Cancellations | Settlement Engine |
| `pacs.029` | MultilateralSettlementRequest | Netting | Clearing Engine |

### P2 - IMPORTANT (Weeks 7-8)

- Account management (`acmt.007-011`)
- Investigations (`camt.027`, `camt.029`)
- Pre-advice (`camt.057`)
- Fee reporting (`camt.105`)

### P3 - NICE TO HAVE (Weeks 9-12)

- Payment tracking (`trck.*`)
- Remittance advice (`remt.*`)
- Modern exceptions (`camt.110/111`)

---

## Message Flow Examples

### Example 1: Successful Payment

```
1. Bank sends pain.001 (payment request)
   ↓
2. Gateway validates → creates obligation (PENDING_FUNDING)
   ↓
3. Gateway sends pain.002 (ACCP - accepted)
   ↓
[Time passes...]
   ↓
4. Bank sends camt.054 (money arrived!) ⭐
   ↓
5. Token Engine mints tokens, updates emi_accounts.balance
   ↓
6. Obligation Engine matches funding → READY_FOR_CLEARING
   ↓
7. Clearing Engine nets payments
   ↓
8. Settlement Engine sends pacs.008 to beneficiary bank
   ↓
9. Settlement sends pacs.002 (ACSC - completed) to originating bank
```

### Example 2: Payment Return

```
1. Bank sends pacs.004 (return rejected payment)
   ↓
2. Settlement Engine processes return
   ↓
3. Token Engine burns tokens, reverses balance
   ↓
4. Settlement sends pacs.002 (RJCT with reason)
```

---

## Using the XSD Schemas

### Validation

**Rust example:**
```rust
use quick_xml::Reader;
use xmlschema::XMLSchema;

fn validate_iso_message(xml: &str, message_type: &str) -> Result<(), ValidationError> {
    let xsd_path = format!("iso20022/payments_clearing_and_settlement/{}.xsd", message_type);
    let schema = XMLSchema::from_file(&xsd_path)?;
    schema.validate(xml)?;
    Ok(())
}

// Usage
let pain001_xml = r#"<?xml version="1.0"?>...</Document>"#;
validate_iso_message(pain001_xml, "pain.001.001.12")?;
```

**Go example:**
```go
import "github.com/lestrrat-go/libxml2/xsd"

func ValidateISOMessage(xmlData []byte, messageType string) error {
    xsdPath := fmt.Sprintf("iso20022/payments_clearing_and_settlement/%s.xsd", messageType)
    schema, err := xsd.ParseFromFile(xsdPath)
    if err != nil {
        return err
    }
    defer schema.Free()

    return schema.Validate(xmlData)
}
```

### Parsing

**Rust example:**
```rust
use serde::{Deserialize, Serialize};
use quick_xml::de::from_str;

#[derive(Debug, Deserialize, Serialize)]
pub struct Pain001 {
    #[serde(rename = "GrpHdr")]
    pub group_header: GroupHeader,

    #[serde(rename = "PmtInf")]
    pub payment_information: Vec<PaymentInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GroupHeader {
    #[serde(rename = "MsgId")]
    pub message_id: String,

    #[serde(rename = "CreDtTm")]
    pub creation_date_time: String,
}

// Usage
let pain001: Pain001 = from_str(&xml_string)?;
println!("Message ID: {}", pain001.group_header.message_id);
```

---

## Common Tasks

### Task 1: Find which message to use for a scenario

**Example:** "I need to notify a bank that money is coming"

1. Check [`DELTRAN_ISO_MAPPING.md`](DELTRAN_ISO_MAPPING.md) section "Notification To Receive"
2. Result: `camt.057` - NotificationToReceive
3. Find XSD: `iso20022/notification_to_receive/camt.057.001.08.xsd`
4. Check MDR Part2 for details: `iso20022/notification_to_receive/ISO20022_MDRPart2_NotificationToReceive_2023_2024_v1.pdf`

### Task 2: Understand a message structure

**Example:** "What fields are in pacs.008?"

1. Open XSD: `iso20022/payments_clearing_and_settlement/pacs.008.001.13.xsd`
2. Read MDR Part2: `iso20022/payments_clearing_and_settlement/ISO20022_MDRPart2_PaymentsClearingAndSettlement_2024_2025_v1.pdf`
3. Check catalog: `iso_message_catalog.json` → search for "pacs.008"
4. Key fields listed:
   - `GrpHdr.MsgId` - Message ID
   - `CdtTrfTxInf.PmtId.UETR` - Universal Transaction Reference
   - `CdtTrfTxInf.IntrBkSttlmAmt` - Settlement amount
   - `CdtTrfTxInf.DbtrAgt` - Debtor agent BIC
   - `CdtTrfTxInf.CdtrAgt` - Creditor agent BIC

### Task 3: Generate code from XSD

**Manual approach (recommended for MVP):**
1. Read XSD structure
2. Hand-write structs with proper types
3. Implement custom parsing/serialization
4. Add business rule validation

**Automated approach (future):**
1. Use XSD-to-code generators (experimental)
2. Tools: `xsd2` (Rust), `xjc` (Java)
3. Post-process generated code for DelTran needs

### Task 4: Add a new ISO message

**Steps:**
1. Identify message type (e.g., `camt.XXX`)
2. Find XSD in appropriate archive directory
3. Read MDR Part2 for business rules
4. Add to [`iso_message_catalog.json`](iso_message_catalog.json)
5. Map to DelTran service in [`DELTRAN_ISO_MAPPING.md`](DELTRAN_ISO_MAPPING.md)
6. Implement parser in Gateway
7. Add Canonical mapping
8. Write tests

---

## Testing

### Test Message Library

Create standard test messages in `tests/iso_messages/`:

```
tests/iso_messages/
├── pain.001.valid.xml
├── pain.001.invalid_schema.xml
├── pacs.008.valid.xml
├── camt.054.valid_credit.xml
├── camt.054.valid_debit.xml
├── camt.053.valid_statement.xml
└── ...
```

**Generate test messages:**
1. Use examples from MDR Part2 documentation
2. ISO 20022 message generators (if available)
3. Real examples from bank test environments
4. Hand-craft based on XSD definitions

### Unit Test Template

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pain001_parsing() {
        let xml = include_str!("../../tests/iso_messages/pain.001.valid.xml");
        let result = parse_pain001(xml);

        assert!(result.is_ok());
        let payment = result.unwrap();
        assert_eq!(payment.group_header.message_id, "MSG123456");
    }

    #[test]
    fn test_pain001_validation() {
        let xml = include_str!("../../tests/iso_messages/pain.001.valid.xml");
        let result = validate_against_xsd(xml, "pain.001.001.12");

        assert!(result.is_ok());
    }

    #[test]
    fn test_canonical_mapping() {
        let pain001 = create_test_pain001();
        let canonical = map_to_canonical(&pain001);

        assert_eq!(canonical.end_to_end_id, pain001.end_to_end_id);
        assert_eq!(canonical.amount, Decimal::from_str("1000.00").unwrap());
    }
}
```

---

## Troubleshooting

### Issue: XML parsing fails

**Symptoms:** Parser rejects valid-looking XML

**Causes:**
- Wrong XSD version
- Missing namespace declarations
- Character encoding issues (non-UTF-8)
- Invalid date/time format

**Solutions:**
1. Validate XML against correct XSD version
2. Check namespace: `xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.12"`
3. Ensure UTF-8 encoding: `<?xml version="1.0" encoding="UTF-8"?>`
4. Use ISO 8601 format for dates: `2025-11-18T14:30:00Z`

### Issue: Business rule validation fails

**Symptoms:** XSD validation passes, but message rejected

**Causes:**
- Control sum mismatch
- Invalid BIC format
- Date constraints violated
- Cross-field validation failed

**Solutions:**
1. Check MDR Part2 for business rules
2. Validate control sums: sum of amounts = control sum
3. Verify BIC codes against directory
4. Check date logic (requested date ≥ today)

### Issue: camt.054 not triggering token mint

**Symptoms:** Funding notification received but balance not updated

**Debugging:**
1. Check `CdtDbtInd` field = `CRDT` (must be credit)
2. Check `Sts` field = `BOOK` (must be booked, not pending)
3. Verify `AcctSvcrRef` matches EMI account mapping
4. Check `EndToEndId` for obligation matching
5. Look for exceptions in Token Engine logs

**Common fixes:**
- Ensure account mapping exists in database
- Verify amount parsing (decimal places)
- Check currency code matches EMI account currency

---

## External Resources

### Official ISO 20022

- **ISO 20022 Website:** https://www.iso20022.org/
- **Message Catalogue:** https://www.iso20022.org/catalogue-messages
- **Registration:** https://www.iso20022.org/registration

### SWIFT ISO 20022

- **SWIFT Standards:** https://www.swift.com/standards/iso-20022
- **MyStandards:** https://www2.swift.com/mystandards/ (requires account)
- **SWIFT gpi:** https://www.swift.com/our-solutions/swift-gpi

### Regional Implementations

- **Payments UK:** https://www.wearepay.uk/iso-20022/
- **ECB (SEPA):** https://www.ecb.europa.eu/paym/groups/shared/docs/75299-sg-recommendations.pdf
- **Federal Reserve (US):** https://www.frbservices.org/resources/financial-services/wires/iso-20022-adoption.html

### Tools

- **XML Validators:** https://www.freeformatter.com/xml-validator-xsd.html
- **ISO 20022 Lab:** https://iso20022lab.com/ (commercial)
- **Postman Collections:** Community-maintained ISO 20022 test collections

---

## Support & Questions

### For Implementation Questions

1. Check [`DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md`](DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md)
2. Review specific message in [`iso_message_catalog.json`](iso_message_catalog.json)
3. Consult MDR Part2 documentation in relevant archive folder
4. Ask in #deltran-iso20022 Slack channel

### For Business Questions

1. Check [`ISO_INDEX.md`](ISO_INDEX.md) for business context
2. Review MDR Part1 (business processes) in archive folders
3. Consult with Product team
4. Escalate to ISO 20022 working group

### For Bank Integration Questions

1. Request bank's ISO 20022 implementation guide
2. Verify XSD versions used by bank
3. Request test endpoint and sample messages
4. Coordinate with bank's integration team

---

## Maintenance

### When to Update

- **New ISO version released:** Download updated XSD/MDR from ISO 20022 website
- **Bank requires different version:** Add new version alongside existing
- **New message type needed:** Follow "Task 4: Add a new ISO message" above
- **Business rules change:** Update validation logic, document in code

### Version Management

**Current versions:**
- Payments Initiation: v12-14 (2024-2025)
- Payments Clearing: v11-15 (2024-2025)
- Cash Management: v7-13 (2024-2025)
- Account Management: v4-8 (2023-2024)

**Strategy:**
- Support multiple versions simultaneously
- Gateway detects version from namespace
- Route to appropriate parser
- Gradually deprecate old versions

---

## Success Metrics

### Implementation Success

- [ ] 100% of P0 messages implemented and tested
- [ ] End-to-end payment flow working (pain.001 → camt.054 → pacs.008)
- [ ] Zero critical bugs in funding/settlement
- [ ] All P0 integration tests passing
- [ ] Production-ready error handling

### Integration Success

- [ ] At least 2 banks integrated successfully
- [ ] camt.054 funding events processing reliably
- [ ] EOD reconciliation (camt.053) automated
- [ ] Settlement success rate > 99.5%
- [ ] Average processing time < 5 seconds

---

## License & Compliance

**ISO 20022 Standards:**
- Copyright © ISO (International Organization for Standardization)
- Usage permitted for implementation purposes
- Redistribution of standards requires ISO permission

**DelTran Implementation:**
- Internal use only
- Do not share XSD files externally without verifying license
- Consult legal team for any external sharing

---

## Changelog

### Version 1.0 (2025-11-18)

- Initial extraction of 18 ISO 20022 archives
- Created master implementation plan
- Mapped all 136 messages to DelTran services
- Built message catalog and navigation guides
- Documented P0-P3 priorities for MVP
- Identified critical paths (camt.054, pacs.008)

### Next Version

- [ ] Extract MDR Part1 summaries to `summary/` folder
- [ ] Build machine-readable data dictionaries from Part3
- [ ] Generate code templates from priority XSD files
- [ ] Create bank-specific integration guides
- [ ] Add message validation test suite

---

**Ready to start implementing? Begin with:**

1. **[DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md](DELTRAN_ISO20022_IMPLEMENTATION_PLAN.md)** - Your complete roadmap
2. **Phase 1 (Weeks 1-2):** Foundation - XML parsing, canonical model, database schema
3. **Phase 2 (Weeks 3-4):** P0 messages - pain.001, camt.054, pacs.008, status reports

**Questions?** See "Support & Questions" section above.

---

*Generated: 2025-11-18*
*Total Archives: 18 | Total Messages: 136 | Priority P0 Messages: 5*
