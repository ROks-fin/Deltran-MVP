// Netting Calculator Module - Calculates net positions from graph

use super::{graph_builder, CurrencyGraph};
use crate::errors::ClearingError;
use crate::models::NetPosition;
use chrono::Utc;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

/// Calculate net positions for all bank pairs in a currency graph
pub fn calculate_positions(
    graph: &CurrencyGraph,
    currency: &str,
    window_id: i64,
) -> Result<Vec<NetPosition>, ClearingError> {
    let mut positions = Vec::new();
    let mut processed_pairs: HashMap<(Uuid, Uuid), bool> = HashMap::new();

    // First, update all net positions in nodes
    let mut graph_mut = graph.clone();
    graph_builder::update_net_positions(&mut graph_mut);

    // Iterate through all edges to find bank pairs
    for edge_ref in graph.edge_references() {
        let source_idx = edge_ref.source();
        let target_idx = edge_ref.target();

        let source_node = graph.node_weight(source_idx)
            .ok_or(ClearingError::NodeNotFound)?;
        let target_node = graph.node_weight(target_idx)
            .ok_or(ClearingError::NodeNotFound)?;

        let bank_a = source_node.bank_id;
        let bank_b = target_node.bank_id;

        // Create canonical pair (sorted by UUID to avoid duplicates)
        let pair = if bank_a < bank_b {
            (bank_a, bank_b)
        } else {
            (bank_b, bank_a)
        };

        // Skip if already processed
        if processed_pairs.contains_key(&pair) {
            continue;
        }

        // Calculate bilateral position
        let position = calculate_bilateral_position(
            graph,
            source_idx,
            target_idx,
            currency,
            window_id,
        )?;

        positions.push(position);
        processed_pairs.insert(pair, true);
    }

    Ok(positions)
}

/// Calculate bilateral net position between two banks
fn calculate_bilateral_position(
    graph: &CurrencyGraph,
    node_a: NodeIndex,
    node_b: NodeIndex,
    currency: &str,
    window_id: i64,
) -> Result<NetPosition, ClearingError> {
    let bank_a = graph.node_weight(node_a)
        .ok_or(ClearingError::NodeNotFound)?;
    let bank_b = graph.node_weight(node_b)
        .ok_or(ClearingError::NodeNotFound)?;

    // Get flows A -> B
    let a_to_b = graph.find_edge(node_a, node_b)
        .and_then(|e| graph.edge_weight(e))
        .map(|e| e.amount)
        .unwrap_or(Decimal::ZERO);

    // Get flows B -> A
    let b_to_a = graph.find_edge(node_b, node_a)
        .and_then(|e| graph.edge_weight(e))
        .map(|e| e.amount)
        .unwrap_or(Decimal::ZERO);

    // Calculate net amount
    let net_amount = a_to_b
        .checked_sub(b_to_a)
        .ok_or(ClearingError::CalculationUnderflow)?
        .abs();

    // Determine direction and participants
    let (net_direction, net_payer_id, net_receiver_id) = if a_to_b > b_to_a {
        ("A_TO_B".to_string(), Some(bank_a.bank_id), Some(bank_b.bank_id))
    } else if b_to_a > a_to_b {
        ("B_TO_A".to_string(), Some(bank_b.bank_id), Some(bank_a.bank_id))
    } else {
        ("BALANCED".to_string(), None, None)
    };

    // Count obligations netted
    let obligations_a_to_b = graph.find_edge(node_a, node_b)
        .and_then(|e| graph.edge_weight(e))
        .map(|e| e.count)
        .unwrap_or(0);

    let obligations_b_to_a = graph.find_edge(node_b, node_a)
        .and_then(|e| graph.edge_weight(e))
        .map(|e| e.count)
        .unwrap_or(0);

    let total_obligations = obligations_a_to_b + obligations_b_to_a;

    // Calculate gross amount
    let gross_amount = a_to_b
        .checked_add(b_to_a)
        .ok_or(ClearingError::CalculationOverflow)?;

    // Calculate amount saved
    let amount_saved = gross_amount
        .checked_sub(net_amount)
        .ok_or(ClearingError::CalculationUnderflow)?;

    // Calculate netting ratio
    let netting_ratio = if gross_amount > Decimal::ZERO {
        net_amount
            .checked_div(gross_amount)
            .unwrap_or(Decimal::ZERO)
    } else {
        Decimal::ZERO
    };

    // Create bank pair hash
    let bank_pair_hash = create_bank_pair_hash(bank_a.bank_id, bank_b.bank_id);

    Ok(NetPosition {
        id: Uuid::new_v4(),
        window_id,
        bank_pair_hash,
        bank_a_id: bank_a.bank_id,
        bank_b_id: bank_b.bank_id,
        currency: currency.to_string(),
        gross_debit_a_to_b: a_to_b,
        gross_credit_b_to_a: b_to_a,
        net_amount,
        net_direction,
        net_payer_id,
        net_receiver_id,
        obligations_netted: total_obligations as i32,
        netting_ratio,
        amount_saved,
        created_at: Utc::now(),
    })
}

/// Create deterministic hash for bank pair
fn create_bank_pair_hash(bank_a: Uuid, bank_b: Uuid) -> String {
    let (first, second) = if bank_a < bank_b {
        (bank_a, bank_b)
    } else {
        (bank_b, bank_a)
    };

    format!("{}:{}", first, second)
}

/// Calculate overall netting efficiency for a graph
pub fn calculate_efficiency(graph: &CurrencyGraph) -> Result<Decimal, ClearingError> {
    let gross = graph_builder::calculate_gross_value(graph);
    let net = graph_builder::calculate_net_value(graph);

    if gross == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    let saved = gross
        .checked_sub(net)
        .ok_or(ClearingError::CalculationUnderflow)?;

    let efficiency = saved
        .checked_div(gross)
        .ok_or(ClearingError::DivisionByZero)?;

    // Convert to percentage
    efficiency
        .checked_mul(Decimal::from(100))
        .ok_or(ClearingError::CalculationOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::netting::graph_builder;

    #[test]
    fn test_bilateral_calculation() {
        let mut graph = petgraph::Graph::new();
        let bank_a_id = Uuid::new_v4();
        let bank_b_id = Uuid::new_v4();

        let idx_a = graph_builder::find_or_create_node(
            &mut graph,
            bank_a_id,
            "BANK_A".to_string(),
        );

        let idx_b = graph_builder::find_or_create_node(
            &mut graph,
            bank_b_id,
            "BANK_B".to_string(),
        );

        // A owes B: 100
        graph_builder::add_or_update_edge(
            &mut graph,
            idx_a,
            idx_b,
            Decimal::from(100),
            Uuid::new_v4(),
        );

        // B owes A: 30
        graph_builder::add_or_update_edge(
            &mut graph,
            idx_b,
            idx_a,
            Decimal::from(30),
            Uuid::new_v4(),
        );

        let position = calculate_bilateral_position(
            &graph,
            idx_a,
            idx_b,
            "USD",
            1,
        ).unwrap();

        assert_eq!(position.net_amount, Decimal::from(70));
        assert_eq!(position.amount_saved, Decimal::from(60));
        assert_eq!(position.net_payer_id, Some(bank_a_id));
        assert_eq!(position.net_receiver_id, Some(bank_b_id));
    }

    #[test]
    fn test_efficiency_calculation() {
        let mut graph = petgraph::Graph::new();
        let bank_a = Uuid::new_v4();
        let bank_b = Uuid::new_v4();

        let idx_a = graph_builder::find_or_create_node(&mut graph, bank_a, "A".to_string());
        let idx_b = graph_builder::find_or_create_node(&mut graph, bank_b, "B".to_string());

        graph_builder::add_or_update_edge(&mut graph, idx_a, idx_b, Decimal::from(100), Uuid::new_v4());
        graph_builder::add_or_update_edge(&mut graph, idx_b, idx_a, Decimal::from(80), Uuid::new_v4());

        let efficiency = calculate_efficiency(&graph).unwrap();

        // Gross = 180, Net = 20, Saved = 160, Efficiency = 88.89%
        assert!(efficiency > Decimal::from(88) && efficiency < Decimal::from(89));
    }
}
