//! ISO 20022 message generation
//!
//! Generates pacs.008 (FIToFICustomerCreditTransfer) messages for settlement.
//!
//! # Standards
//!
//! - ISO 20022: Universal financial industry message scheme
//! - pacs.008.001.08: Financial Institution To Financial Institution Customer Credit Transfer
//!
//! # Example Output
//!
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
//!   <FIToFICstmrCdtTrf>
//!     <GrpHdr>
//!       <MsgId>DELTRAN-20250930-001</MsgId>
//!       <CreDtTm>2025-09-30T12:00:00Z</CreDtTm>
//!       <NbOfTxs>1</NbOfTxs>
//!       <TtlIntrBkSttlmAmt Ccy="USD">100.00</TtlIntrBkSttlmAmt>
//!     </GrpHdr>
//!     <CdtTrfTxInf>
//!       <PmtId>...</PmtId>
//!       <IntrBkSttlmAmt Ccy="USD">100.00</IntrBkSttlmAmt>
//!       <Dbtr>...</Dbtr>
//!       <Cdtr>...</Cdtr>
//!     </CdtTrfTxInf>
//!   </FIToFICstmrCdtTrf>
//! </Document>
//! ```

use crate::{types::*, Error, Result};
use chrono::{DateTime, Utc};
use quick_xml::se::to_string as to_xml_string;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// ISO 20022 pacs.008 message generator
pub struct Iso20022Generator {
    /// Sender BIC
    sender_bic: String,

    /// Output directory
    output_dir: std::path::PathBuf,

    /// Pretty print
    pretty_print: bool,
}

impl Iso20022Generator {
    /// Create new generator
    pub fn new(
        sender_bic: String,
        output_dir: std::path::PathBuf,
        pretty_print: bool,
    ) -> Self {
        Self {
            sender_bic,
            output_dir,
            pretty_print,
        }
    }

    /// Generate pacs.008 for settlement batch
    pub fn generate_pacs008(&self, batch: &SettlementBatch) -> Result<Vec<String>> {
        // Create output directory
        std::fs::create_dir_all(&self.output_dir)?;

        let mut files = Vec::new();

        // Generate one file per net transfer
        for (idx, transfer) in batch.net_transfers.iter().enumerate() {
            let document = self.build_pacs008_document(batch, transfer, idx)?;
            let xml = self.serialize_xml(&document)?;

            // File name: DELTRAN-YYYYMMDD-HHMMSS-NNN.xml
            let filename = format!(
                "DELTRAN-{}-{:03}.xml",
                batch.created_at.format("%Y%m%d-%H%M%S"),
                idx
            );

            let filepath = self.output_dir.join(&filename);
            std::fs::write(&filepath, xml)?;

            tracing::info!("Generated ISO 20022 file: {}", filename);
            files.push(filename);
        }

        Ok(files)
    }

    /// Build pacs.008 document structure
    fn build_pacs008_document(
        &self,
        batch: &SettlementBatch,
        transfer: &NetTransfer,
        index: usize,
    ) -> Result<Pacs008Document> {
        let msg_id = format!(
            "DELTRAN-{}-{:03}",
            batch.created_at.format("%Y%m%d-%H%M%S"),
            index
        );

        let group_header = GroupHeader {
            msg_id: msg_id.clone(),
            cre_dt_tm: batch.created_at,
            nb_of_txs: 1,
            ttl_intrBk_sttlm_amt: AmountAndCurrency {
                ccy: transfer.currency.code().to_string(),
                value: transfer.net_amount,
            },
        };

        let credit_transfer = CreditTransferTxInfo {
            pmt_id: PaymentIdentification {
                instr_id: transfer.transfer_id.to_string(),
                end_to_end_id: transfer.transfer_id.to_string(),
                tx_id: transfer.transfer_id.to_string(),
            },
            intrBk_sttlm_amt: AmountAndCurrency {
                ccy: transfer.currency.code().to_string(),
                value: transfer.net_amount,
            },
            dbtr: Party {
                nm: transfer.debtor_bank.as_str().to_string(),
                fin_instn_id: FinancialInstitutionId {
                    bicfi: transfer.debtor_bank.as_str().to_string(),
                },
            },
            cdtr: Party {
                nm: transfer.creditor_bank.as_str().to_string(),
                fin_instn_id: FinancialInstitutionId {
                    bicfi: transfer.creditor_bank.as_str().to_string(),
                },
            },
        };

        Ok(Pacs008Document {
            xmlns: "urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08".to_string(),
            fi_to_fi_cstmr_cdt_trf: FIToFICstmrCdtTrf {
                grp_hdr: group_header,
                cdt_trf_tx_inf: credit_transfer,
            },
        })
    }

    /// Serialize to XML
    fn serialize_xml(&self, document: &Pacs008Document) -> Result<String> {
        let xml = to_xml_string(document)
            .map_err(|e| Error::Iso20022(format!("XML serialization failed: {}", e)))?;

        // Add XML declaration
        let full_xml = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", xml);

        if self.pretty_print {
            // Basic pretty printing (in production, use a proper XML library)
            Ok(full_xml)
        } else {
            Ok(full_xml)
        }
    }
}

// ISO 20022 pacs.008 structures

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "Document")]
struct Pacs008Document {
    #[serde(rename = "@xmlns")]
    xmlns: String,

    #[serde(rename = "FIToFICstmrCdtTrf")]
    fi_to_fi_cstmr_cdt_trf: FIToFICstmrCdtTrf,
}

#[derive(Debug, Serialize, Deserialize)]
struct FIToFICstmrCdtTrf {
    #[serde(rename = "GrpHdr")]
    grp_hdr: GroupHeader,

    #[serde(rename = "CdtTrfTxInf")]
    cdt_trf_tx_inf: CreditTransferTxInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroupHeader {
    #[serde(rename = "MsgId")]
    msg_id: String,

    #[serde(rename = "CreDtTm")]
    cre_dt_tm: DateTime<Utc>,

    #[serde(rename = "NbOfTxs")]
    nb_of_txs: u32,

    #[serde(rename = "TtlIntrBkSttlmAmt")]
    ttl_intrBk_sttlm_amt: AmountAndCurrency,
}

#[derive(Debug, Serialize, Deserialize)]
struct AmountAndCurrency {
    #[serde(rename = "@Ccy")]
    ccy: String,

    #[serde(rename = "$text")]
    value: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreditTransferTxInfo {
    #[serde(rename = "PmtId")]
    pmt_id: PaymentIdentification,

    #[serde(rename = "IntrBkSttlmAmt")]
    intrBk_sttlm_amt: AmountAndCurrency,

    #[serde(rename = "Dbtr")]
    dbtr: Party,

    #[serde(rename = "Cdtr")]
    cdtr: Party,
}

#[derive(Debug, Serialize, Deserialize)]
struct PaymentIdentification {
    #[serde(rename = "InstrId")]
    instr_id: String,

    #[serde(rename = "EndToEndId")]
    end_to_end_id: String,

    #[serde(rename = "TxId")]
    tx_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Party {
    #[serde(rename = "Nm")]
    nm: String,

    #[serde(rename = "FinInstnId")]
    fin_instn_id: FinancialInstitutionId,
}

#[derive(Debug, Serialize, Deserialize)]
struct FinancialInstitutionId {
    #[serde(rename = "BICFI")]
    bicfi: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_iso20022_generation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let generator = Iso20022Generator::new(
            "DELTRAEAD".to_string(),
            temp_dir.path().to_path_buf(),
            true,
        );

        let transfer = NetTransfer {
            transfer_id: Uuid::new_v4(),
            debtor_bank: BankId::new("CHASUS33"),
            creditor_bank: BankId::new("DEUTDEFF"),
            currency: Currency::USD,
            net_amount: Decimal::new(100000, 2), // $1,000.00
            payment_ids: vec![],
            netting_ratio: 0.5,
        };

        let batch = SettlementBatch {
            batch_id: Uuid::new_v4(),
            window_start: Utc::now(),
            window_end: Utc::now(),
            currency: Currency::USD,
            payment_count: 10,
            gross_obligations: vec![],
            net_transfers: vec![transfer],
            total_gross_amount: Decimal::new(200000, 2),
            total_net_amount: Decimal::new(100000, 2),
            netting_efficiency: 0.5,
            status: SettlementStatus::Netted,
            created_at: Utc::now(),
            iso20022_files: vec![],
        };

        let files = generator.generate_pacs008(&batch).unwrap();
        assert_eq!(files.len(), 1);

        // Verify file exists
        let filepath = temp_dir.path().join(&files[0]);
        assert!(filepath.exists());

        // Verify XML content
        let xml_content = std::fs::read_to_string(&filepath).unwrap();
        assert!(xml_content.contains("<?xml version"));
        assert!(xml_content.contains("Document"));
        assert!(xml_content.contains("FIToFICstmrCdtTrf"));
        assert!(xml_content.contains("CHASUS33"));
        assert!(xml_content.contains("DEUTDEFF"));
        assert!(xml_content.contains("USD"));
    }
}