// Graph Builder Module - Constructs directed graphs for each currency

use super::{BankNode, CurrencyGraph, ObligationEdge};
use petgraph::graph::NodeIndex;
use rust_decimal::Decimal;
use uuid::Uuid;

/// Find existing node or create new one for a bank
pub fn find_or_create_node(
    graph: &mut CurrencyGraph,
    bank_id: Uuid,
    bank_code: String,
) -> NodeIndex {
    // Search for existing node
    for node_idx in graph.node_indices() {
        if let Some(node) = graph.node_weight(node_idx) {
            if node.bank_id == bank_id {
                return node_idx;
            }
        }
    }

    // Create new node if not found
    graph.add_node(BankNode {
        bank_id,
        bank_code,
        net_position: Decimal::ZERO,
    })
}

/// Add new edge or update existing edge between two banks
pub fn add_or_update_edge(
    graph: &mut CurrencyGraph,
    from: NodeIndex,
    to: NodeIndex,
    amount: Decimal,
    obligation_id: Uuid,
) {
    // Check if edge already exists
    if let Some(edge_idx) = graph.find_edge(from, to) {
        // Update existing edge
        if let Some(edge) = graph.edge_weight_mut(edge_idx) {
            edge.amount = edge.amount
                .checked_add(amount)
                .unwrap_or(edge.amount); // Fallback to current if overflow
            edge.obligation_ids.push(obligation_id);
            edge.count += 1;
        }
    } else {
        // Create new edge
        graph.add_edge(
            from,
            to,
            ObligationEdge {
                obligation_ids: vec![obligation_id],
                amount,
                count: 1,
            },
        );
    }
}

/// Calculate incoming and outgoing totals for a node
pub fn calculate_node_flows(
    graph: &CurrencyGraph,
    node: NodeIndex,
) -> (Decimal, Decimal) {
    let mut incoming = Decimal::ZERO;
    let mut outgoing = Decimal::ZERO;

    // Calculate incoming (edges pointing TO this node)
    for edge in graph.edges_directed(node, petgraph::Direction::Incoming) {
        incoming = incoming
            .checked_add(edge.weight().amount)
            .unwrap_or(incoming);
    }

    // Calculate outgoing (edges FROM this node)
    for edge in graph.edges_directed(node, petgraph::Direction::Outgoing) {
        outgoing = outgoing
            .checked_add(edge.weight().amount)
            .unwrap_or(outgoing);
    }

    (incoming, outgoing)
}

/// Update net position for all nodes in the graph
pub fn update_net_positions(graph: &mut CurrencyGraph) {
    let node_indices: Vec<_> = graph.node_indices().collect();

    for node_idx in node_indices {
        let (incoming, outgoing) = calculate_node_flows(graph, node_idx);

        // Net position = incoming - outgoing
        // Positive = net receiver, Negative = net payer
        if let Some(node) = graph.node_weight_mut(node_idx) {
            node.net_position = incoming
                .checked_sub(outgoing)
                .unwrap_or(Decimal::ZERO);
        }
    }
}

/// Get total gross value in the graph
pub fn calculate_gross_value(graph: &CurrencyGraph) -> Decimal {
    let mut total = Decimal::ZERO;

    for edge in graph.edge_references() {
        total = total
            .checked_add(edge.weight().amount)
            .unwrap_or(total);
    }

    total
}

/// Get total net value (sum of absolute net positions / 2)
pub fn calculate_net_value(graph: &CurrencyGraph) -> Decimal {
    let mut total = Decimal::ZERO;

    for node_idx in graph.node_indices() {
        if let Some(node) = graph.node_weight(node_idx) {
            total = total
                .checked_add(node.net_position.abs())
                .unwrap_or(total);
        }
    }

    // Divide by 2 since we count both payer and payee
    total
        .checked_div(Decimal::from(2))
        .unwrap_or(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_or_create_node() {
        let mut graph = petgraph::Graph::new();
        let bank_id = Uuid::new_v4();

        let idx1 = find_or_create_node(&mut graph, bank_id, "BANK_A".to_string());
        let idx2 = find_or_create_node(&mut graph, bank_id, "BANK_A".to_string());

        assert_eq!(idx1, idx2);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_add_or_update_edge() {
        let mut graph = petgraph::Graph::new();
        let bank_a = Uuid::new_v4();
        let bank_b = Uuid::new_v4();

        let idx_a = find_or_create_node(&mut graph, bank_a, "BANK_A".to_string());
        let idx_b = find_or_create_node(&mut graph, bank_b, "BANK_B".to_string());

        let ob1 = Uuid::new_v4();
        let ob2 = Uuid::new_v4();

        add_or_update_edge(&mut graph, idx_a, idx_b, Decimal::from(100), ob1);
        add_or_update_edge(&mut graph, idx_a, idx_b, Decimal::from(50), ob2);

        assert_eq!(graph.edge_count(), 1);

        if let Some(edge) = graph.edge_weight(graph.find_edge(idx_a, idx_b).unwrap()) {
            assert_eq!(edge.amount, Decimal::from(150));
            assert_eq!(edge.count, 2);
        }
    }

    #[test]
    fn test_calculate_node_flows() {
        let mut graph = petgraph::Graph::new();
        let bank_a = Uuid::new_v4();
        let bank_b = Uuid::new_v4();
        let bank_c = Uuid::new_v4();

        let idx_a = find_or_create_node(&mut graph, bank_a, "BANK_A".to_string());
        let idx_b = find_or_create_node(&mut graph, bank_b, "BANK_B".to_string());
        let idx_c = find_or_create_node(&mut graph, bank_c, "BANK_C".to_string());

        add_or_update_edge(&mut graph, idx_a, idx_b, Decimal::from(100), Uuid::new_v4());
        add_or_update_edge(&mut graph, idx_c, idx_b, Decimal::from(50), Uuid::new_v4());

        let (incoming, outgoing) = calculate_node_flows(&graph, idx_b);

        assert_eq!(incoming, Decimal::from(150));
        assert_eq!(outgoing, Decimal::ZERO);
    }
}
