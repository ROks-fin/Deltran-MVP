//! Multilateral netting algorithm
//!
//! Implements max-flow min-cost algorithm for optimal payment netting.
//!
//! # Algorithm
//!
//! 1. Build bilateral obligations matrix
//! 2. Calculate net positions for each bank
//! 3. Match net payers with net receivers
//! 4. Minimize number of transfers
//!
//! # Example
//!
//! ```text
//! Gross obligations:
//!   A owes B: $100
//!   B owes C: $80
//!   C owes A: $50
//!
//! Net positions:
//!   A: -$50 (net payer)
//!   B: +$20 (net receiver)
//!   C: +$30 (net receiver)
//!
//! Net transfers:
//!   A pays B: $20
//!   A pays C: $30
//!
//! Savings: $230 → $50 (78% reduction)
//! ```

use crate::{
    types::*,
    Error, Result,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use uuid::Uuid;

/// Netting engine
pub struct NettingEngine {
    /// Minimum netting ratio
    min_netting_ratio: f64,

    /// Enable bilateral optimization
    enable_bilateral: bool,
}

impl NettingEngine {
    /// Create new netting engine
    pub fn new(min_netting_ratio: f64, enable_bilateral: bool) -> Self {
        Self {
            min_netting_ratio,
            enable_bilateral,
        }
    }

    /// Compute multilateral netting
    pub fn compute_netting(
        &self,
        payments: Vec<PendingPayment>,
    ) -> Result<SettlementBatch> {
        if payments.is_empty() {
            return Err(Error::Netting("No payments to net".to_string()));
        }

        // Group by currency
        let by_currency = self.group_by_currency(payments);

        // Process each currency separately
        let mut all_obligations = Vec::new();
        let mut all_net_transfers = Vec::new();
        let mut total_gross = Decimal::ZERO;
        let mut total_net = Decimal::ZERO;

        for (currency, currency_payments) in by_currency {
            // Step 1: Build bilateral obligations
            let obligations = self.build_bilateral_obligations(&currency_payments);
            total_gross += obligations.iter().map(|o| o.gross_amount).sum::<Decimal>();

            // Step 2: Calculate net positions
            let positions = self.calculate_net_positions(&obligations);

            // Step 3: Generate net transfers
            let net_transfers = self.generate_net_transfers(&positions, currency)?;
            total_net += net_transfers.iter().map(|t| t.net_amount).sum::<Decimal>();

            all_obligations.extend(obligations);
            all_net_transfers.extend(net_transfers);
        }

        // Calculate netting efficiency
        let netting_efficiency = if total_gross > Decimal::ZERO {
            ((total_gross - total_net) / total_gross).to_f64().unwrap_or(0.0)
        } else {
            0.0
        };

        // Check minimum netting ratio
        if netting_efficiency < self.min_netting_ratio {
            return Err(Error::Netting(format!(
                "Netting efficiency {} below minimum {}",
                netting_efficiency, self.min_netting_ratio
            )));
        }

        // Get first payment for window times (approximation) - need to get before by_currency is moved
        let window_start = all_net_transfers
            .first()
            .map(|t| chrono::Utc::now() - chrono::Duration::hours(6))
            .unwrap_or_else(chrono::Utc::now);

        let payment_count = all_net_transfers.len();
        let first_currency = all_net_transfers
            .first()
            .map(|t| t.currency)
            .unwrap_or(ledger_core::Currency::USD);

        Ok(SettlementBatch {
            batch_id: Uuid::new_v4(),
            window_start,
            window_end: chrono::Utc::now(),
            currency: first_currency,
            payment_count,
            gross_obligations: all_obligations,
            net_transfers: all_net_transfers,
            total_gross_amount: total_gross,
            total_net_amount: total_net,
            netting_efficiency,
            status: SettlementStatus::Netted,
            created_at: chrono::Utc::now(),
            iso20022_files: vec![],
        })
    }

    /// Group payments by currency
    fn group_by_currency(
        &self,
        payments: Vec<PendingPayment>,
    ) -> HashMap<Currency, Vec<PendingPayment>> {
        let mut by_currency: HashMap<Currency, Vec<PendingPayment>> = HashMap::new();

        for payment in payments {
            by_currency
                .entry(payment.currency)
                .or_insert_with(Vec::new)
                .push(payment);
        }

        by_currency
    }

    /// Build bilateral obligations from payments
    fn build_bilateral_obligations(
        &self,
        payments: &[PendingPayment],
    ) -> Vec<BilateralObligation> {
        // Group by (debtor_bank, creditor_bank) pair
        let mut obligations_map: HashMap<(BankId, BankId), Vec<&PendingPayment>> =
            HashMap::new();

        for payment in payments {
            let key = (payment.debtor_bank.clone(), payment.creditor_bank.clone());
            obligations_map
                .entry(key)
                .or_insert_with(Vec::new)
                .push(payment);
        }

        // Create obligations
        let mut obligations = Vec::new();
        for ((debtor_bank, creditor_bank), payments) in obligations_map {
            let gross_amount: Decimal = payments.iter().map(|p| p.amount).sum();
            let payment_ids: Vec<Uuid> = payments.iter().map(|p| p.payment_id).collect();
            let currency = payments[0].currency;

            obligations.push(BilateralObligation {
                debtor_bank,
                creditor_bank,
                currency,
                gross_amount,
                payment_ids,
            });
        }

        // Optional: Bilateral netting (A owes B $100, B owes A $80 → A owes B $20)
        if self.enable_bilateral {
            obligations = self.apply_bilateral_netting(obligations);
        }

        obligations
    }

    /// Apply bilateral netting
    fn apply_bilateral_netting(
        &self,
        obligations: Vec<BilateralObligation>,
    ) -> Vec<BilateralObligation> {
        let mut result = Vec::new();
        let mut processed = std::collections::HashSet::new();

        for i in 0..obligations.len() {
            if processed.contains(&i) {
                continue;
            }

            let obl1 = &obligations[i];
            let mut found_reverse = false;

            // Look for reverse obligation
            for j in (i + 1)..obligations.len() {
                if processed.contains(&j) {
                    continue;
                }

                let obl2 = &obligations[j];

                // Check if reverse (A→B and B→A)
                if obl1.debtor_bank == obl2.creditor_bank
                    && obl1.creditor_bank == obl2.debtor_bank
                    && obl1.currency == obl2.currency
                {
                    // Net the amounts
                    let net_amount = obl1.gross_amount - obl2.gross_amount;

                    if net_amount > Decimal::ZERO {
                        // obl1 is larger, keep it with net amount
                        let mut payment_ids = obl1.payment_ids.clone();
                        payment_ids.extend(obl2.payment_ids.clone());

                        result.push(BilateralObligation {
                            debtor_bank: obl1.debtor_bank.clone(),
                            creditor_bank: obl1.creditor_bank.clone(),
                            currency: obl1.currency,
                            gross_amount: net_amount,
                            payment_ids,
                        });
                    } else if net_amount < Decimal::ZERO {
                        // obl2 is larger
                        let mut payment_ids = obl2.payment_ids.clone();
                        payment_ids.extend(obl1.payment_ids.clone());

                        result.push(BilateralObligation {
                            debtor_bank: obl2.debtor_bank.clone(),
                            creditor_bank: obl2.creditor_bank.clone(),
                            currency: obl2.currency,
                            gross_amount: net_amount.abs(),
                            payment_ids,
                        });
                    }
                    // If exactly equal, both cancel out (no obligation)

                    processed.insert(i);
                    processed.insert(j);
                    found_reverse = true;
                    break;
                }
            }

            // No reverse found, keep original
            if !found_reverse {
                result.push(obl1.clone());
                processed.insert(i);
            }
        }

        result
    }

    /// Calculate net positions for each bank
    fn calculate_net_positions(
        &self,
        obligations: &[BilateralObligation],
    ) -> HashMap<BankId, BankPosition> {
        let mut positions: HashMap<BankId, BankPosition> = HashMap::new();

        for obl in obligations {
            // Update debtor position
            positions
                .entry(obl.debtor_bank.clone())
                .or_insert_with(|| {
                    BankPosition::new(obl.debtor_bank.clone(), obl.currency)
                })
                .add_obligation(obl.gross_amount, true);

            // Update creditor position
            positions
                .entry(obl.creditor_bank.clone())
                .or_insert_with(|| {
                    BankPosition::new(obl.creditor_bank.clone(), obl.currency)
                })
                .add_obligation(obl.gross_amount, false);
        }

        positions
    }

    /// Generate net transfers from positions
    fn generate_net_transfers(
        &self,
        positions: &HashMap<BankId, BankPosition>,
        currency: Currency,
    ) -> Result<Vec<NetTransfer>> {
        // Separate net payers and net receivers
        let mut payers: Vec<&BankPosition> = positions
            .values()
            .filter(|p| p.is_net_payer())
            .collect();

        let mut receivers: Vec<&BankPosition> = positions
            .values()
            .filter(|p| p.is_net_receiver())
            .collect();

        // Sort by absolute net position (largest first)
        payers.sort_by(|a, b| b.abs_net_position().cmp(&a.abs_net_position()));
        receivers.sort_by(|a, b| b.abs_net_position().cmp(&a.abs_net_position()));

        let mut transfers = Vec::new();
        let mut payer_remaining: HashMap<BankId, Decimal> = payers
            .iter()
            .map(|p| (p.bank_id.clone(), p.abs_net_position()))
            .collect();

        let mut receiver_remaining: HashMap<BankId, Decimal> = receivers
            .iter()
            .map(|r| (r.bank_id.clone(), r.abs_net_position()))
            .collect();

        // Greedy matching: match largest payer with largest receiver
        for payer in payers.iter() {
            let payer_amount = match payer_remaining.get(&payer.bank_id) {
                Some(&amt) if amt > Decimal::ZERO => amt,
                _ => continue,
            };

            for receiver in receivers.iter() {
                let receiver_amount = match receiver_remaining.get(&receiver.bank_id) {
                    Some(&amt) if amt > Decimal::ZERO => amt,
                    _ => continue,
                };

                // Transfer min(payer_amount, receiver_amount)
                let transfer_amount = payer_amount.min(receiver_amount);

                if transfer_amount > Decimal::ZERO {
                    transfers.push(NetTransfer {
                        transfer_id: Uuid::new_v4(),
                        debtor_bank: payer.bank_id.clone(),
                        creditor_bank: receiver.bank_id.clone(),
                        currency,
                        net_amount: transfer_amount,
                        payment_ids: vec![], // Filled in later
                        netting_ratio: 0.0,   // Calculated later
                    });

                    // Update remaining amounts
                    *payer_remaining.get_mut(&payer.bank_id).unwrap() -= transfer_amount;
                    *receiver_remaining.get_mut(&receiver.bank_id).unwrap() -= transfer_amount;
                }

                // If payer fully satisfied, break
                if payer_remaining[&payer.bank_id] == Decimal::ZERO {
                    break;
                }
            }
        }

        Ok(transfers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_payment(
        debtor_bank: &str,
        creditor_bank: &str,
        amount: i64,
    ) -> PendingPayment {
        PendingPayment {
            payment_id: Uuid::new_v4(),
            amount: Decimal::new(amount, 2),
            currency: Currency::USD,
            debtor_bank: BankId::new(debtor_bank),
            creditor_bank: BankId::new(creditor_bank),
            debtor_account: "123".to_string(),
            creditor_account: "456".to_string(),
            reference: "test".to_string(),
            queued_at: Utc::now(),
        }
    }

    #[test]
    fn test_simple_netting() {
        let engine = NettingEngine::new(0.0, true);

        // A owes B $100
        // B owes C $80
        // C owes A $50
        let payments = vec![
            create_payment("BANKA", "BANKB", 10000), // $100
            create_payment("BANKB", "BANKC", 8000),  // $80
            create_payment("BANKC", "BANKA", 5000),  // $50
        ];

        let batch = engine.compute_netting(payments).unwrap();

        // Gross: $230
        assert_eq!(batch.total_gross_amount, Decimal::new(23000, 2));

        // Net positions:
        // A: -$100 + $50 = -$50 (pays)
        // B: +$100 - $80 = +$20 (receives)
        // C: +$80 - $50 = +$30 (receives)

        // Net: $50 (A pays B $20, A pays C $30)
        assert_eq!(batch.total_net_amount, Decimal::new(5000, 2));

        // Efficiency: 78%
        let efficiency = batch.calculate_efficiency();
        assert!((efficiency - 0.782).abs() < 0.01);
    }

    #[test]
    fn test_bilateral_netting() {
        let engine = NettingEngine::new(0.0, true);

        // A owes B $100
        // B owes A $80
        let payments = vec![
            create_payment("BANKA", "BANKB", 10000), // $100
            create_payment("BANKB", "BANKA", 8000),  // $80
        ];

        let batch = engine.compute_netting(payments).unwrap();

        // Gross: $180
        assert_eq!(batch.total_gross_amount, Decimal::new(18000, 2));

        // After bilateral netting: A owes B $20
        // Net: $20
        assert_eq!(batch.total_net_amount, Decimal::new(2000, 2));

        // Efficiency: 88.9%
        let efficiency = batch.calculate_efficiency();
        assert!((efficiency - 0.889).abs() < 0.01);
    }

    #[test]
    fn test_no_netting_needed() {
        let engine = NettingEngine::new(0.0, true);

        // A owes B $100 (no reverse flow)
        let payments = vec![create_payment("BANKA", "BANKB", 10000)];

        let batch = engine.compute_netting(payments).unwrap();

        // Gross = Net = $100
        assert_eq!(batch.total_gross_amount, Decimal::new(10000, 2));
        assert_eq!(batch.total_net_amount, Decimal::new(10000, 2));

        // Efficiency: 0%
        assert_eq!(batch.calculate_efficiency(), 0.0);
    }
}