// camt.054 XML parser

use crate::models::AccountTransaction;
use anyhow::{Result, anyhow};
use quick_xml::de::from_str;
use rust_decimal::Decimal;
use serde::Deserialize;
use tracing::{info, warn};

pub struct CamtParser;

// ISO 20022 camt.054 structures
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Document {
    #[serde(rename = "BkToCstmrDbtCdtNtfctn")]
    notification: BankToCustomerNotification,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct BankToCustomerNotification {
    ntfctn: Vec<Notification>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Notification {
    acct: Account,
    ntry: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Account {
    id: AccountId,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
struct AccountId {
    #[serde(rename = "IBAN")]
    iban: Option<String>,
    othr: Option<OtherAccountId>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct OtherAccountId {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Entry {
    amt: Amount,
    cdt_dbt_ind: String,
    booking_dt: BookingDate,
    val_dt: Option<ValueDate>,
    ntry_dtls: Option<EntryDetails>,
}

#[derive(Debug, Deserialize)]
struct Amount {
    #[serde(rename = "@Ccy")]
    currency: String,
    #[serde(rename = "$text")]
    value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct BookingDate {
    dt: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ValueDate {
    dt: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EntryDetails {
    tx_dtls: Vec<TransactionDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TransactionDetails {
    refs: References,
    rltd_pties: Option<RelatedParties>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct References {
    end_to_end_id: Option<String>,
    tx_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RelatedParties {
    dbtr: Option<Party>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Party {
    nm: Option<String>,
    #[serde(rename = "PstlAdr")]
    postal_address: Option<PostalAddress>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PostalAddress {
    ctry: Option<String>,
}

impl CamtParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse camt.054 XML into AccountTransaction list
    pub fn parse_camt054(&self, xml: &str) -> Result<Vec<AccountTransaction>> {
        info!("📄 Parsing camt.054 XML ({} bytes)", xml.len());

        // Parse XML
        let document: Document = from_str(xml).map_err(|e| {
            anyhow!("Failed to parse camt.054 XML: {}", e)
        })?;

        let mut transactions = Vec::new();

        // Extract transactions from notification
        for notification in document.notification.ntfctn {
            let account_id = self.extract_account_id(&notification.acct);

            for entry in notification.ntry {
                // Parse amount
                let amount: Decimal = entry.amt.value.parse()
                    .map_err(|e| anyhow!("Failed to parse amount: {}", e))?;

                let currency = entry.amt.currency.clone();
                let credit_debit_indicator = entry.cdt_dbt_ind.clone();
                let booking_date = entry.booking_dt.dt.clone();
                let value_date = entry.val_dt.as_ref().map(|vd| vd.dt.clone());

                // Extract transaction details
                if let Some(details) = entry.ntry_dtls {
                    for tx_detail in details.tx_dtls {
                        let transaction_id = tx_detail.refs.tx_id.clone();
                        let end_to_end_id = tx_detail.refs.end_to_end_id.clone();

                        // Extract debtor info
                        let (debtor_name, debtor_account) = if let Some(parties) = &tx_detail.rltd_pties {
                            if let Some(debtor) = &parties.dbtr {
                                (debtor.nm.clone(), None) // TODO: Extract account from debtor
                            } else {
                                (None, None)
                            }
                        } else {
                            (None, None)
                        };

                        transactions.push(AccountTransaction {
                            transaction_id,
                            account_id: account_id.clone(),
                            amount,
                            currency: currency.clone(),
                            credit_debit_indicator: credit_debit_indicator.clone(),
                            end_to_end_id,
                            debtor_name,
                            debtor_account,
                            booking_date: booking_date.parse().ok(),
                            value_date: value_date.as_ref().and_then(|v| v.parse().ok()),
                        });
                    }
                }
            }
        }

        info!("✅ Parsed {} transactions from camt.054", transactions.len());

        Ok(transactions)
    }

    fn extract_account_id(&self, account: &Account) -> String {
        if let Some(ref iban) = account.id.iban {
            iban.clone()
        } else if let Some(ref othr) = account.id.othr {
            othr.id.clone()
        } else {
            "UNKNOWN".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_camt054_example() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.054.001.08">
  <BkToCstmrDbtCdtNtfctn>
    <Ntfctn>
      <Acct>
        <Id>
          <IBAN>AE070331234567890123456</IBAN>
        </Id>
      </Acct>
      <Ntry>
        <Amt Ccy="AED">100000.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <BookingDt>
          <Dt>2025-01-18</Dt>
        </BookingDt>
        <NtryDtls>
          <TxDtls>
            <Refs>
              <EndToEndId>E2E123456</EndToEndId>
              <TxId>TXN987654</TxId>
            </Refs>
            <RltdPties>
              <Dbtr>
                <Nm>Test Company LLC</Nm>
              </Dbtr>
            </RltdPties>
          </TxDtls>
        </NtryDtls>
      </Ntry>
    </Ntfctn>
  </BkToCstmrDbtCdtNtfctn>
</Document>"#;

        let parser = CamtParser::new();
        let transactions = parser.parse_camt054(xml).unwrap();

        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].amount, Decimal::new(10000000, 2)); // 100000.00
        assert_eq!(transactions[0].currency, "AED");
        assert_eq!(transactions[0].credit_debit_indicator, "CRDT");
    }
}
