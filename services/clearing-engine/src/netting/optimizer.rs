// Optimizer Module - Detects and eliminates cycles to minimize settlements

use super::{CurrencyGraph, OptimizerStats};
use crate::errors::ClearingError;
use petgraph::algo::{is_cyclic_directed, kosaraju_scc};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use rust_decimal::Decimal;
use tracing::{info, warn};

/// Optimize graph by detecting and eliminating cycles
pub fn optimize_graph(
    graph: &mut CurrencyGraph,
    currency: &str,
) -> Result<OptimizerStats, ClearingError> {
    let mut cycles_found = 0;
    let mut amount_eliminated = Decimal::ZERO;

    info!("Starting optimization for currency: {}", currency);

    // Check if graph has cycles
    if !is_cyclic_directed(graph) {
        info!("No cycles detected in {} graph", currency);
        return Ok(OptimizerStats {
            cycles_found: 0,
            amount_eliminated: Decimal::ZERO,
        });
    }

    // Find strongly connected components (SCCs)
    let sccs = kosaraju_scc(graph);

    for scc in sccs {
        if scc.len() <= 1 {
            continue; // Skip single-node components
        }

        // Process cycle
        match process_cycle(graph, &scc) {
            Ok(eliminated) => {
                cycles_found += 1;
                amount_eliminated = amount_eliminated
                    .checked_add(eliminated)
                    .ok_or(ClearingError::CalculationOverflow)?;

                info!(
                    "Eliminated cycle with {} banks, saved: {}",
                    scc.len(),
                    eliminated
                );
            }
            Err(e) => {
                warn!("Failed to process cycle: {:?}", e);
            }
        }
    }

    // Remove zero-value edges
    cleanup_zero_edges(graph);

    info!(
        "Optimization complete: {} cycles eliminated, {} saved",
        cycles_found, amount_eliminated
    );

    Ok(OptimizerStats {
        cycles_found,
        amount_eliminated,
    })
}

/// Process a single cycle and eliminate minimum flow
fn process_cycle(
    graph: &mut CurrencyGraph,
    cycle_nodes: &[NodeIndex],
) -> Result<Decimal, ClearingError> {
    if cycle_nodes.is_empty() {
        return Ok(Decimal::ZERO);
    }

    // Find minimum edge weight in the cycle
    let min_flow = find_minimum_flow(graph, cycle_nodes)?;

    if min_flow == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    // Reduce all edges in the cycle by the minimum flow
    for i in 0..cycle_nodes.len() {
        let from = cycle_nodes[i];
        let to = cycle_nodes[(i + 1) % cycle_nodes.len()];

        if let Some(edge_idx) = graph.find_edge(from, to) {
            if let Some(edge) = graph.edge_weight_mut(edge_idx) {
                edge.amount = edge.amount
                    .checked_sub(min_flow)
                    .unwrap_or(Decimal::ZERO);
            }
        }
    }

    Ok(min_flow.checked_mul(Decimal::from(cycle_nodes.len()))
        .ok_or(ClearingError::CalculationOverflow)?)
}

/// Find minimum flow in a cycle
fn find_minimum_flow(
    graph: &CurrencyGraph,
    cycle_nodes: &[NodeIndex],
) -> Result<Decimal, ClearingError> {
    let mut min_flow = Decimal::MAX;

    for i in 0..cycle_nodes.len() {
        let from = cycle_nodes[i];
        let to = cycle_nodes[(i + 1) % cycle_nodes.len()];

        if let Some(edge_idx) = graph.find_edge(from, to) {
            if let Some(edge) = graph.edge_weight(edge_idx) {
                if edge.amount < min_flow {
                    min_flow = edge.amount;
                }
            }
        } else {
            // If any edge is missing, this isn't a valid cycle
            return Ok(Decimal::ZERO);
        }
    }

    if min_flow == Decimal::MAX {
        Ok(Decimal::ZERO)
    } else {
        Ok(min_flow)
    }
}

/// Remove edges with zero or near-zero amounts
fn cleanup_zero_edges(graph: &mut CurrencyGraph) {
    let threshold = Decimal::new(1, 8); // 0.00000001

    let edges_to_remove: Vec<_> = graph
        .edge_references()
        .filter(|e| e.weight().amount < threshold)
        .map(|e| e.id())
        .collect();

    for edge_id in edges_to_remove {
        graph.remove_edge(edge_id);
    }
}

/// Detect simple 3-node cycles (A→B→C→A)
pub fn detect_simple_cycles(
    graph: &CurrencyGraph,
) -> Vec<Vec<NodeIndex>> {
    let mut cycles = Vec::new();
    let nodes: Vec<_> = graph.node_indices().collect();

    // Check all combinations of 3 nodes
    for i in 0..nodes.len() {
        for j in 0..nodes.len() {
            if i == j {
                continue;
            }
            for k in 0..nodes.len() {
                if k == i || k == j {
                    continue;
                }

                // Check if cycle exists: i→j→k→i
                if graph.find_edge(nodes[i], nodes[j]).is_some()
                    && graph.find_edge(nodes[j], nodes[k]).is_some()
                    && graph.find_edge(nodes[k], nodes[i]).is_some()
                {
                    cycles.push(vec![nodes[i], nodes[j], nodes[k]]);
                }
            }
        }
    }

    cycles
}

/// Calculate potential savings from cycle elimination
pub fn calculate_potential_savings(graph: &CurrencyGraph) -> Decimal {
    let mut total_savings = Decimal::ZERO;

    let cycles = detect_simple_cycles(graph);

    for cycle in cycles {
        if let Ok(min_flow) = find_minimum_flow(graph, &cycle) {
            total_savings = total_savings
                .checked_add(min_flow.checked_mul(Decimal::from(cycle.len())).unwrap_or(Decimal::ZERO))
                .unwrap_or(total_savings);
        }
    }

    total_savings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netting::graph_builder;
    use uuid::Uuid;

    #[test]
    fn test_cycle_detection() {
        let mut graph = petgraph::Graph::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let idx_a = graph_builder::find_or_create_node(&mut graph, a, "A".to_string());
        let idx_b = graph_builder::find_or_create_node(&mut graph, b, "B".to_string());
        let idx_c = graph_builder::find_or_create_node(&mut graph, c, "C".to_string());

        // Create cycle: A→B→C→A
        graph_builder::add_or_update_edge(&mut graph, idx_a, idx_b, Decimal::from(100), Uuid::new_v4());
        graph_builder::add_or_update_edge(&mut graph, idx_b, idx_c, Decimal::from(80), Uuid::new_v4());
        graph_builder::add_or_update_edge(&mut graph, idx_c, idx_a, Decimal::from(90), Uuid::new_v4());

        assert!(is_cyclic_directed(&graph));

        let cycles = detect_simple_cycles(&graph);
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_cycle_elimination() {
        let mut graph = petgraph::Graph::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let idx_a = graph_builder::find_or_create_node(&mut graph, a, "A".to_string());
        let idx_b = graph_builder::find_or_create_node(&mut graph, b, "B".to_string());
        let idx_c = graph_builder::find_or_create_node(&mut graph, c, "C".to_string());

        // Create cycle with min flow of 50
        graph_builder::add_or_update_edge(&mut graph, idx_a, idx_b, Decimal::from(100), Uuid::new_v4());
        graph_builder::add_or_update_edge(&mut graph, idx_b, idx_c, Decimal::from(50), Uuid::new_v4());
        graph_builder::add_or_update_edge(&mut graph, idx_c, idx_a, Decimal::from(75), Uuid::new_v4());

        let cycle = vec![idx_a, idx_b, idx_c];
        let eliminated = process_cycle(&mut graph, &cycle).unwrap();

        // Should eliminate 50 * 3 = 150
        assert_eq!(eliminated, Decimal::from(150));

        // Check remaining edges
        let edge_ab = graph.find_edge(idx_a, idx_b).unwrap();
        assert_eq!(graph.edge_weight(edge_ab).unwrap().amount, Decimal::from(50));

        let edge_bc = graph.find_edge(idx_b, idx_c).unwrap();
        assert_eq!(graph.edge_weight(edge_bc).unwrap().amount, Decimal::ZERO);
    }
}
