//! Settlement window management
//!
//! Manages time-based settlement windows with configurable schedules.
//!
//! # Design
//!
//! Settlement windows run on a schedule (e.g., 4 times per day):
//! - 00:00 UTC
//! - 06:00 UTC
//! - 12:00 UTC
//! - 18:00 UTC
//!
//! Each window:
//! 1. Collects pending payments from ledger
//! 2. Triggers netting computation
//! 3. Generates ISO 20022 files
//! 4. Records settlement batch

use crate::{types::*, Error, Result};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Settlement window
#[derive(Debug, Clone)]
pub struct SettlementWindow {
    /// Window ID
    pub window_id: Uuid,

    /// Window start time
    pub start_time: DateTime<Utc>,

    /// Window end time
    pub end_time: DateTime<Utc>,

    /// Currency
    pub currency: Currency,

    /// Window status
    pub status: WindowStatus,
}

/// Window status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowStatus {
    /// Window open, collecting payments
    Open,
    /// Window closed, ready for settlement
    Closed,
    /// Settlement in progress
    Settling,
    /// Settlement completed
    Completed,
    /// Settlement failed
    Failed,
}

/// Window manager
pub struct WindowManager {
    /// Current windows (by currency)
    windows: Arc<Mutex<std::collections::HashMap<Currency, SettlementWindow>>>,

    /// Window duration
    duration: Duration,

    /// Minimum payments for settlement
    min_payments: usize,

    /// Maximum payments per window
    max_payments: usize,
}

impl WindowManager {
    /// Create new window manager
    pub fn new(
        duration_seconds: u64,
        min_payments: usize,
        max_payments: usize,
    ) -> Self {
        Self {
            windows: Arc::new(Mutex::new(std::collections::HashMap::new())),
            duration: Duration::seconds(duration_seconds as i64),
            min_payments,
            max_payments,
        }
    }

    /// Open a new settlement window
    pub async fn open_window(&self, currency: Currency) -> Result<SettlementWindow> {
        let mut windows = self.windows.lock().await;

        // Check if window already open for this currency
        if let Some(existing) = windows.get(&currency) {
            if matches!(existing.status, WindowStatus::Open) {
                return Err(Error::Window(format!(
                    "Window already open for {}",
                    currency
                )));
            }
        }

        let now = Utc::now();
        let window = SettlementWindow {
            window_id: Uuid::new_v4(),
            start_time: now,
            end_time: now + self.duration,
            currency,
            status: WindowStatus::Open,
        };

        windows.insert(currency, window.clone());
        tracing::info!(
            "Opened settlement window {} for {} (ends at {})",
            window.window_id,
            currency,
            window.end_time
        );

        Ok(window)
    }

    /// Close window (ready for settlement)
    pub async fn close_window(&self, currency: Currency) -> Result<SettlementWindow> {
        let mut windows = self.windows.lock().await;

        let window = windows
            .get_mut(&currency)
            .ok_or_else(|| Error::Window(format!("No window open for {}", currency)))?;

        if !matches!(window.status, WindowStatus::Open) {
            return Err(Error::Window(format!(
                "Window {} not open (status: {:?})",
                window.window_id, window.status
            )));
        }

        window.status = WindowStatus::Closed;
        tracing::info!("Closed settlement window {} for {}", window.window_id, currency);

        Ok(window.clone())
    }

    /// Mark window as settling
    pub async fn mark_settling(&self, currency: Currency) -> Result<()> {
        let mut windows = self.windows.lock().await;

        let window = windows
            .get_mut(&currency)
            .ok_or_else(|| Error::Window(format!("No window for {}", currency)))?;

        window.status = WindowStatus::Settling;
        tracing::info!("Window {} settling", window.window_id);

        Ok(())
    }

    /// Mark window as completed
    pub async fn mark_completed(&self, currency: Currency) -> Result<()> {
        let mut windows = self.windows.lock().await;

        let window = windows
            .get_mut(&currency)
            .ok_or_else(|| Error::Window(format!("No window for {}", currency)))?;

        window.status = WindowStatus::Completed;
        tracing::info!("Window {} completed", window.window_id);

        Ok(())
    }

    /// Mark window as failed
    pub async fn mark_failed(&self, currency: Currency, error: String) -> Result<()> {
        let mut windows = self.windows.lock().await;

        let window = windows
            .get_mut(&currency)
            .ok_or_else(|| Error::Window(format!("No window for {}", currency)))?;

        window.status = WindowStatus::Failed;
        tracing::error!("Window {} failed: {}", window.window_id, error);

        Ok(())
    }

    /// Get current window for currency
    pub async fn get_window(&self, currency: Currency) -> Option<SettlementWindow> {
        let windows = self.windows.lock().await;
        windows.get(&currency).cloned()
    }

    /// Check if window should be closed
    pub async fn should_close_window(&self, currency: Currency) -> bool {
        let windows = self.windows.lock().await;

        if let Some(window) = windows.get(&currency) {
            if matches!(window.status, WindowStatus::Open) {
                return Utc::now() >= window.end_time;
            }
        }

        false
    }

    /// Get all open windows
    pub async fn get_open_windows(&self) -> Vec<SettlementWindow> {
        let windows = self.windows.lock().await;
        windows
            .values()
            .filter(|w| matches!(w.status, WindowStatus::Open))
            .cloned()
            .collect()
    }
}

/// Settlement scheduler
pub struct SettlementScheduler {
    /// Window manager
    window_manager: Arc<WindowManager>,

    /// Currencies to track
    currencies: Vec<Currency>,
}

impl SettlementScheduler {
    /// Create new scheduler
    pub fn new(window_manager: Arc<WindowManager>, currencies: Vec<Currency>) -> Self {
        Self {
            window_manager,
            currencies,
        }
    }

    /// Start scheduler
    pub async fn start(self: Arc<Self>) -> Result<()> {
        tracing::info!("Starting settlement scheduler");

        // Open initial windows
        for currency in &self.currencies {
            self.window_manager.open_window(*currency).await?;
        }

        // Check windows every minute
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

        loop {
            interval.tick().await;

            for currency in &self.currencies {
                if self.window_manager.should_close_window(*currency).await {
                    tracing::info!("Window expired for {}, triggering settlement", currency);

                    // Close current window
                    if let Err(e) = self.window_manager.close_window(*currency).await {
                        tracing::error!("Failed to close window for {}: {}", currency, e);
                        continue;
                    }

                    // TODO: Trigger settlement here
                    // For now, just mark completed and open new window
                    self.window_manager.mark_completed(*currency).await?;

                    // Open new window
                    if let Err(e) = self.window_manager.open_window(*currency).await {
                        tracing::error!("Failed to open new window for {}: {}", currency, e);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_open_close_window() {
        let manager = WindowManager::new(3600, 10, 10000);

        // Open window
        let window = manager.open_window(Currency::USD).await.unwrap();
        assert_eq!(window.currency, Currency::USD);
        assert!(matches!(window.status, WindowStatus::Open));

        // Can't open duplicate
        let result = manager.open_window(Currency::USD).await;
        assert!(result.is_err());

        // Close window
        let closed = manager.close_window(Currency::USD).await.unwrap();
        assert!(matches!(closed.status, WindowStatus::Closed));
    }

    #[tokio::test]
    async fn test_window_expiry() {
        // 1 second window
        let manager = WindowManager::new(1, 10, 10000);

        manager.open_window(Currency::USD).await.unwrap();

        // Not expired yet
        assert!(!manager.should_close_window(Currency::USD).await);

        // Wait for expiry
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Now expired
        assert!(manager.should_close_window(Currency::USD).await);
    }

    #[tokio::test]
    async fn test_multiple_currencies() {
        let manager = WindowManager::new(3600, 10, 10000);

        // Open windows for different currencies
        manager.open_window(Currency::USD).await.unwrap();
        manager.open_window(Currency::EUR).await.unwrap();
        manager.open_window(Currency::GBP).await.unwrap();

        let open_windows = manager.get_open_windows().await;
        assert_eq!(open_windows.len(), 3);
    }
}