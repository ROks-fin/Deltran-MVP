//! ISO 20022 message validation
//!
//! Validates both inbound and outbound ISO 20022 messages against schema rules.

use crate::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validation rule severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,   // Blocks processing
    Warning, // Logs warning but continues
    Info,    // Informational only
}

/// Validation error detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub severity: String,
    pub field_path: String,
    pub message: String,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, code: &str, field_path: &str, message: String) {
        self.valid = false;
        self.errors.push(ValidationError {
            code: code.to_string(),
            severity: "ERROR".to_string(),
            field_path: field_path.to_string(),
            message,
        });
    }

    pub fn add_warning(&mut self, code: &str, field_path: &str, message: String) {
        self.warnings.push(ValidationError {
            code: code.to_string(),
            severity: "WARNING".to_string(),
            field_path: field_path.to_string(),
            message,
        });
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// ISO 20022 message validator
pub struct Iso20022Validator {
    /// Supported currency codes (ISO 4217)
    supported_currencies: Vec<String>,

    /// Maximum transaction amount per currency
    max_amounts: HashMap<String, Decimal>,

    /// Strict mode (reject warnings as errors)
    strict_mode: bool,
}

impl Iso20022Validator {
    /// Create new validator
    pub fn new(supported_currencies: Vec<String>, strict_mode: bool) -> Self {
        let mut max_amounts = HashMap::new();
        // Default limits per currency
        max_amounts.insert("USD".to_string(), Decimal::from(10_000_000));
        max_amounts.insert("EUR".to_string(), Decimal::from(10_000_000));
        max_amounts.insert("GBP".to_string(), Decimal::from(10_000_000));
        max_amounts.insert("INR".to_string(), Decimal::from(750_000_000)); // ~$10M
        max_amounts.insert("AED".to_string(), Decimal::from(37_000_000));
        max_amounts.insert("PKR".to_string(), Decimal::from(2_800_000_000));
        max_amounts.insert("NIS".to_string(), Decimal::from(35_000_000));

        Self {
            supported_currencies,
            max_amounts,
            strict_mode,
        }
    }

    /// Validate pacs.008 message
    pub fn validate_pacs008(&self, xml_content: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::success();

        // Parse XML
        let doc = match roxmltree::Document::parse(xml_content) {
            Ok(d) => d,
            Err(e) => {
                result.add_error("XML_PARSE", "root", format!("XML parsing failed: {}", e));
                return Ok(result);
            }
        };

        // Validate root element
        let root = doc.root_element();
        if root.tag_name().name() != "Document" {
            result.add_error(
                "INVALID_ROOT",
                "root",
                format!("Expected 'Document' root element, found '{}'", root.tag_name().name()),
            );
            return Ok(result);
        }

        // Validate namespace
        if let Some(ns) = root.attribute("xmlns") {
            if !ns.starts_with("urn:iso:std:iso:20022:tech:xsd:pacs.008") {
                result.add_warning(
                    "NAMESPACE_MISMATCH",
                    "Document@xmlns",
                    format!("Unexpected namespace: {}", ns),
                );
            }
        } else {
            result.add_error("MISSING_NAMESPACE", "Document@xmlns", "Missing xmlns attribute".to_string());
        }

        // Validate FIToFICstmrCdtTrf element
        if let Some(fi_to_fi) = root.children().find(|n| n.tag_name().name() == "FIToFICstmrCdtTrf") {
            self.validate_fi_to_fi(&fi_to_fi, &mut result)?;
        } else {
            result.add_error(
                "MISSING_ELEMENT",
                "FIToFICstmrCdtTrf",
                "Missing FIToFICstmrCdtTrf element".to_string(),
            );
        }

        Ok(result)
    }

    /// Validate pacs.009 (financial institution credit transfer) message
    pub fn validate_pacs009(&self, xml_content: &str) -> Result<ValidationResult> {
        let mut result = ValidationResult::success();

        let doc = match roxmltree::Document::parse(xml_content) {
            Ok(d) => d,
            Err(e) => {
                result.add_error("XML_PARSE", "root", format!("XML parsing failed: {}", e));
                return Ok(result);
            }
        };

        let root = doc.root_element();
        if root.tag_name().name() != "Document" {
            result.add_error(
                "INVALID_ROOT",
                "root",
                format!("Expected 'Document' root element, found '{}'", root.tag_name().name()),
            );
            return Ok(result);
        }

        // Validate namespace for pacs.009
        if let Some(ns) = root.attribute("xmlns") {
            if !ns.starts_with("urn:iso:std:iso:20022:tech:xsd:pacs.009") {
                result.add_warning(
                    "NAMESPACE_MISMATCH",
                    "Document@xmlns",
                    format!("Unexpected namespace for pacs.009: {}", ns),
                );
            }
        }

        // Validate FinInstnCdtTrf element
        if let Some(fin_cdt_trf) = root.children().find(|n| n.tag_name().name() == "FICdtTrf") {
            self.validate_fi_credit_transfer(&fin_cdt_trf, &mut result)?;
        } else {
            result.add_error(
                "MISSING_ELEMENT",
                "FICdtTrf",
                "Missing FICdtTrf element".to_string(),
            );
        }

        Ok(result)
    }

    fn validate_fi_to_fi(&self, node: &roxmltree::Node, result: &mut ValidationResult) -> Result<()> {
        // Validate GroupHeader
        if let Some(grp_hdr) = node.children().find(|n| n.tag_name().name() == "GrpHdr") {
            self.validate_group_header(&grp_hdr, result)?;
        } else {
            result.add_error("MISSING_ELEMENT", "GrpHdr", "Missing GrpHdr element".to_string());
        }

        // Validate CreditTransferTransactionInformation
        if let Some(cdt_trf) = node.children().find(|n| n.tag_name().name() == "CdtTrfTxInf") {
            self.validate_credit_transfer_tx(&cdt_trf, result)?;
        } else {
            result.add_error("MISSING_ELEMENT", "CdtTrfTxInf", "Missing CdtTrfTxInf element".to_string());
        }

        Ok(())
    }

    fn validate_fi_credit_transfer(&self, node: &roxmltree::Node, result: &mut ValidationResult) -> Result<()> {
        // Similar validation for pacs.009
        if let Some(grp_hdr) = node.children().find(|n| n.tag_name().name() == "GrpHdr") {
            self.validate_group_header(&grp_hdr, result)?;
        } else {
            result.add_error("MISSING_ELEMENT", "GrpHdr", "Missing GrpHdr element".to_string());
        }

        Ok(())
    }

    fn validate_group_header(&self, node: &roxmltree::Node, result: &mut ValidationResult) -> Result<()> {
        // Validate MsgId
        if let Some(msg_id) = self.get_child_text(node, "MsgId") {
            if msg_id.is_empty() || msg_id.len() > 35 {
                result.add_error(
                    "INVALID_LENGTH",
                    "GrpHdr/MsgId",
                    format!("MsgId must be 1-35 characters, got {}", msg_id.len()),
                );
            }
        } else {
            result.add_error("MISSING_FIELD", "GrpHdr/MsgId", "Missing MsgId".to_string());
        }

        // Validate CreDtTm (creation date/time)
        if self.get_child_text(node, "CreDtTm").is_none() {
            result.add_error("MISSING_FIELD", "GrpHdr/CreDtTm", "Missing CreDtTm".to_string());
        }

        // Validate NbOfTxs (number of transactions)
        if let Some(nb_txs_str) = self.get_child_text(node, "NbOfTxs") {
            match nb_txs_str.parse::<u32>() {
                Ok(nb) if nb == 0 => {
                    result.add_error(
                        "INVALID_VALUE",
                        "GrpHdr/NbOfTxs",
                        "NbOfTxs must be greater than 0".to_string(),
                    );
                }
                Err(_) => {
                    result.add_error(
                        "INVALID_FORMAT",
                        "GrpHdr/NbOfTxs",
                        format!("Invalid number format: {}", nb_txs_str),
                    );
                }
                _ => {}
            }
        } else {
            result.add_error("MISSING_FIELD", "GrpHdr/NbOfTxs", "Missing NbOfTxs".to_string());
        }

        // Validate TtlIntrBkSttlmAmt (total settlement amount)
        if let Some(amt_node) = node.children().find(|n| n.tag_name().name() == "TtlIntrBkSttlmAmt") {
            self.validate_amount(&amt_node, "GrpHdr/TtlIntrBkSttlmAmt", result)?;
        } else {
            result.add_error(
                "MISSING_FIELD",
                "GrpHdr/TtlIntrBkSttlmAmt",
                "Missing TtlIntrBkSttlmAmt".to_string(),
            );
        }

        Ok(())
    }

    fn validate_credit_transfer_tx(&self, node: &roxmltree::Node, result: &mut ValidationResult) -> Result<()> {
        // Validate PaymentIdentification
        if node.children().find(|n| n.tag_name().name() == "PmtId").is_none() {
            result.add_error("MISSING_ELEMENT", "CdtTrfTxInf/PmtId", "Missing PmtId".to_string());
        }

        // Validate IntrBkSttlmAmt
        if let Some(amt_node) = node.children().find(|n| n.tag_name().name() == "IntrBkSttlmAmt") {
            self.validate_amount(&amt_node, "CdtTrfTxInf/IntrBkSttlmAmt", result)?;
        } else {
            result.add_error(
                "MISSING_FIELD",
                "CdtTrfTxInf/IntrBkSttlmAmt",
                "Missing IntrBkSttlmAmt".to_string(),
            );
        }

        // Validate Debtor
        if let Some(dbtr) = node.children().find(|n| n.tag_name().name() == "Dbtr") {
            self.validate_party(&dbtr, "CdtTrfTxInf/Dbtr", result)?;
        } else {
            result.add_error("MISSING_ELEMENT", "CdtTrfTxInf/Dbtr", "Missing Dbtr".to_string());
        }

        // Validate Creditor
        if let Some(cdtr) = node.children().find(|n| n.tag_name().name() == "Cdtr") {
            self.validate_party(&cdtr, "CdtTrfTxInf/Cdtr", result)?;
        } else {
            result.add_error("MISSING_ELEMENT", "CdtTrfTxInf/Cdtr", "Missing Cdtr".to_string());
        }

        Ok(())
    }

    fn validate_amount(&self, node: &roxmltree::Node, path: &str, result: &mut ValidationResult) -> Result<()> {
        // Validate currency attribute
        if let Some(ccy) = node.attribute("Ccy") {
            if !self.supported_currencies.contains(&ccy.to_string()) {
                result.add_warning(
                    "UNSUPPORTED_CURRENCY",
                    &format!("{}@Ccy", path),
                    format!("Currency '{}' not in supported list", ccy),
                );
            }

            // Validate amount value
            if let Some(amount_text) = node.text() {
                match amount_text.parse::<Decimal>() {
                    Ok(amount) => {
                        if amount <= Decimal::ZERO {
                            result.add_error(
                                "INVALID_AMOUNT",
                                path,
                                "Amount must be greater than zero".to_string(),
                            );
                        }

                        // Check against max amount
                        if let Some(max_amt) = self.max_amounts.get(ccy) {
                            if amount > *max_amt {
                                result.add_error(
                                    "AMOUNT_EXCEEDS_LIMIT",
                                    path,
                                    format!("Amount {} exceeds limit {} for {}", amount, max_amt, ccy),
                                );
                            }
                        }
                    }
                    Err(_) => {
                        result.add_error(
                            "INVALID_FORMAT",
                            path,
                            format!("Invalid decimal format: {}", amount_text),
                        );
                    }
                }
            } else {
                result.add_error("MISSING_VALUE", path, "Amount value is empty".to_string());
            }
        } else {
            result.add_error("MISSING_ATTRIBUTE", &format!("{}@Ccy", path), "Missing Ccy attribute".to_string());
        }

        Ok(())
    }

    fn validate_party(&self, node: &roxmltree::Node, path: &str, result: &mut ValidationResult) -> Result<()> {
        // Validate Name
        if self.get_child_text(node, "Nm").is_none() {
            result.add_warning("MISSING_FIELD", &format!("{}/Nm", path), "Missing party name".to_string());
        }

        // Validate FinInstnId
        if let Some(fin_instn) = node.children().find(|n| n.tag_name().name() == "FinInstnId") {
            // Validate BIC
            if let Some(bic) = self.get_child_text(&fin_instn, "BICFI") {
                if !self.is_valid_bic(&bic) {
                    result.add_error(
                        "INVALID_BIC",
                        &format!("{}/FinInstnId/BICFI", path),
                        format!("Invalid BIC format: {}", bic),
                    );
                }
            } else {
                result.add_error(
                    "MISSING_FIELD",
                    &format!("{}/FinInstnId/BICFI", path),
                    "Missing BIC code".to_string(),
                );
            }
        } else {
            result.add_error(
                "MISSING_ELEMENT",
                &format!("{}/FinInstnId", path),
                "Missing FinInstnId".to_string(),
            );
        }

        Ok(())
    }

    fn get_child_text(&self, node: &roxmltree::Node, tag_name: &str) -> Option<String> {
        node.children()
            .find(|n| n.tag_name().name() == tag_name)
            .and_then(|n| n.text())
            .map(|s| s.to_string())
    }

    fn is_valid_bic(&self, bic: &str) -> bool {
        // BIC format: 4 letters (institution) + 2 letters (country) + 2 alphanumeric (location) + optional 3 alphanumeric (branch)
        let len = bic.len();
        (len == 8 || len == 11) && bic.chars().all(|c| c.is_ascii_alphanumeric())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_validator() -> Iso20022Validator {
        Iso20022Validator::new(
            vec!["USD".to_string(), "EUR".to_string(), "GBP".to_string()],
            false,
        )
    }

    #[test]
    fn test_valid_pacs008() {
        let validator = create_validator();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
  <FIToFICstmrCdtTrf>
    <GrpHdr>
      <MsgId>DELTRAN-20250930-001</MsgId>
      <CreDtTm>2025-09-30T12:00:00Z</CreDtTm>
      <NbOfTxs>1</NbOfTxs>
      <TtlIntrBkSttlmAmt Ccy="USD">1000.00</TtlIntrBkSttlmAmt>
    </GrpHdr>
    <CdtTrfTxInf>
      <PmtId>
        <InstrId>TEST001</InstrId>
        <EndToEndId>TEST001</EndToEndId>
        <TxId>TEST001</TxId>
      </PmtId>
      <IntrBkSttlmAmt Ccy="USD">1000.00</IntrBkSttlmAmt>
      <Dbtr>
        <Nm>Test Bank A</Nm>
        <FinInstnId>
          <BICFI>CHASUS33</BICFI>
        </FinInstnId>
      </Dbtr>
      <Cdtr>
        <Nm>Test Bank B</Nm>
        <FinInstnId>
          <BICFI>DEUTDEFF</BICFI>
        </FinInstnId>
      </Cdtr>
    </CdtTrfTxInf>
  </FIToFICstmrCdtTrf>
</Document>"#;

        let result = validator.validate_pacs008(xml).unwrap();
        assert!(result.valid);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_invalid_bic() {
        let validator = create_validator();
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
  <FIToFICstmrCdtTrf>
    <GrpHdr>
      <MsgId>TEST001</MsgId>
      <CreDtTm>2025-09-30T12:00:00Z</CreDtTm>
      <NbOfTxs>1</NbOfTxs>
      <TtlIntrBkSttlmAmt Ccy="USD">1000.00</TtlIntrBkSttlmAmt>
    </GrpHdr>
    <CdtTrfTxInf>
      <PmtId>
        <InstrId>TEST001</InstrId>
      </PmtId>
      <IntrBkSttlmAmt Ccy="USD">1000.00</IntrBkSttlmAmt>
      <Dbtr>
        <Nm>Test Bank A</Nm>
        <FinInstnId>
          <BICFI>INVALID</BICFI>
        </FinInstnId>
      </Dbtr>
      <Cdtr>
        <Nm>Test Bank B</Nm>
        <FinInstnId>
          <BICFI>DEUTDEFF</BICFI>
        </FinInstnId>
      </Cdtr>
    </CdtTrfTxInf>
  </FIToFICstmrCdtTrf>
</Document>"#;

        let result = validator.validate_pacs008(xml).unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "INVALID_BIC"));
    }

    #[test]
    fn test_bic_validation() {
        let validator = create_validator();
        assert!(validator.is_valid_bic("CHASUS33"));
        assert!(validator.is_valid_bic("DEUTDEFF"));
        assert!(validator.is_valid_bic("CHASUS33XXX"));
        assert!(!validator.is_valid_bic("INVALID"));
        assert!(!validator.is_valid_bic("CHAS"));
    }
}
