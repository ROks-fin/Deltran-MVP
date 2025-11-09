use crate::errors::Result;
use crate::models::{NetPosition, NettingResult, Obligation};
use chrono::Utc;
use petgraph::graph::{DiGraph, NodeIndex};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

/// Netting engine for optimizing settlement flows
pub struct NettingEngine {
    efficiency_target: f64,
}

impl NettingEngine {
    pub fn new(efficiency_target: f64) -> Self {
        NettingEngine { efficiency_target }
    }

    /// Calculate net positions for all banks in a clearing window
    pub fn calculate_net_positions(
        &self,
        obligations: &[Obligation],
        clearing_window: i64,
    ) -> Result<NettingResult> {
        info!(
            "Starting netting calculation for {} obligations in window {}",
            obligations.len(),
            clearing_window
        );

        // Track positions per bank per currency
        let mut positions: HashMap<(Uuid, String), NetPositionAccumulator> = HashMap::new();
        let mut gross_amount = Decimal::ZERO;

        // Process all obligations
        for obligation in obligations {
            // Skip non-pending obligations
            if obligation.status != "PENDING" {
                continue;
            }

            // Debtor position (negative - they owe)
            let debtor_key = (obligation.bank_debtor_id, obligation.sent_currency.clone());
            positions
                .entry(debtor_key)
                .or_insert_with(|| NetPositionAccumulator::new(
                    obligation.bank_debtor_id,
                    obligation.sent_currency.clone()
                ))
                .add_outflow(obligation.amount_sent);

            // Creditor position (positive - they are owed)
            let creditor_key = (obligation.bank_creditor_id, obligation.credited_currency.clone());
            positions
                .entry(creditor_key)
                .or_insert_with(|| NetPositionAccumulator::new(
                    obligation.bank_creditor_id,
                    obligation.credited_currency.clone()
                ))
                .add_inflow(obligation.amount_credited);

            // Track gross amounts
            gross_amount += obligation.amount_sent;
        }

        // Convert to net positions
        let mut net_positions: Vec<NetPosition> = Vec::new();
        let mut net_amount = Decimal::ZERO;

        for ((bank_id, currency), accumulator) in positions {
            let net_position = NetPosition {
                bank_id,
                currency,
                net_amount: accumulator.get_net_amount(),
                gross_inflow: accumulator.inflow,
                gross_outflow: accumulator.outflow,
                clearing_window,
                calculated_at: Utc::now(),
            };

            // Only add non-zero positions
            if net_position.net_amount != Decimal::ZERO {
                net_amount += net_position.net_amount.abs();
                net_positions.push(net_position);
            }
        }

        // Calculate netting efficiency
        let netting_efficiency = if gross_amount > Decimal::ZERO {
            let efficiency = 1.0 - (net_amount / gross_amount).to_f64().unwrap_or(1.0);
            efficiency.max(0.0).min(1.0)
        } else {
            0.0
        };

        info!(
            "Netting completed: {} obligations -> {} net positions, efficiency: {:.2}%",
            obligations.len(),
            net_positions.len(),
            netting_efficiency * 100.0
        );

        Ok(NettingResult {
            clearing_window,
            total_obligations: obligations.len(),
            net_positions,
            netting_efficiency,
            gross_amount,
            net_amount: net_amount / Decimal::from(2), // Divide by 2 since we count both sides
            calculated_at: Utc::now(),
        })
    }

    /// Optimize settlement paths using graph algorithms
    pub fn optimize_settlement_paths(
        &self,
        net_positions: &[NetPosition],
    ) -> Result<Vec<SettlementPath>> {
        let mut paths = Vec::new();

        // Group by currency
        let mut currency_groups: HashMap<String, Vec<&NetPosition>> = HashMap::new();
        for position in net_positions {
            currency_groups
                .entry(position.currency.clone())
                .or_insert_with(Vec::new)
                .push(position);
        }

        // Process each currency separately
        for (currency, positions) in currency_groups {
            let currency_paths = self.optimize_currency_paths(&currency, positions)?;
            paths.extend(currency_paths);
        }

        Ok(paths)
    }

    /// Optimize paths for a single currency
    fn optimize_currency_paths(
        &self,
        currency: &str,
        positions: Vec<&NetPosition>,
    ) -> Result<Vec<SettlementPath>> {
        let mut paths = Vec::new();

        // Separate payers and receivers
        let mut payers: Vec<(&NetPosition, Decimal)> = Vec::new();
        let mut receivers: Vec<(&NetPosition, Decimal)> = Vec::new();

        for position in positions {
            if position.net_amount < Decimal::ZERO {
                // Negative = needs to pay
                payers.push((position, position.net_amount.abs()));
            } else if position.net_amount > Decimal::ZERO {
                // Positive = needs to receive
                receivers.push((position, position.net_amount));
            }
        }

        // Sort for optimal matching (largest first)
        payers.sort_by(|a, b| b.1.cmp(&a.1));
        receivers.sort_by(|a, b| b.1.cmp(&a.1));

        // Match payers with receivers
        let mut payer_idx = 0;
        let mut receiver_idx = 0;

        while payer_idx < payers.len() && receiver_idx < receivers.len() {
            let (payer, mut payer_amount) = payers[payer_idx];
            let (receiver, mut receiver_amount) = receivers[receiver_idx];

            let transfer_amount = payer_amount.min(receiver_amount);

            paths.push(SettlementPath {
                from_bank_id: payer.bank_id,
                to_bank_id: receiver.bank_id,
                currency: currency.to_string(),
                amount: transfer_amount,
            });

            payer_amount -= transfer_amount;
            receiver_amount -= transfer_amount;

            if payer_amount == Decimal::ZERO {
                payer_idx += 1;
            } else {
                payers[payer_idx].1 = payer_amount;
            }

            if receiver_amount == Decimal::ZERO {
                receiver_idx += 1;
            } else {
                receivers[receiver_idx].1 = receiver_amount;
            }
        }

        Ok(paths)
    }

    /// Detect circular dependencies in obligations
    pub fn detect_circular_dependencies(
        &self,
        obligations: &[Obligation],
    ) -> Vec<Vec<Uuid>> {
        let mut graph = DiGraph::new();
        let mut node_map: HashMap<Uuid, NodeIndex> = HashMap::new();

        // Build graph
        for obligation in obligations {
            let debtor_node = *node_map
                .entry(obligation.bank_debtor_id)
                .or_insert_with(|| graph.add_node(obligation.bank_debtor_id));

            let creditor_node = *node_map
                .entry(obligation.bank_creditor_id)
                .or_insert_with(|| graph.add_node(obligation.bank_creditor_id));

            graph.add_edge(debtor_node, creditor_node, obligation.id);
        }

        // Find cycles using Tarjan's algorithm
        let sccs = petgraph::algo::tarjan_scc(&graph);

        // Extract cycles (SCCs with more than 1 node)
        let mut cycles = Vec::new();
        for scc in sccs {
            if scc.len() > 1 {
                let cycle: Vec<Uuid> = scc
                    .iter()
                    .map(|&idx| graph[idx])
                    .collect();
                cycles.push(cycle);
            }
        }

        if !cycles.is_empty() {
            info!("Detected {} circular dependencies in obligations", cycles.len());
        }

        cycles
    }

    /// Check if netting efficiency meets target
    pub fn meets_efficiency_target(&self, efficiency: f64) -> bool {
        efficiency >= self.efficiency_target
    }
}

/// Helper struct to accumulate net positions
#[allow(dead_code)]
struct NetPositionAccumulator {
    bank_id: Uuid,
    currency: String,
    inflow: Decimal,
    outflow: Decimal,
}

impl NetPositionAccumulator {
    fn new(bank_id: Uuid, currency: String) -> Self {
        NetPositionAccumulator {
            bank_id,
            currency,
            inflow: Decimal::ZERO,
            outflow: Decimal::ZERO,
        }
    }

    fn add_inflow(&mut self, amount: Decimal) {
        self.inflow += amount;
    }

    fn add_outflow(&mut self, amount: Decimal) {
        self.outflow += amount;
    }

    fn get_net_amount(&self) -> Decimal {
        self.inflow - self.outflow
    }
}

/// Settlement path after netting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementPath {
    pub from_bank_id: Uuid,
    pub to_bank_id: Uuid,
    pub currency: String,
    pub amount: Decimal,
}

use serde::{Deserialize, Serialize};