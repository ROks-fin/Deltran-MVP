// pacs.002.001.10 - FI to FI Payment Status Report
// Used for reporting status of payment instructions (Accept/Reject)

use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::{Result, anyhow};

/// pacs.002 Document root
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Document {
    #[serde(rename = "FIToFIPmtStsRpt")]
    pub fi_to_fi_payment_status_report: FIToFIPaymentStatusReport,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FIToFIPaymentStatusReport {
    pub grp_hdr: GroupHeader,
    #[serde(rename = "TxInfAndSts")]
    pub transaction_info_and_status: Vec<TransactionInfoAndStatus>,
    #[serde(rename = "OrgnlGrpInfAndSts")]
    pub original_group_info_and_status: Option<OriginalGroupInfoAndStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GroupHeader {
    pub msg_id: String,
    pub cre_dt_tm: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TransactionInfoAndStatus {
    #[serde(rename = "StsId")]
    pub status_id: Option<String>,

    #[serde(rename = "OrgnlInstrId")]
    pub original_instruction_id: Option<String>,

    #[serde(rename = "OrgnlEndToEndId")]
    pub original_end_to_end_id: Option<String>,

    #[serde(rename = "OrgnlTxId")]
    pub original_transaction_id: Option<String>,

    #[serde(rename = "OrgnlUETR")]
    pub original_uetr: Option<String>,

    #[serde(rename = "TxSts")]
    pub transaction_status: String, // ACCP, ACSC, ACSP, ACTC, ACWC, PART, PDNG, RJCT

    #[serde(rename = "StsRsnInf")]
    pub status_reason_info: Option<Vec<StatusReasonInfo>>,

    #[serde(rename = "AccptncDtTm")]
    pub acceptance_date_time: Option<String>,

    #[serde(rename = "AcctSvcrRef")]
    pub account_servicer_reference: Option<String>,

    #[serde(rename = "ClrSysRef")]
    pub clearing_system_reference: Option<String>,

    #[serde(rename = "OrgnlTxRef")]
    pub original_transaction_reference: Option<OriginalTransactionReference>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct StatusReasonInfo {
    #[serde(rename = "Rsn")]
    pub reason: Option<Reason>,

    #[serde(rename = "AddtlInf")]
    pub additional_info: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Reason {
    pub cd: Option<String>, // ISO 20022 reason code
    pub prtry: Option<String>, // Proprietary reason code
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OriginalTransactionReference {
    #[serde(rename = "IntrBkSttlmAmt")]
    pub interbank_settlement_amount: Option<Amount>,

    #[serde(rename = "IntrBkSttlmDt")]
    pub interbank_settlement_date: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Amount {
    #[serde(rename = "@Ccy")]
    pub currency: String,

    #[serde(rename = "$text")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OriginalGroupInfoAndStatus {
    #[serde(rename = "OrgnlMsgId")]
    pub original_message_id: String,

    #[serde(rename = "OrgnlMsgNmId")]
    pub original_message_name_id: String,

    #[serde(rename = "GrpSts")]
    pub group_status: Option<String>,
}

/// Payment Status representation for DelTran
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentStatusReport {
    pub status_id: String,
    pub original_end_to_end_id: Option<String>,
    pub original_tx_id: Option<String>,
    pub original_uetr: Option<Uuid>,
    pub status: PaymentStatus,
    pub status_code: String,
    pub reason_code: Option<String>,
    pub reason_description: Option<String>,
    pub acceptance_date_time: Option<DateTime<Utc>>,
    pub clearing_system_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Accepted,           // ACCP, ACSC, ACSP, ACTC, ACWC
    Pending,            // PDNG, PART
    Rejected,           // RJCT
    Unknown,
}

/// Parse pacs.002 XML message
pub fn parse_pacs002(xml: &str) -> Result<Document> {
    quick_xml::de::from_str(xml)
        .map_err(|e| anyhow!("Failed to parse pacs.002 XML: {}", e))
}

/// Convert pacs.002 to DelTran payment status reports
pub fn to_payment_status_reports(document: &Document) -> Result<Vec<PaymentStatusReport>> {
    let mut reports = Vec::new();

    for tx_info in &document.fi_to_fi_payment_status_report.transaction_info_and_status {
        let status = match tx_info.transaction_status.as_str() {
            "ACCP" | "ACSC" | "ACSP" | "ACTC" | "ACWC" => PaymentStatus::Accepted,
            "PDNG" | "PART" => PaymentStatus::Pending,
            "RJCT" => PaymentStatus::Rejected,
            _ => PaymentStatus::Unknown,
        };

        // Parse UETR if present
        let original_uetr = tx_info.original_uetr.as_ref()
            .and_then(|uetr| Uuid::parse_str(uetr).ok());

        // Extract reason code and description
        let (reason_code, reason_description) = if let Some(reasons) = &tx_info.status_reason_info {
            let first_reason = reasons.first();
            let code = first_reason
                .and_then(|r| r.reason.as_ref())
                .and_then(|r| r.cd.clone().or_else(|| r.prtry.clone()));

            let description = first_reason
                .and_then(|r| r.additional_info.as_ref())
                .and_then(|info| info.first())
                .cloned();

            (code, description)
        } else {
            (None, None)
        };

        // Parse acceptance date time
        let acceptance_date_time = tx_info.acceptance_date_time.as_ref()
            .and_then(|dt| DateTime::parse_from_rfc3339(dt).ok())
            .map(|dt| dt.with_timezone(&Utc));

        reports.push(PaymentStatusReport {
            status_id: tx_info.status_id.clone()
                .unwrap_or_else(|| Uuid::new_v4().to_string()),
            original_end_to_end_id: tx_info.original_end_to_end_id.clone(),
            original_tx_id: tx_info.original_transaction_id.clone(),
            original_uetr,
            status,
            status_code: tx_info.transaction_status.clone(),
            reason_code,
            reason_description,
            acceptance_date_time,
            clearing_system_reference: tx_info.clearing_system_reference.clone(),
        });
    }

    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pacs002_accepted() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.002.001.10">
  <FIToFIPmtStsRpt>
    <GrpHdr>
      <MsgId>STATUS123</MsgId>
      <CreDtTm>2025-01-19T10:00:00Z</CreDtTm>
    </GrpHdr>
    <TxInfAndSts>
      <StsId>STS001</StsId>
      <OrgnlEndToEndId>E2E123456</OrgnlEndToEndId>
      <OrgnlTxId>TXN987654</OrgnlTxId>
      <TxSts>ACCP</TxSts>
      <AccptncDtTm>2025-01-19T10:01:00Z</AccptncDtTm>
    </TxInfAndSts>
  </FIToFIPmtStsRpt>
</Document>"#;

        let document = parse_pacs002(xml).unwrap();
        let reports = to_payment_status_reports(&document).unwrap();

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].original_end_to_end_id, Some("E2E123456".to_string()));
        assert!(matches!(reports[0].status, PaymentStatus::Accepted));
    }

    #[test]
    fn test_parse_pacs002_rejected() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.002.001.10">
  <FIToFIPmtStsRpt>
    <GrpHdr>
      <MsgId>STATUS456</MsgId>
      <CreDtTm>2025-01-19T10:00:00Z</CreDtTm>
    </GrpHdr>
    <TxInfAndSts>
      <OrgnlEndToEndId>E2E789012</OrgnlEndToEndId>
      <TxSts>RJCT</TxSts>
      <StsRsnInf>
        <Rsn>
          <Cd>AM04</Cd>
        </Rsn>
        <AddtlInf>Insufficient funds</AddtlInf>
      </StsRsnInf>
    </TxInfAndSts>
  </FIToFIPmtStsRpt>
</Document>"#;

        let document = parse_pacs002(xml).unwrap();
        let reports = to_payment_status_reports(&document).unwrap();

        assert_eq!(reports.len(), 1);
        assert!(matches!(reports[0].status, PaymentStatus::Rejected));
        assert_eq!(reports[0].reason_code, Some("AM04".to_string()));
    }
}
