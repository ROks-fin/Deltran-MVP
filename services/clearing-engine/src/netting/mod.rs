// Multi-currency Netting Module
// Implements directed graph-based netting for each currency

pub mod graph_builder;
pub mod calculator;
pub mod optimizer;

use crate::errors::ClearingError;
use crate::models::NetPosition;
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

/// Currency-specific netting graph
pub type CurrencyGraph = petgraph::Graph<BankNode, ObligationEdge, petgraph::Directed>;

/// Bank node in the clearing graph
#[derive(Debug, Clone)]
pub struct BankNode {
    pub bank_id: Uuid,
    pub bank_code: String,
    pub net_position: Decimal,
}

/// Obligation edge between two banks
#[derive(Debug, Clone)]
pub struct ObligationEdge {
    pub obligation_ids: Vec<Uuid>,
    pub amount: Decimal,
    pub count: usize,
}

/// Multi-currency netting engine
pub struct NettingEngine {
    /// Separate graph for each currency
    graphs: HashMap<String, CurrencyGraph>,
    /// Window ID being processed
    window_id: i64,
}

impl NettingEngine {
    pub fn new(window_id: i64) -> Self {
        Self {
            graphs: HashMap::new(),
            window_id,
        }
    }

    /// Add obligation to the appropriate currency graph
    pub fn add_obligation(
        &mut self,
        currency: String,
        payer_id: Uuid,
        payee_id: Uuid,
        amount: Decimal,
        obligation_id: Uuid,
    ) -> Result<(), ClearingError> {
        // Ensure graph exists for currency
        let graph = self.graphs.entry(currency.clone()).or_insert_with(|| {
            petgraph::Graph::new()
        });

        // Find or create payer node
        let payer_idx = graph_builder::find_or_create_node(
            graph,
            payer_id,
            format!("BANK_{}", payer_id),
        );

        // Find or create payee node
        let payee_idx = graph_builder::find_or_create_node(
            graph,
            payee_id,
            format!("BANK_{}", payee_id),
        );

        // Add or update edge
        graph_builder::add_or_update_edge(
            graph,
            payer_idx,
            payee_idx,
            amount,
            obligation_id,
        );

        Ok(())
    }

    /// Calculate net positions for all currencies
    pub fn calculate_net_positions(&self) -> Result<Vec<NetPosition>, ClearingError> {
        let mut all_positions = Vec::new();

        for (currency, graph) in &self.graphs {
            let positions = calculator::calculate_positions(
                graph,
                currency,
                self.window_id,
            )?;
            all_positions.extend(positions);
        }

        Ok(all_positions)
    }

    /// Optimize netting by detecting and eliminating cycles
    pub fn optimize(&mut self) -> Result<OptimizerStats, ClearingError> {
        let mut total_cycles = 0;
        let mut total_eliminated = Decimal::ZERO;

        for (currency, graph) in &mut self.graphs {
            let stats = optimizer::optimize_graph(graph, currency)?;
            total_cycles += stats.cycles_found;
            total_eliminated = total_eliminated
                .checked_add(stats.amount_eliminated)
                .ok_or(ClearingError::CalculationOverflow)?;
        }

        Ok(OptimizerStats {
            cycles_found: total_cycles,
            amount_eliminated: total_eliminated,
        })
    }

    /// Get summary statistics
    pub fn get_stats(&self) -> NettingStats {
        let mut stats = NettingStats::default();

        for graph in self.graphs.values() {
            stats.total_banks += graph.node_count();
            stats.total_edges += graph.edge_count();
        }

        stats.currencies_count = self.graphs.len();
        stats
    }
}

#[derive(Debug, Clone)]
pub struct OptimizerStats {
    pub cycles_found: usize,
    pub amount_eliminated: Decimal,
}

#[derive(Debug, Clone, Default)]
pub struct NettingStats {
    pub currencies_count: usize,
    pub total_banks: usize,
    pub total_edges: usize,
}
