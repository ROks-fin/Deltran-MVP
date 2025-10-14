//! Core types for settlement engine

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Bank identifier (BIC/SWIFT code)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BankId(String);

impl BankId {
    /// Create new bank ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BankId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Currency code (ISO 4217)
pub type Currency = ledger_core::types::Currency;

/// Pending payment ready for settlement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPayment {
    /// Payment ID
    pub payment_id: Uuid,

    /// Payment amount
    pub amount: Decimal,

    /// Currency
    pub currency: Currency,

    /// Debtor bank
    pub debtor_bank: BankId,

    /// Creditor bank
    pub creditor_bank: BankId,

    /// Debtor account
    pub debtor_account: String,

    /// Creditor account
    pub creditor_account: String,

    /// Payment reference
    pub reference: String,

    /// Queued timestamp
    pub queued_at: DateTime<Utc>,
}

/// Gross bilateral obligation between two banks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BilateralObligation {
    /// Debtor bank
    pub debtor_bank: BankId,

    /// Creditor bank
    pub creditor_bank: BankId,

    /// Currency
    pub currency: Currency,

    /// Gross amount owed
    pub gross_amount: Decimal,

    /// Payment IDs included
    pub payment_ids: Vec<Uuid>,
}

/// Net transfer after netting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetTransfer {
    /// Transfer ID
    pub transfer_id: Uuid,

    /// Debtor bank (pays)
    pub debtor_bank: BankId,

    /// Creditor bank (receives)
    pub creditor_bank: BankId,

    /// Currency
    pub currency: Currency,

    /// Net amount to transfer
    pub net_amount: Decimal,

    /// Payment IDs netted
    pub payment_ids: Vec<Uuid>,

    /// Netting ratio (0.0 - 1.0)
    /// 1.0 = 100% netted (no transfer needed)
    /// 0.0 = 0% netted (full gross amount)
    pub netting_ratio: f64,
}

/// Settlement batch result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementBatch {
    /// Batch ID
    pub batch_id: Uuid,

    /// Window start time
    pub window_start: DateTime<Utc>,

    /// Window end time
    pub window_end: DateTime<Utc>,

    /// Currency
    pub currency: Currency,

    /// Pending payments processed
    pub payment_count: usize,

    /// Gross bilateral obligations
    pub gross_obligations: Vec<BilateralObligation>,

    /// Net transfers to execute
    pub net_transfers: Vec<NetTransfer>,

    /// Total gross amount
    pub total_gross_amount: Decimal,

    /// Total net amount
    pub total_net_amount: Decimal,

    /// Netting efficiency (0.0 - 1.0)
    /// Higher = more netting
    pub netting_efficiency: f64,

    /// Settlement status
    pub status: SettlementStatus,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// ISO 20022 files generated
    pub iso20022_files: Vec<String>,
}

impl SettlementBatch {
    /// Calculate netting efficiency
    pub fn calculate_efficiency(&self) -> f64 {
        if self.total_gross_amount == Decimal::ZERO {
            return 0.0;
        }

        let netted = self.total_gross_amount - self.total_net_amount;
        let efficiency = netted / self.total_gross_amount;
        efficiency.to_f64().unwrap_or(0.0)
    }

    /// Calculate savings from netting
    pub fn calculate_savings(&self) -> Decimal {
        self.total_gross_amount - self.total_net_amount
    }
}

/// Settlement status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SettlementStatus {
    /// Pending netting
    Pending = 1,
    /// Netting computed
    Netted = 2,
    /// ISO 20022 files generated
    FilesGenerated = 3,
    /// Submitted to banks
    Submitted = 4,
    /// Confirmed by banks
    Confirmed = 5,
    /// Failed
    Failed = 6,
}

/// Netting graph node (bank position)
#[derive(Debug, Clone)]
pub struct BankPosition {
    /// Bank ID
    pub bank_id: BankId,

    /// Currency
    pub currency: Currency,

    /// Total owed to others
    pub total_owed: Decimal,

    /// Total owed by others
    pub total_receivable: Decimal,

    /// Net position (positive = net receiver, negative = net payer)
    pub net_position: Decimal,
}

impl BankPosition {
    /// Create new position
    pub fn new(bank_id: BankId, currency: Currency) -> Self {
        Self {
            bank_id,
            currency,
            total_owed: Decimal::ZERO,
            total_receivable: Decimal::ZERO,
            net_position: Decimal::ZERO,
        }
    }

    /// Update position with obligation
    pub fn add_obligation(&mut self, amount: Decimal, is_debtor: bool) {
        if is_debtor {
            self.total_owed += amount;
        } else {
            self.total_receivable += amount;
        }
        self.net_position = self.total_receivable - self.total_owed;
    }

    /// Check if net payer (owes money)
    pub fn is_net_payer(&self) -> bool {
        self.net_position < Decimal::ZERO
    }

    /// Check if net receiver (receives money)
    pub fn is_net_receiver(&self) -> bool {
        self.net_position > Decimal::ZERO
    }

    /// Get absolute net position
    pub fn abs_net_position(&self) -> Decimal {
        self.net_position.abs()
    }
}

/// Netting statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NettingStats {
    /// Number of banks involved
    pub bank_count: usize,

    /// Number of gross payments
    pub gross_payment_count: usize,

    /// Number of net transfers
    pub net_transfer_count: usize,

    /// Total gross amount
    pub total_gross: Decimal,

    /// Total net amount
    pub total_net: Decimal,

    /// Amount saved
    pub amount_saved: Decimal,

    /// Netting efficiency (0.0 - 1.0)
    pub efficiency: f64,

    /// Number of transfers eliminated
    pub transfers_eliminated: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bank_position() {
        let bank_id = BankId::new("CHASUS33");
        let mut position = BankPosition::new(bank_id, Currency::USD);

        // Bank owes $100
        position.add_obligation(Decimal::new(10000, 2), true);
        assert_eq!(position.total_owed, Decimal::new(10000, 2));
        assert!(position.is_net_payer());

        // Bank receives $150
        position.add_obligation(Decimal::new(15000, 2), false);
        assert_eq!(position.total_receivable, Decimal::new(15000, 2));
        assert!(position.is_net_receiver());

        // Net position: +$50
        assert_eq!(position.net_position, Decimal::new(5000, 2));
    }

    #[test]
    fn test_settlement_batch_efficiency() {
        let batch = SettlementBatch {
            batch_id: Uuid::new_v4(),
            window_start: Utc::now(),
            window_end: Utc::now(),
            currency: Currency::USD,
            payment_count: 100,
            gross_obligations: vec![],
            net_transfers: vec![],
            total_gross_amount: Decimal::new(100000, 2), // $1,000
            total_net_amount: Decimal::new(30000, 2),    // $300
            netting_efficiency: 0.0,
            status: SettlementStatus::Netted,
            created_at: Utc::now(),
            iso20022_files: vec![],
        };

        // Efficiency = (1000 - 300) / 1000 = 0.7 = 70%
        assert_eq!(batch.calculate_efficiency(), 0.7);

        // Savings = $700
        assert_eq!(batch.calculate_savings(), Decimal::new(70000, 2));
    }
}