// pain.002.001.10 - Customer Payment Status Report
// Used to inform customer about payment initiation status

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use anyhow::{Result, anyhow};

/// pain.002 Document root
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Document {
    #[serde(rename = "CstmrPmtStsRpt")]
    pub customer_payment_status_report: CustomerPaymentStatusReport,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CustomerPaymentStatusReport {
    pub grp_hdr: GroupHeader,

    #[serde(rename = "OrgnlGrpInfAndSts")]
    pub original_group_info_and_status: Option<OriginalGroupInfoAndStatus>,

    #[serde(rename = "OrgnlPmtInfAndSts")]
    pub original_payment_info_and_status: Option<Vec<OriginalPaymentInfoAndStatus>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GroupHeader {
    pub msg_id: String,
    pub cre_dt_tm: String,
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

    #[serde(rename = "StsRsnInf")]
    pub status_reason_info: Option<Vec<StatusReasonInfo>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OriginalPaymentInfoAndStatus {
    #[serde(rename = "OrgnlPmtInfId")]
    pub original_payment_info_id: String,

    #[serde(rename = "PmtInfSts")]
    pub payment_info_status: Option<String>,

    #[serde(rename = "StsRsnInf")]
    pub status_reason_info: Option<Vec<StatusReasonInfo>>,

    #[serde(rename = "TxInfAndSts")]
    pub transaction_info_and_status: Option<Vec<TransactionInfoAndStatus>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TransactionInfoAndStatus {
    #[serde(rename = "StsId")]
    pub status_id: Option<String>,

    #[serde(rename = "OrgnlInstrId")]
    pub original_instruction_id: Option<String>,

    #[serde(rename = "OrgnlEndToEndId")]
    pub original_end_to_end_id: String,

    #[serde(rename = "TxSts")]
    pub transaction_status: String, // ACCP, ACTC, ACWC, PART, PDNG, RJCT

    #[serde(rename = "StsRsnInf")]
    pub status_reason_info: Option<Vec<StatusReasonInfo>>,

    #[serde(rename = "AccptncDtTm")]
    pub acceptance_date_time: Option<String>,

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
    pub cd: Option<String>,
    pub prtry: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OriginalTransactionReference {
    #[serde(rename = "Amt")]
    pub amount: Option<Amount>,

    #[serde(rename = "ReqdExctnDt")]
    pub requested_execution_date: Option<String>,

    #[serde(rename = "Cdtr")]
    pub creditor: Option<Party>,

    #[serde(rename = "CdtrAcct")]
    pub creditor_account: Option<Account>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Amount {
    #[serde(rename = "InstdAmt")]
    pub instructed_amount: Option<InstructedAmount>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InstructedAmount {
    #[serde(rename = "@Ccy")]
    pub currency: String,

    #[serde(rename = "$text")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Party {
    pub nm: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Account {
    pub id: Option<AccountId>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct AccountId {
    #[serde(rename = "IBAN")]
    pub iban: Option<String>,

    #[serde(rename = "Othr")]
    pub other: Option<OtherAccountId>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OtherAccountId {
    pub id: String,
}

/// Customer payment status report for DelTran
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerPaymentStatus {
    pub status_id: String,
    pub original_end_to_end_id: String,
    pub original_instruction_id: Option<String>,
    pub status: PaymentStatus,
    pub status_code: String,
    pub reason_code: Option<String>,
    pub reason_description: Option<String>,
    pub acceptance_date_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentStatus {
    Accepted,
    Pending,
    Rejected,
    Unknown,
}

/// Parse pain.002 XML message
pub fn parse_pain002(xml: &str) -> Result<Document> {
    quick_xml::de::from_str(xml)
        .map_err(|e| anyhow!("Failed to parse pain.002 XML: {}", e))
}

/// Convert pain.002 to DelTran customer payment status reports
pub fn to_customer_payment_status(document: &Document) -> Result<Vec<CustomerPaymentStatus>> {
    let mut statuses = Vec::new();

    if let Some(pmt_info_list) = &document.customer_payment_status_report.original_payment_info_and_status {
        for pmt_info in pmt_info_list {
            if let Some(tx_list) = &pmt_info.transaction_info_and_status {
                for tx_info in tx_list {
                    let status = match tx_info.transaction_status.as_str() {
                        "ACCP" | "ACTC" | "ACWC" => PaymentStatus::Accepted,
                        "PDNG" | "PART" => PaymentStatus::Pending,
                        "RJCT" => PaymentStatus::Rejected,
                        _ => PaymentStatus::Unknown,
                    };

                    // Extract reason
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

                    statuses.push(CustomerPaymentStatus {
                        status_id: tx_info.status_id.clone()
                            .unwrap_or_else(|| Uuid::new_v4().to_string()),
                        original_end_to_end_id: tx_info.original_end_to_end_id.clone(),
                        original_instruction_id: tx_info.original_instruction_id.clone(),
                        status,
                        status_code: tx_info.transaction_status.clone(),
                        reason_code,
                        reason_description,
                        acceptance_date_time,
                    });
                }
            }
        }
    }

    Ok(statuses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pain002_accepted() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.002.001.10">
  <CstmrPmtStsRpt>
    <GrpHdr>
      <MsgId>CUST_STATUS_001</MsgId>
      <CreDtTm>2025-01-19T10:00:00Z</CreDtTm>
    </GrpHdr>
    <OrgnlPmtInfAndSts>
      <OrgnlPmtInfId>PMT_INFO_001</OrgnlPmtInfId>
      <TxInfAndSts>
        <OrgnlEndToEndId>E2E123456</OrgnlEndToEndId>
        <TxSts>ACCP</TxSts>
        <AccptncDtTm>2025-01-19T10:01:00Z</AccptncDtTm>
      </TxInfAndSts>
    </OrgnlPmtInfAndSts>
  </CstmrPmtStsRpt>
</Document>"#;

        let document = parse_pain002(xml).unwrap();
        let statuses = to_customer_payment_status(&document).unwrap();

        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].original_end_to_end_id, "E2E123456");
        assert!(matches!(statuses[0].status, PaymentStatus::Accepted));
    }
}
