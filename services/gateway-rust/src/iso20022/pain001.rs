// pain.001 - Customer Credit Transfer Initiation
// This is the PRIMARY ENTRY POINT for payments into DelTran

use quick_xml::de::from_str;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use chrono::{DateTime, Utc, NaiveDate};
use rust_decimal::Decimal;
use crate::models::canonical::*;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Document {
    #[serde(rename = "CstmrCdtTrfInitn")]
    pub customer_credit_transfer_initiation: CustomerCreditTransferInitiation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CustomerCreditTransferInitiation {
    pub grp_hdr: GroupHeader,
    pub pmt_inf: Vec<PaymentInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GroupHeader {
    pub msg_id: String,
    pub cre_dt_tm: String,
    pub nb_of_txs: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctrl_sum: Option<String>,
    pub initg_pty: Party,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PaymentInformation {
    pub pmt_inf_id: String,
    pub pmt_mtd: String, // Should be "TRF" for transfers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub btch_bookg: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nb_of_txs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctrl_sum: Option<String>,
    pub pmt_tp_inf: Option<PaymentTypeInformation>,
    pub reqd_exctn_dt: Option<DateAndDateTime>,
    pub dbtr: Party,
    pub dbtr_acct: Account,
    pub dbtr_agt: FinancialInstitutionIdentification,
    #[serde(rename = "CdtTrfTxInf")]
    pub credit_transfer_tx_info: Vec<CreditTransferTransactionInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PaymentTypeInformation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instr_prty: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svc_lvl: Option<ServiceLevel>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceLevel {
    pub cd: Option<String>,
    pub prtry: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DateAndDateTime {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dt_tm: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Party {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pstl_adr: Option<PostalAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<PartyIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PostalAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adr_tp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strt_nm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bldg_nb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pst_cd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub twn_nm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctry: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PartyIdentification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<OrganisationIdentification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prvt_id: Option<PrivateIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct OrganisationIdentification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_bic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub othr: Option<Vec<GenericIdentification>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PrivateIdentification {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub othr: Option<Vec<GenericIdentification>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GenericIdentification {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schme_nm: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Account {
    pub id: AccountIdentification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ccy: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AccountIdentification {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "IBAN")]
    pub iban: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub othr: Option<GenericAccountIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GenericAccountIdentification {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schme_nm: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FinancialInstitutionIdentification {
    pub fin_instn_id: FinancialInstitution,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct FinancialInstitution {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "BICFI")]
    pub bicfi: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pstl_adr: Option<PostalAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub othr: Option<GenericIdentification>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreditTransferTransactionInformation {
    pub pmt_id: PaymentIdentification,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pmt_tp_inf: Option<PaymentTypeInformation>,
    pub amt: Amount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chrg_br: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ultmt_dbtr: Option<Party>,
    pub cdtr_agt: FinancialInstitutionIdentification,
    pub cdtr: Party,
    pub cdtr_acct: Account,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ultmt_cdtr: Option<Party>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purp: Option<Purpose>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rmt_inf: Option<RemittanceInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PaymentIdentification {
    pub instr_id: String,
    pub end_to_end_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "UETR")]
    pub uetr: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Amount {
    #[serde(rename = "InstdAmt")]
    pub instructed_amount: CurrencyAndAmount,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrencyAndAmount {
    #[serde(rename = "@Ccy")]
    pub currency: String,
    #[serde(rename = "$text")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Purpose {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prtry: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RemittanceInformation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ustrd: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strd: Option<Vec<StructuredRemittanceInformation>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct StructuredRemittanceInformation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rfrd_doc_inf: Option<Vec<ReferredDocumentInformation>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ReferredDocumentInformation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tp: Option<DocumentType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rltd_dt: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DocumentType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cd_or_prtry: Option<CodeOrProprietary>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CodeOrProprietary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prtry: Option<String>,
}

// Parser function
pub fn parse_pain001(xml: &str) -> Result<Document> {
    from_str(xml).context("Failed to parse pain.001 XML")
}

// Conversion to Canonical Model
pub fn to_canonical(pain001: &Document) -> Result<Vec<CanonicalPayment>> {
    let mut payments = Vec::new();
    let initiation = &pain001.customer_credit_transfer_initiation;

    for pmt_inf in &initiation.pmt_inf {
        for tx_inf in &pmt_inf.credit_transfer_tx_info {
            let payment = convert_transaction(
                &initiation.grp_hdr,
                pmt_inf,
                tx_inf,
            )?;
            payments.push(payment);
        }
    }

    Ok(payments)
}

fn convert_transaction(
    grp_hdr: &GroupHeader,
    pmt_inf: &PaymentInformation,
    tx_inf: &CreditTransferTransactionInformation,
) -> Result<CanonicalPayment> {
    // Parse amount
    let amount_str = &tx_inf.amt.instructed_amount.value;
    let amount: Decimal = amount_str.parse()
        .context(format!("Failed to parse amount: {}", amount_str))?;

    // Parse currency
    let currency = Currency::from_str(&tx_inf.amt.instructed_amount.currency)
        .context(format!("Unsupported currency: {}", tx_inf.amt.instructed_amount.currency))?;

    // Convert debtor
    let debtor = convert_party(&pmt_inf.dbtr)?;

    // Convert creditor
    let creditor = convert_party(&tx_inf.cdtr)?;

    // Convert debtor agent
    let debtor_agent = convert_financial_institution(&pmt_inf.dbtr_agt)?;

    // Convert creditor agent
    let creditor_agent = convert_financial_institution(&tx_inf.cdtr_agt)?;

    // Create canonical payment
    let mut payment = CanonicalPayment::new(
        tx_inf.pmt_id.end_to_end_id.clone(),
        tx_inf.pmt_id.instr_id.clone(),
        grp_hdr.msg_id.clone(),
        amount,
        currency,
        debtor,
        creditor,
        debtor_agent,
        creditor_agent,
    );

    // Set UETR from message if present, otherwise keep generated one
    if let Some(uetr_str) = &tx_inf.pmt_id.uetr {
        if let Ok(uetr_from_msg) = uuid::Uuid::parse_str(uetr_str) {
            payment.uetr = Some(uetr_from_msg);
        }
        // If parsing fails, keep the auto-generated UETR from CanonicalPayment::new()
    }
    // Note: UETR is now always present (generated in new() if not in message)

    // Set debtor account
    payment.debtor_account = convert_account(&pmt_inf.dbtr_acct)?;

    // Set creditor account
    payment.creditor_account = convert_account(&tx_inf.cdtr_acct)?;

    // Set charge bearer
    if let Some(chrg_br) = &tx_inf.chrg_br {
        payment.charge_bearer = match chrg_br.as_str() {
            "SHAR" => ChargeBearer::Shar,
            "SLEV" => ChargeBearer::Slev,
            "DEBT" => ChargeBearer::Debt,
            "CRED" => ChargeBearer::Cred,
            _ => ChargeBearer::Shar, // Default
        };
    }

    // Set remittance information
    if let Some(rmt_inf) = &tx_inf.rmt_inf {
        if let Some(ustrd) = &rmt_inf.ustrd {
            payment.remittance_info = ustrd.join("; ");
        }
    }

    // Determine corridor
    let debtor_country = extract_country_code(&pmt_inf.dbtr);
    let creditor_country = extract_country_code(&tx_inf.cdtr);
    payment.corridor = format!("{}_{}", debtor_country, creditor_country);

    payment.status = PaymentStatus::Validated;

    Ok(payment)
}

fn convert_party(party: &Party) -> Result<crate::models::canonical::Party> {
    let name = party.nm.clone().unwrap_or_else(|| "Unknown".to_string());

    let postal_address = party.pstl_adr.as_ref().map(|addr| {
        crate::models::canonical::PostalAddress {
            street_name: addr.strt_nm.clone(),
            building_number: addr.bldg_nb.clone(),
            post_code: addr.pst_cd.clone(),
            town_name: addr.twn_nm.clone(),
            country: addr.ctry.clone().unwrap_or_else(|| "XX".to_string()),
        }
    });

    let country_code = party.pstl_adr.as_ref()
        .and_then(|addr| addr.ctry.clone())
        .unwrap_or_else(|| "XX".to_string());

    Ok(crate::models::canonical::Party {
        name,
        postal_address,
        identification: None,
        country_code,
    })
}

fn convert_financial_institution(fin_inst: &FinancialInstitutionIdentification)
    -> Result<crate::models::canonical::FinancialInstitution> {

    let bic = fin_inst.fin_instn_id.bicfi.clone();
    let name = fin_inst.fin_instn_id.nm.clone().unwrap_or_else(|| "Unknown Bank".to_string());

    let country_code = fin_inst.fin_instn_id.pstl_adr.as_ref()
        .and_then(|addr| addr.ctry.clone())
        .unwrap_or_else(|| {
            // Try to extract from BIC (first 2 chars after bank code are country)
            bic.as_ref()
                .and_then(|b| b.get(4..6))
                .unwrap_or("XX")
                .to_string()
        });

    Ok(crate::models::canonical::FinancialInstitution {
        bic,
        name,
        country_code,
        clearing_system_member_id: None,
    })
}

fn convert_account(account: &Account) -> Result<crate::models::canonical::AccountIdentification> {
    let iban = account.id.iban.clone();
    let other = account.id.othr.as_ref().map(|o| o.id.clone());

    let account_type = if iban.is_some() {
        crate::models::canonical::AccountType::Checking
    } else {
        crate::models::canonical::AccountType::Other
    };

    Ok(crate::models::canonical::AccountIdentification {
        iban,
        bban: None,
        other,
        account_type,
    })
}

fn extract_country_code(party: &Party) -> String {
    party.pstl_adr.as_ref()
        .and_then(|addr| addr.ctry.clone())
        .unwrap_or_else(|| "XX".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pain001() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.12">
  <CstmrCdtTrfInitn>
    <GrpHdr>
      <MsgId>MSG-20251118-001</MsgId>
      <CreDtTm>2025-11-18T14:30:00Z</CreDtTm>
      <NbOfTxs>1</NbOfTxs>
      <InitgPty>
        <Nm>ACME Corp</Nm>
      </InitgPty>
    </GrpHdr>
    <PmtInf>
      <PmtInfId>PMT-INFO-001</PmtInfId>
      <PmtMtd>TRF</PmtMtd>
      <Dbtr>
        <Nm>John Doe</Nm>
        <PstlAdr>
          <Ctry>AE</Ctry>
        </PstlAdr>
      </Dbtr>
      <DbtrAcct>
        <Id>
          <IBAN>AE070331234567890123456</IBAN>
        </Id>
      </DbtrAcct>
      <DbtrAgt>
        <FinInstnId>
          <BICFI>BANKAEADXXX</BICFI>
          <Nm>Bank of UAE</Nm>
        </FinInstnId>
      </DbtrAgt>
      <CdtTrfTxInf>
        <PmtId>
          <InstrId>INSTR-001</InstrId>
          <EndToEndId>E2E-001</EndToEndId>
        </PmtId>
        <Amt>
          <InstdAmt Ccy="AED">10000.00</InstdAmt>
        </Amt>
        <CdtrAgt>
          <FinInstnId>
            <BICFI>ICICIINBBXXX</BICFI>
            <Nm>ICICI Bank India</Nm>
          </FinInstnId>
        </CdtrAgt>
        <Cdtr>
          <Nm>Jane Smith</Nm>
          <PstlAdr>
            <Ctry>IN</Ctry>
          </PstlAdr>
        </Cdtr>
        <CdtrAcct>
          <Id>
            <Othr>
              <Id>123456789012</Id>
            </Othr>
          </Id>
        </CdtrAcct>
      </CdtTrfTxInf>
    </PmtInf>
  </CstmrCdtTrfInitn>
</Document>"#;

        let result = parse_pain001(xml);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let doc = result.unwrap();
        assert_eq!(doc.customer_credit_transfer_initiation.grp_hdr.msg_id, "MSG-20251118-001");
        assert_eq!(doc.customer_credit_transfer_initiation.pmt_inf.len(), 1);
    }

    #[test]
    fn test_to_canonical() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.12">
  <CstmrCdtTrfInitn>
    <GrpHdr>
      <MsgId>MSG-TEST</MsgId>
      <CreDtTm>2025-11-18T14:30:00Z</CreDtTm>
      <NbOfTxs>1</NbOfTxs>
      <InitgPty><Nm>Test</Nm></InitgPty>
    </GrpHdr>
    <PmtInf>
      <PmtInfId>PMT-001</PmtInfId>
      <PmtMtd>TRF</PmtMtd>
      <Dbtr><Nm>John</Nm><PstlAdr><Ctry>AE</Ctry></PstlAdr></Dbtr>
      <DbtrAcct><Id><IBAN>AE123</IBAN></Id></DbtrAcct>
      <DbtrAgt><FinInstnId><BICFI>BANKAEXX</BICFI></FinInstnId></DbtrAgt>
      <CdtTrfTxInf>
        <PmtId><InstrId>I1</InstrId><EndToEndId>E1</EndToEndId></PmtId>
        <Amt><InstdAmt Ccy="AED">1000.00</InstdAmt></Amt>
        <CdtrAgt><FinInstnId><BICFI>BANKINXX</BICFI></FinInstnId></CdtrAgt>
        <Cdtr><Nm>Jane</Nm><PstlAdr><Ctry>IN</Ctry></PstlAdr></Cdtr>
        <CdtrAcct><Id><Othr><Id>ACC123</Id></Othr></Id></CdtrAcct>
      </CdtTrfTxInf>
    </PmtInf>
  </CstmrCdtTrfInitn>
</Document>"#;

        let doc = parse_pain001(xml).unwrap();
        let payments = to_canonical(&doc).unwrap();

        assert_eq!(payments.len(), 1);
        let payment = &payments[0];
        assert_eq!(payment.end_to_end_id, "E1");
        assert_eq!(payment.instruction_id, "I1");
        assert_eq!(payment.corridor, "AE_IN");
        assert_eq!(payment.status, PaymentStatus::Validated);
    }
}
