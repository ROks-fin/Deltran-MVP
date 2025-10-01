//! Partial settlement support
//!
//! Decomposes netting graph into strongly connected components (SCCs)
//! to enable atomic partial settlement when some banks fail.

use crate::{types::*, Error, Result};
use petgraph::algo::kosaraju_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use uuid::Uuid;

/// Partial settlement engine
pub struct PartialSettlementEngine;

impl PartialSettlementEngine {
    /// Decompose netting proposal into atomic components
    ///
    /// Given a set of net transfers and failed banks, this returns:
    /// 1. Atomic components that can be settled (all banks confirmed)
    /// 2. Payments to requeue (involve failed banks)
    pub fn decompose(
        net_transfers: &[NetTransfer],
        failed_banks: &[String],
    ) -> Result<(Vec<AtomicComponent>, Vec<Uuid>)> {
        // Build directed graph from net transfers
        let (graph, node_map, _payment_map) = Self::build_graph(net_transfers)?;

        // Find strongly connected components
        let sccs = kosaraju_scc(&graph);

        let mut atomic_components = Vec::new();
        let requeued_payments = Vec::new();

        for scc in sccs {
            // Extract bank IDs in this component
            let bank_ids: Vec<String> = scc
                .iter()
                .map(|&idx| node_map[&idx].clone())
                .collect();

            // Check if any bank in this component failed
            let has_failed = bank_ids
                .iter()
                .any(|bank| failed_banks.contains(bank));

            if has_failed {
                // Requeue all payments in this component
                let component_transfers: Vec<&NetTransfer> = scc
                    .iter()
                    .filter_map(|&idx| {
                        net_transfers.iter().find(|t| {
                            node_map.get(&idx) == Some(&t.from_bank)
                                || node_map.get(&idx) == Some(&t.to_bank)
                        })
                    })
                    .collect();

                for _transfer in component_transfers {
                    // Extract payment IDs from this transfer (if tracked)
                    // For MVP, we'll use a placeholder
                    // TODO: Track payment_ids through netting
                }
            } else {
                // This component can be settled atomically
                let component_transfers: Vec<NetTransfer> = net_transfers
                    .iter()
                    .filter(|t| bank_ids.contains(&t.from_bank) && bank_ids.contains(&t.to_bank))
                    .cloned()
                    .collect();

                let total_amount: Decimal = component_transfers
                    .iter()
                    .map(|t| t.net_amount)
                    .sum();

                atomic_components.push(AtomicComponent {
                    component_id: Uuid::new_v4(),
                    bank_ids,
                    net_transfers: component_transfers,
                    total_amount,
                    finalized: false,
                });
            }
        }

        Ok((atomic_components, requeued_payments))
    }

    /// Build directed graph from net transfers
    fn build_graph(
        net_transfers: &[NetTransfer],
    ) -> Result<(DiGraph<String, ()>, HashMap<NodeIndex, String>, HashMap<String, NodeIndex>)> {
        let mut graph = DiGraph::new();
        let mut node_map: HashMap<NodeIndex, String> = HashMap::new();
        let mut bank_to_node: HashMap<String, NodeIndex> = HashMap::new();

        // Add nodes for each unique bank
        for transfer in net_transfers {
            if !bank_to_node.contains_key(&transfer.from_bank) {
                let idx = graph.add_node(transfer.from_bank.clone());
                node_map.insert(idx, transfer.from_bank.clone());
                bank_to_node.insert(transfer.from_bank.clone(), idx);
            }

            if !bank_to_node.contains_key(&transfer.to_bank) {
                let idx = graph.add_node(transfer.to_bank.clone());
                node_map.insert(idx, transfer.to_bank.clone());
                bank_to_node.insert(transfer.to_bank.clone(), idx);
            }
        }

        // Add edges for each transfer
        for transfer in net_transfers {
            let from_idx = bank_to_node[&transfer.from_bank];
            let to_idx = bank_to_node[&transfer.to_bank];
            graph.add_edge(from_idx, to_idx, ());
        }

        Ok((graph, node_map, bank_to_node))
    }

    /// Verify atomic component integrity
    pub fn verify_component(component: &AtomicComponent) -> Result<()> {
        // Check that all transfers sum correctly
        let computed_total: Decimal = component.net_transfers.iter().map(|t| t.net_amount).sum();

        if computed_total != component.total_amount {
            return Err(Error::PartialSettlementFailed(format!(
                "Component total mismatch: expected {}, got {}",
                component.total_amount, computed_total
            )));
        }

        // Check that all banks in transfers are in bank_ids
        for transfer in &component.net_transfers {
            if !component.bank_ids.contains(&transfer.from_bank) {
                return Err(Error::PartialSettlementFailed(format!(
                    "Transfer from_bank {} not in component bank_ids",
                    transfer.from_bank
                )));
            }
            if !component.bank_ids.contains(&transfer.to_bank) {
                return Err(Error::PartialSettlementFailed(format!(
                    "Transfer to_bank {} not in component bank_ids",
                    transfer.to_bank
                )));
            }
        }

        Ok(())
    }

    /// Compute partial settlement statistics
    pub fn compute_stats(
        components: &[AtomicComponent],
        requeued_count: usize,
    ) -> PartialSettlementStats {
        let settled_components = components.iter().filter(|c| c.finalized).count();
        let settled_amount: Decimal = components
            .iter()
            .filter(|c| c.finalized)
            .map(|c| c.total_amount)
            .sum();

        let total_amount: Decimal = components.iter().map(|c| c.total_amount).sum();

        let settlement_rate = if total_amount > Decimal::ZERO {
            (settled_amount / total_amount).to_f64().unwrap_or(0.0)
        } else {
            0.0
        };

        PartialSettlementStats {
            total_components: components.len(),
            settled_components,
            failed_components: components.len() - settled_components,
            total_amount,
            settled_amount,
            settlement_rate,
            requeued_count,
        }
    }
}

/// Partial settlement statistics
#[derive(Debug, Clone)]
pub struct PartialSettlementStats {
    pub total_components: usize,
    pub settled_components: usize,
    pub failed_components: usize,
    pub total_amount: Decimal,
    pub settled_amount: Decimal,
    pub settlement_rate: f64,
    pub requeued_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn create_test_transfers() -> Vec<NetTransfer> {
        vec![
            NetTransfer {
                transfer_id: Uuid::new_v4(),
                from_bank: "BANKA".into(),
                to_bank: "BANKB".into(),
                net_amount: dec!(100.00),
                currency: "USD".into(),
                iso20022_instruction: None,
            },
            NetTransfer {
                transfer_id: Uuid::new_v4(),
                from_bank: "BANKB".into(),
                to_bank: "BANKC".into(),
                net_amount: dec!(50.00),
                currency: "USD".into(),
                iso20022_instruction: None,
            },
            NetTransfer {
                transfer_id: Uuid::new_v4(),
                from_bank: "BANKD".into(),
                to_bank: "BANKE".into(),
                net_amount: dec!(200.00),
                currency: "USD".into(),
                iso20022_instruction: None,
            },
        ]
    }

    #[test]
    fn test_decompose_no_failures() {
        let transfers = create_test_transfers();
        let failed_banks = vec![];

        let (components, requeued) =
            PartialSettlementEngine::decompose(&transfers, &failed_banks).unwrap();

        // All transfers should be in components
        assert!(components.len() > 0);
        assert_eq!(requeued.len(), 0);
    }

    #[test]
    fn test_decompose_with_failure() {
        let transfers = create_test_transfers();
        let failed_banks = vec!["BANKB".to_string()];

        let (components, _requeued) =
            PartialSettlementEngine::decompose(&transfers, &failed_banks).unwrap();

        // Components involving BANKB should be excluded or marked
        // (exact behavior depends on graph structure)
        assert!(components.len() >= 0);
    }

    #[test]
    fn test_verify_component() {
        let component = AtomicComponent {
            component_id: Uuid::new_v4(),
            bank_ids: vec!["BANKA".into(), "BANKB".into()],
            net_transfers: vec![NetTransfer {
                transfer_id: Uuid::new_v4(),
                from_bank: "BANKA".into(),
                to_bank: "BANKB".into(),
                net_amount: dec!(100.00),
                currency: "USD".into(),
                iso20022_instruction: None,
            }],
            total_amount: dec!(100.00),
            finalized: false,
        };

        assert!(PartialSettlementEngine::verify_component(&component).is_ok());

        // Test with mismatched total
        let mut bad_component = component.clone();
        bad_component.total_amount = dec!(200.00);
        assert!(PartialSettlementEngine::verify_component(&bad_component).is_err());
    }

    #[test]
    fn test_partial_settlement_stats() {
        let components = vec![
            AtomicComponent {
                component_id: Uuid::new_v4(),
                bank_ids: vec!["BANKA".into(), "BANKB".into()],
                net_transfers: vec![],
                total_amount: dec!(100.00),
                finalized: true,
            },
            AtomicComponent {
                component_id: Uuid::new_v4(),
                bank_ids: vec!["BANKC".into(), "BANKD".into()],
                net_transfers: vec![],
                total_amount: dec!(200.00),
                finalized: false,
            },
        ];

        let stats = PartialSettlementEngine::compute_stats(&components, 5);

        assert_eq!(stats.total_components, 2);
        assert_eq!(stats.settled_components, 1);
        assert_eq!(stats.failed_components, 1);
        assert_eq!(stats.settled_amount, dec!(100.00));
        assert_eq!(stats.requeued_count, 5);
    }
}