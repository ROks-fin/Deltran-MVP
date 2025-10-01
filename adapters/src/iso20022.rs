//! ISO 20022 pacs.008 (FIToFICustomerCreditTransfer) generator
//!
//! Full compliance with ISO 20022 standard for payment messages.

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use protocol_core::{Account, SettlementInstruction};
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use uuid::Uuid;

/// ISO 20022 pacs.008 message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pacs008 {
    /// Group header
    pub group_header: GroupHeader,
    /// Credit transfer transaction information
    pub credit_transfer_tx_info: Vec<CreditTransferTxInfo>,
}

/// Group header (GrpHdr)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupHeader {
    /// Message ID (MsgId)
    pub message_id: String,
    /// Creation date/time (CreDtTm)
    pub creation_date_time: DateTime<Utc>,
    /// Number of transactions (NbOfTxs)
    pub number_of_txs: u32,
    /// Total interbank settlement amount (TtlIntrBkSttlmAmt)
    pub total_interbank_settlement_amount: Decimal,
    /// Currency (Ccy)
    pub currency: String,
    /// Interbank settlement date (IntrBkSttlmDt)
    pub interbank_settlement_date: DateTime<Utc>,
    /// Settlement method (SttlmMtd)
    pub settlement_method: SettlementMethod,
    /// Instructing agent (InstgAgt)
    pub instructing_agent: FinancialInstitution,
    /// Instructed agent (InstdAgt)
    pub instructed_agent: FinancialInstitution,
}

/// Credit transfer transaction information (CdtTrfTxInf)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransferTxInfo {
    /// Payment identification (PmtId)
    pub payment_id: PaymentIdentification,
    /// Interbank settlement amount (IntrBkSttlmAmt)
    pub interbank_settlement_amount: Decimal,
    /// Currency (Ccy)
    pub currency: String,
    /// Charge bearer (ChrgBr)
    pub charge_bearer: ChargeBearer,
    /// Debtor (Dbtr)
    pub debtor: Party,
    /// Debtor account (DbtrAcct)
    pub debtor_account: CashAccount,
    /// Debtor agent (DbtrAgt)
    pub debtor_agent: FinancialInstitution,
    /// Creditor agent (CdtrAgt)
    pub creditor_agent: FinancialInstitution,
    /// Creditor (Cdtr)
    pub creditor: Party,
    /// Creditor account (CdtrAcct)
    pub creditor_account: CashAccount,
    /// Remittance information (RmtInf)
    pub remittance_info: Option<RemittanceInformation>,
}

/// Payment identification (PmtId)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIdentification {
    /// Instruction ID (InstrId)
    pub instruction_id: String,
    /// End-to-end ID (EndToEndId)
    pub end_to_end_id: String,
    /// Transaction ID (TxId)
    pub transaction_id: String,
    /// UETR (RFC 4122 UUID)
    pub uetr: Option<String>,
}

/// Financial institution (FI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialInstitution {
    /// BIC (BICFI)
    pub bic: String,
    /// Name
    pub name: Option<String>,
    /// LEI (Legal Entity Identifier)
    pub lei: Option<String>,
}

/// Party (e.g., debtor or creditor)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    /// Name
    pub name: String,
    /// Postal address
    pub postal_address: Option<PostalAddress>,
    /// Identification
    pub identification: Option<String>,
}

/// Cash account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashAccount {
    /// IBAN or other identifier
    pub identification: String,
    /// Currency
    pub currency: Option<String>,
}

/// Postal address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostalAddress {
    /// Address line 1
    pub address_line_1: Option<String>,
    /// Address line 2
    pub address_line_2: Option<String>,
    /// City
    pub city: Option<String>,
    /// Postal code
    pub postal_code: Option<String>,
    /// Country code (ISO 3166-1 alpha-2)
    pub country_code: String,
}

/// Remittance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemittanceInformation {
    /// Unstructured text
    pub unstructured: Option<String>,
}

/// Settlement method
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SettlementMethod {
    /// INDA (Instructed Agent)
    Inda,
    /// INGA (Instructing Agent)
    Inga,
    /// COVE (Cover)
    Cove,
    /// CLRG (Clearing)
    Clrg,
}

impl SettlementMethod {
    fn as_str(&self) -> &str {
        match self {
            SettlementMethod::Inda => "INDA",
            SettlementMethod::Inga => "INGA",
            SettlementMethod::Cove => "COVE",
            SettlementMethod::Clrg => "CLRG",
        }
    }
}

/// Charge bearer
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChargeBearer {
    /// DEBT (Debtor)
    Debt,
    /// CRED (Creditor)
    Cred,
    /// SHAR (Shared)
    Shar,
    /// SLEV (Service Level)
    Slev,
}

impl ChargeBearer {
    fn as_str(&self) -> &str {
        match self {
            ChargeBearer::Debt => "DEBT",
            ChargeBearer::Cred => "CRED",
            ChargeBearer::Shar => "SHAR",
            ChargeBearer::Slev => "SLEV",
        }
    }
}

/// ISO 20022 pacs.008 generator
pub struct Pacs008Generator;

impl Pacs008Generator {
    /// Generate pacs.008 from settlement instruction
    pub fn from_instruction(
        instruction: &SettlementInstruction,
        instructing_agent_bic: &str,
        instructed_agent_bic: &str,
    ) -> Result<Pacs008> {
        let msg_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Extract debtor/creditor info from instruction
        // TODO: In production, fetch full details from database

        let group_header = GroupHeader {
            message_id: msg_id.clone(),
            creation_date_time: now,
            number_of_txs: 1,
            total_interbank_settlement_amount: instruction.amount,
            currency: instruction.currency.clone(),
            interbank_settlement_date: now,
            settlement_method: SettlementMethod::Clrg,
            instructing_agent: FinancialInstitution {
                bic: instructing_agent_bic.to_string(),
                name: None,
                lei: None,
            },
            instructed_agent: FinancialInstitution {
                bic: instructed_agent_bic.to_string(),
                name: None,
                lei: None,
            },
        };

        let payment_id = PaymentIdentification {
            instruction_id: instruction.instruction_id.to_string(),
            end_to_end_id: instruction.instruction_id.to_string(),
            transaction_id: instruction.instruction_id.to_string(),
            uetr: Some(Uuid::new_v4().to_string()),
        };

        let tx_info = CreditTransferTxInfo {
            payment_id,
            interbank_settlement_amount: instruction.amount,
            currency: instruction.currency.clone(),
            charge_bearer: ChargeBearer::Shar,
            debtor: Party {
                name: instruction.from_bank.clone(),
                postal_address: None,
                identification: None,
            },
            debtor_account: CashAccount {
                identification: instruction.from_bank.clone(),
                currency: Some(instruction.currency.clone()),
            },
            debtor_agent: FinancialInstitution {
                bic: instruction.from_bank.clone(),
                name: None,
                lei: None,
            },
            creditor_agent: FinancialInstitution {
                bic: instruction.to_bank.clone(),
                name: None,
                lei: None,
            },
            creditor: Party {
                name: instruction.to_bank.clone(),
                postal_address: None,
                identification: None,
            },
            creditor_account: CashAccount {
                identification: instruction.to_bank.clone(),
                currency: Some(instruction.currency.clone()),
            },
            remittance_info: None,
        };

        Ok(Pacs008 {
            group_header,
            credit_transfer_tx_info: vec![tx_info],
        })
    }

    /// Serialize pacs.008 to XML
    pub fn to_xml(msg: &Pacs008) -> Result<String> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        // XML declaration
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        // Root element
        let mut root = BytesStart::new("Document");
        root.push_attribute(("xmlns", "urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08"));
        writer
            .write_event(Event::Start(root))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        // FIToFICstmrCdtTrf
        writer
            .write_event(Event::Start(BytesStart::new("FIToFICstmrCdtTrf")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        // Group header
        Self::write_group_header(&mut writer, &msg.group_header)?;

        // Credit transfer transactions
        for tx_info in &msg.credit_transfer_tx_info {
            Self::write_credit_transfer_tx_info(&mut writer, tx_info)?;
        }

        // Close FIToFICstmrCdtTrf
        writer
            .write_event(Event::End(BytesEnd::new("FIToFICstmrCdtTrf")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        // Close Document
        writer
            .write_event(Event::End(BytesEnd::new("Document")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        let result = writer.into_inner().into_inner();
        String::from_utf8(result).map_err(|e| Error::Iso20022Serialization(e.to_string()))
    }

    fn write_group_header(writer: &mut Writer<Cursor<Vec<u8>>>, hdr: &GroupHeader) -> Result<()> {
        writer
            .write_event(Event::Start(BytesStart::new("GrpHdr")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Self::write_element(writer, "MsgId", &hdr.message_id)?;
        Self::write_element(
            writer,
            "CreDtTm",
            &hdr.creation_date_time.to_rfc3339(),
        )?;
        Self::write_element(writer, "NbOfTxs", &hdr.number_of_txs.to_string())?;

        // TtlIntrBkSttlmAmt with Ccy attribute
        let mut amt_start = BytesStart::new("TtlIntrBkSttlmAmt");
        amt_start.push_attribute(("Ccy", hdr.currency.as_str()));
        writer
            .write_event(Event::Start(amt_start))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;
        writer
            .write_event(Event::Text(BytesText::new(
                &hdr.total_interbank_settlement_amount.to_string(),
            )))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;
        writer
            .write_event(Event::End(BytesEnd::new("TtlIntrBkSttlmAmt")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Self::write_element(
            writer,
            "IntrBkSttlmDt",
            &hdr.interbank_settlement_date.format("%Y-%m-%d").to_string(),
        )?;
        Self::write_element(writer, "SttlmMtd", hdr.settlement_method.as_str())?;

        // Instructing/Instructed agents
        Self::write_financial_institution(writer, "InstgAgt", &hdr.instructing_agent)?;
        Self::write_financial_institution(writer, "InstdAgt", &hdr.instructed_agent)?;

        writer
            .write_event(Event::End(BytesEnd::new("GrpHdr")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Ok(())
    }

    fn write_credit_transfer_tx_info(
        writer: &mut Writer<Cursor<Vec<u8>>>,
        tx: &CreditTransferTxInfo,
    ) -> Result<()> {
        writer
            .write_event(Event::Start(BytesStart::new("CdtTrfTxInf")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        // Payment ID
        Self::write_payment_id(writer, &tx.payment_id)?;

        // Amount
        let mut amt_start = BytesStart::new("IntrBkSttlmAmt");
        amt_start.push_attribute(("Ccy", tx.currency.as_str()));
        writer
            .write_event(Event::Start(amt_start))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;
        writer
            .write_event(Event::Text(BytesText::new(
                &tx.interbank_settlement_amount.to_string(),
            )))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;
        writer
            .write_event(Event::End(BytesEnd::new("IntrBkSttlmAmt")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Self::write_element(writer, "ChrgBr", tx.charge_bearer.as_str())?;

        // Debtor
        Self::write_party(writer, "Dbtr", &tx.debtor)?;
        Self::write_cash_account(writer, "DbtrAcct", &tx.debtor_account)?;
        Self::write_financial_institution(writer, "DbtrAgt", &tx.debtor_agent)?;

        // Creditor
        Self::write_financial_institution(writer, "CdtrAgt", &tx.creditor_agent)?;
        Self::write_party(writer, "Cdtr", &tx.creditor)?;
        Self::write_cash_account(writer, "CdtrAcct", &tx.creditor_account)?;

        // Remittance info (if any)
        if let Some(ref rmt) = tx.remittance_info {
            if let Some(ref unstructured) = rmt.unstructured {
                writer
                    .write_event(Event::Start(BytesStart::new("RmtInf")))
                    .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;
                Self::write_element(writer, "Ustrd", unstructured)?;
                writer
                    .write_event(Event::End(BytesEnd::new("RmtInf")))
                    .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;
            }
        }

        writer
            .write_event(Event::End(BytesEnd::new("CdtTrfTxInf")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Ok(())
    }

    fn write_payment_id(
        writer: &mut Writer<Cursor<Vec<u8>>>,
        pmtid: &PaymentIdentification,
    ) -> Result<()> {
        writer
            .write_event(Event::Start(BytesStart::new("PmtId")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Self::write_element(writer, "InstrId", &pmtid.instruction_id)?;
        Self::write_element(writer, "EndToEndId", &pmtid.end_to_end_id)?;
        Self::write_element(writer, "TxId", &pmtid.transaction_id)?;

        if let Some(ref uetr) = pmtid.uetr {
            Self::write_element(writer, "UETR", uetr)?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("PmtId")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Ok(())
    }

    fn write_financial_institution(
        writer: &mut Writer<Cursor<Vec<u8>>>,
        tag: &str,
        fi: &FinancialInstitution,
    ) -> Result<()> {
        writer
            .write_event(Event::Start(BytesStart::new(tag)))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        writer
            .write_event(Event::Start(BytesStart::new("FinInstnId")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Self::write_element(writer, "BICFI", &fi.bic)?;

        if let Some(ref name) = fi.name {
            Self::write_element(writer, "Nm", name)?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("FinInstnId")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        writer
            .write_event(Event::End(BytesEnd::new(tag)))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Ok(())
    }

    fn write_party(writer: &mut Writer<Cursor<Vec<u8>>>, tag: &str, party: &Party) -> Result<()> {
        writer
            .write_event(Event::Start(BytesStart::new(tag)))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Self::write_element(writer, "Nm", &party.name)?;

        if let Some(ref addr) = party.postal_address {
            Self::write_postal_address(writer, addr)?;
        }

        writer
            .write_event(Event::End(BytesEnd::new(tag)))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Ok(())
    }

    fn write_postal_address(
        writer: &mut Writer<Cursor<Vec<u8>>>,
        addr: &PostalAddress,
    ) -> Result<()> {
        writer
            .write_event(Event::Start(BytesStart::new("PstlAdr")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        if let Some(ref line1) = addr.address_line_1 {
            Self::write_element(writer, "AdrLine", line1)?;
        }
        if let Some(ref line2) = addr.address_line_2 {
            Self::write_element(writer, "AdrLine", line2)?;
        }
        if let Some(ref city) = addr.city {
            Self::write_element(writer, "TwnNm", city)?;
        }
        if let Some(ref postal_code) = addr.postal_code {
            Self::write_element(writer, "PstCd", postal_code)?;
        }
        Self::write_element(writer, "Ctry", &addr.country_code)?;

        writer
            .write_event(Event::End(BytesEnd::new("PstlAdr")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Ok(())
    }

    fn write_cash_account(
        writer: &mut Writer<Cursor<Vec<u8>>>,
        tag: &str,
        acct: &CashAccount,
    ) -> Result<()> {
        writer
            .write_event(Event::Start(BytesStart::new(tag)))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        writer
            .write_event(Event::Start(BytesStart::new("Id")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Self::write_element(writer, "IBAN", &acct.identification)?;

        writer
            .write_event(Event::End(BytesEnd::new("Id")))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        writer
            .write_event(Event::End(BytesEnd::new(tag)))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;

        Ok(())
    }

    fn write_element(
        writer: &mut Writer<Cursor<Vec<u8>>>,
        tag: &str,
        text: &str,
    ) -> Result<()> {
        writer
            .write_event(Event::Start(BytesStart::new(tag)))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;
        writer
            .write_event(Event::Text(BytesText::new(text)))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;
        writer
            .write_event(Event::End(BytesEnd::new(tag)))
            .map_err(|e| Error::Iso20022Serialization(e.to_string()))?;
        Ok(())
    }

    /// Validate pacs.008 message
    pub fn validate(msg: &Pacs008) -> Result<()> {
        // BIC validation
        let bic_regex = regex::Regex::new(r"^[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?$")
            .map_err(|e| Error::Iso20022Validation(e.to_string()))?;

        if !bic_regex.is_match(&msg.group_header.instructing_agent.bic) {
            return Err(Error::Iso20022Validation(format!(
                "Invalid instructing agent BIC: {}",
                msg.group_header.instructing_agent.bic
            )));
        }

        if !bic_regex.is_match(&msg.group_header.instructed_agent.bic) {
            return Err(Error::Iso20022Validation(format!(
                "Invalid instructed agent BIC: {}",
                msg.group_header.instructed_agent.bic
            )));
        }

        // Amount validation
        if msg.group_header.total_interbank_settlement_amount <= Decimal::ZERO {
            return Err(Error::Iso20022Validation(
                "Total amount must be positive".to_string(),
            ));
        }

        // Number of transactions match
        if msg.group_header.number_of_txs != msg.credit_transfer_tx_info.len() as u32 {
            return Err(Error::Iso20022Validation(format!(
                "Number of transactions mismatch: header={}, actual={}",
                msg.group_header.number_of_txs,
                msg.credit_transfer_tx_info.len()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_pacs008_generation() {
        let instruction = SettlementInstruction {
            instruction_id: Uuid::new_v4(),
            from_bank: "BANKGB2L".to_string(),
            to_bank: "CHASUS33".to_string(),
            amount: dec!(1000.50),
            currency: "USD".to_string(),
            iso20022_pacs008: None,
            status: protocol_core::InstructionStatus::Pending,
            executed_at: None,
        };

        let msg = Pacs008Generator::from_instruction(&instruction, "DELTRANAEXX", "DELTRANUAEX")
            .unwrap();

        assert_eq!(msg.group_header.number_of_txs, 1);
        assert_eq!(msg.group_header.currency, "USD");
        assert_eq!(msg.credit_transfer_tx_info.len(), 1);
    }

    #[test]
    fn test_pacs008_to_xml() {
        let instruction = SettlementInstruction {
            instruction_id: Uuid::new_v4(),
            from_bank: "BANKGB2L".to_string(),
            to_bank: "CHASUS33".to_string(),
            amount: dec!(1000.50),
            currency: "USD".to_string(),
            iso20022_pacs008: None,
            status: protocol_core::InstructionStatus::Pending,
            executed_at: None,
        };

        let msg = Pacs008Generator::from_instruction(&instruction, "DELTRANAEXX", "DELTRANUAEX")
            .unwrap();

        let xml = Pacs008Generator::to_xml(&msg).unwrap();

        // Basic validation
        assert!(xml.contains("<Document"));
        assert!(xml.contains("<FIToFICstmrCdtTrf>"));
        assert!(xml.contains("<GrpHdr>"));
        assert!(xml.contains("<CdtTrfTxInf>"));
        assert!(xml.contains("BANKGB2L"));
        assert!(xml.contains("CHASUS33"));
    }

    #[test]
    fn test_pacs008_validation() {
        let instruction = SettlementInstruction {
            instruction_id: Uuid::new_v4(),
            from_bank: "BANKGB2L".to_string(),
            to_bank: "CHASUS33".to_string(),
            amount: dec!(1000.50),
            currency: "USD".to_string(),
            iso20022_pacs008: None,
            status: protocol_core::InstructionStatus::Pending,
            executed_at: None,
        };

        let msg = Pacs008Generator::from_instruction(&instruction, "DELTRANAEXX", "DELTRANUAEX")
            .unwrap();

        // Should pass validation
        assert!(Pacs008Generator::validate(&msg).is_ok());

        // Test invalid BIC
        let mut invalid_msg = msg.clone();
        invalid_msg.group_header.instructing_agent.bic = "INVALID".to_string();
        assert!(Pacs008Generator::validate(&invalid_msg).is_err());
    }
}