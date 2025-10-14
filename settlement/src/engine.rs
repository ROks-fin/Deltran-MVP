//! Main settlement engine
//!
//! Orchestrates netting, window management, and ISO 20022 generation.

use crate::{
    config::Config,
    iso20022::Iso20022Generator,
    netting::NettingEngine,
    types::*,
    window::{SettlementScheduler, WindowManager},
    Error, Result,
};
use ledger_core::Ledger;
use std::sync::Arc;
use uuid::Uuid;

/// Settlement engine
pub struct SettlementEngine {
    /// Ledger core
    ledger: Arc<Ledger>,

    /// Netting engine
    netting: NettingEngine,

    /// Window manager
    window_manager: Arc<WindowManager>,

    /// ISO 20022 generator
    iso20022: Iso20022Generator,

    /// Configuration
    config: Config,
}

impl SettlementEngine {
    /// Create new settlement engine
    pub async fn new(config: Config) -> Result<Self> {
        // Open ledger
        let ledger_config = ledger_core::Config {
            data_dir: config.ledger_data_dir.clone(),
            ..Default::default()
        };
        let ledger = Arc::new(Ledger::open(ledger_config).await?);

        // Create netting engine
        let netting = NettingEngine::new(
            config.netting.min_netting_ratio,
            config.netting.enable_bilateral_optimization,
        );

        // Create window manager
        let window_manager = Arc::new(WindowManager::new(
            config.window.duration_seconds,
            config.window.min_payments,
            config.window.max_payments,
        ));

        // Create ISO 20022 generator
        let iso20022 = Iso20022Generator::new(
            config.iso20022.sender_bic.clone(),
            config.iso20022.output_dir.clone(),
            config.iso20022.pretty_print,
        );

        Ok(Self {
            ledger,
            netting,
            window_manager,
            iso20022,
            config,
        })
    }

    /// Run settlement for a specific window
    pub async fn run_settlement_window(&self) -> Result<SettlementBatch> {
        tracing::info!("Starting settlement window");

        // Step 1: Get pending payments from ledger
        let pending_payments = self.get_pending_payments().await?;

        if pending_payments.is_empty() {
            return Err(Error::Window("No pending payments".to_string()));
        }

        tracing::info!("Found {} pending payments", pending_payments.len());

        // Step 2: Compute netting
        let mut batch = self.netting.compute_netting(pending_payments)?;

        tracing::info!(
            "Netting complete: {} gross → {} net ({:.1}% efficiency)",
            batch.total_gross_amount,
            batch.total_net_amount,
            batch.netting_efficiency * 100.0
        );

        // Step 3: Generate ISO 20022 files
        let files = self.iso20022.generate_pacs008(&batch)?;
        batch.iso20022_files = files.clone();
        batch.status = SettlementStatus::FilesGenerated;

        tracing::info!("Generated {} ISO 20022 files", files.len());

        // Step 4: Record batch in ledger
        // TODO: Add settlement batch recording to ledger

        tracing::info!("Settlement window complete: batch {}", batch.batch_id);

        Ok(batch)
    }

    /// Get pending payments from ledger
    async fn get_pending_payments(&self) -> Result<Vec<PendingPayment>> {
        // TODO: Query ledger for payments in status QueuedForSettlement
        // For now, return empty vec (will be implemented when ledger API is ready)

        // This is a placeholder - in production, this would:
        // 1. Query ledger for events with status QueuedForSettlement
        // 2. Convert LedgerEvents to PendingPayments
        // 3. Group by currency and time window

        Ok(vec![])
    }

    /// Start settlement scheduler
    pub async fn start_scheduler(self: Arc<Self>) -> Result<()> {
        let currencies = vec![
            ledger_core::types::Currency::USD,
            ledger_core::types::Currency::EUR,
            ledger_core::types::Currency::GBP,
            ledger_core::types::Currency::AED,
            ledger_core::types::Currency::INR,
        ];

        let scheduler = Arc::new(SettlementScheduler::new(
            self.window_manager.clone(),
            currencies,
        ));

        scheduler.start().await
    }

    /// Get settlement statistics
    pub async fn get_statistics(&self, batch_id: Uuid) -> Result<NettingStats> {
        // TODO: Query ledger for batch statistics

        Ok(NettingStats {
            bank_count: 0,
            gross_payment_count: 0,
            net_transfer_count: 0,
            total_gross: rust_decimal::Decimal::ZERO,
            total_net: rust_decimal::Decimal::ZERO,
            amount_saved: rust_decimal::Decimal::ZERO,
            efficiency: 0.0,
            transfers_eliminated: 0,
        })
    }

    /// Shutdown engine
    pub async fn shutdown(self) -> Result<()> {
        tracing::info!("Shutting down settlement engine");
        // Ledger shutdown would happen here if needed
        // Arc<Ledger> doesn't have shutdown method in current implementation
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.ledger_data_dir = temp_dir.path().join("ledger");
        config.iso20022.output_dir = temp_dir.path().join("iso20022");

        let engine = SettlementEngine::new(config).await.unwrap();
        engine.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_settlement_with_no_payments() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.ledger_data_dir = temp_dir.path().join("ledger");
        config.iso20022.output_dir = temp_dir.path().join("iso20022");

        let engine = SettlementEngine::new(config).await.unwrap();

        // Should fail with no payments
        let result = engine.run_settlement_window().await;
        assert!(result.is_err());

        engine.shutdown().await.unwrap();
    }
}