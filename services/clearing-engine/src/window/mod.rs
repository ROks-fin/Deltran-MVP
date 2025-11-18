// Window Manager Module - Manages clearing windows with cron scheduling

pub mod scheduler;
pub mod state_machine;
pub mod grace_period;

use crate::errors::ClearingError;
use crate::models::{ClearingWindow, WindowStatus};
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Window Manager handles clearing window lifecycle
pub struct WindowManager {
    db_pool: Arc<PgPool>,
    current_window: Arc<RwLock<Option<ClearingWindow>>>,
    config: WindowConfig,
}

/// Configuration for clearing windows
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Cron schedule for windows (e.g., "0 0,6,12,18 * * *")
    pub schedule: String,
    /// Grace period duration in minutes
    pub grace_period_minutes: i32,
    /// Window duration in hours
    pub window_duration_hours: i64,
    /// Region for this window manager
    pub region: String,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            schedule: "0 0,6,12,18 * * *".to_string(), // Every 6 hours
            grace_period_minutes: 30,
            window_duration_hours: 6,
            region: "Global".to_string(),
        }
    }
}

impl WindowManager {
    pub fn new(db_pool: Arc<PgPool>, config: WindowConfig) -> Self {
        Self {
            db_pool,
            current_window: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// Create a new clearing window
    pub async fn create_window(&self) -> Result<ClearingWindow, ClearingError> {
        let now = Utc::now();
        let end_time = now + Duration::hours(self.config.window_duration_hours);
        let cutoff_time = end_time - Duration::minutes(self.config.grace_period_minutes as i64);

        let window = ClearingWindow {
            id: 0, // Will be set by database
            window_name: format!("CLEAR_{}_{}", self.config.region, now.format("%Y%m%d_%H%M")),
            start_time: now,
            end_time,
            cutoff_time,
            status: WindowStatus::Open.as_str().to_string(),
            region: self.config.region.clone(),
            transactions_count: 0,
            obligations_count: 0,
            total_gross_value: Decimal::ZERO,
            total_net_value: Decimal::ZERO,
            saved_amount: Decimal::ZERO,
            netting_efficiency: Decimal::ZERO,
            settlement_instructions: None,
            metadata: serde_json::json!({}),
            created_at: now,
            closed_at: None,
            processed_at: None,
            completed_at: None,
            grace_period_seconds: self.config.grace_period_minutes * 60,
            grace_period_started: None,
        };

        // Insert into database
        let inserted = sqlx::query_as::<_, ClearingWindow>(
            r#"
            INSERT INTO clearing_windows (
                window_name, start_time, end_time, cutoff_time, status, region,
                transactions_count, obligations_count, total_gross_value, total_net_value,
                saved_amount, netting_efficiency, metadata, created_at,
                grace_period_seconds
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING *
            "#,
        )
        .bind(&window.window_name)
        .bind(&window.start_time)
        .bind(&window.end_time)
        .bind(&window.cutoff_time)
        .bind(&window.status)
        .bind(&window.region)
        .bind(&window.transactions_count)
        .bind(&window.obligations_count)
        .bind(&window.total_gross_value)
        .bind(&window.total_net_value)
        .bind(&window.saved_amount)
        .bind(&window.netting_efficiency)
        .bind(&window.metadata)
        .bind(&window.created_at)
        .bind(&window.grace_period_seconds)
        .fetch_one(self.db_pool.as_ref())
        .await
        .map_err(|e| ClearingError::DatabaseError(e.to_string()))?;

        // Update current window
        *self.current_window.write().await = Some(inserted.clone());

        Ok(inserted)
    }

    /// Get current active window
    pub async fn get_current_window(&self) -> Option<ClearingWindow> {
        self.current_window.read().await.clone()
    }

    /// Close current window and start grace period
    pub async fn close_window(&self, window_id: i64) -> Result<(), ClearingError> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE clearing_windows
            SET status = $1, closed_at = $2, grace_period_started = $2
            WHERE id = $3
            "#,
        )
        .bind(WindowStatus::Closing.as_str())
        .bind(&now)
        .bind(window_id)
        .execute(self.db_pool.as_ref())
        .await
        .map_err(|e| ClearingError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update window status
    pub async fn update_status(
        &self,
        window_id: i64,
        new_status: WindowStatus,
    ) -> Result<(), ClearingError> {
        sqlx::query(
            r#"
            UPDATE clearing_windows
            SET status = $1
            WHERE id = $2
            "#,
        )
        .bind(new_status.as_str())
        .bind(window_id)
        .execute(self.db_pool.as_ref())
        .await
        .map_err(|e| ClearingError::DatabaseError(e.to_string()))?;

        // Update in-memory state
        if let Some(mut window) = self.current_window.write().await.as_mut() {
            if window.id == window_id {
                window.status = new_status.as_str().to_string();
            }
        }

        Ok(())
    }

    /// Update window metrics after clearing
    pub async fn update_metrics(
        &self,
        window_id: i64,
        obligations_count: i32,
        gross_value: Decimal,
        net_value: Decimal,
        efficiency: Decimal,
    ) -> Result<(), ClearingError> {
        let saved = gross_value
            .checked_sub(net_value)
            .ok_or(ClearingError::CalculationUnderflow)?;

        sqlx::query(
            r#"
            UPDATE clearing_windows
            SET obligations_count = $1,
                total_gross_value = $2,
                total_net_value = $3,
                saved_amount = $4,
                netting_efficiency = $5,
                processed_at = $6
            WHERE id = $7
            "#,
        )
        .bind(obligations_count)
        .bind(&gross_value)
        .bind(&net_value)
        .bind(&saved)
        .bind(&efficiency)
        .bind(&Utc::now())
        .bind(window_id)
        .execute(self.db_pool.as_ref())
        .await
        .map_err(|e| ClearingError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get window by ID
    pub async fn get_window(&self, window_id: i64) -> Result<ClearingWindow, ClearingError> {
        sqlx::query_as::<_, ClearingWindow>(
            "SELECT * FROM clearing_windows WHERE id = $1"
        )
        .bind(window_id)
        .fetch_one(self.db_pool.as_ref())
        .await
        .map_err(|e| ClearingError::DatabaseError(e.to_string()))
    }

    /// List recent windows
    pub async fn list_windows(&self, limit: i64) -> Result<Vec<ClearingWindow>, ClearingError> {
        sqlx::query_as::<_, ClearingWindow>(
            "SELECT * FROM clearing_windows ORDER BY created_at DESC LIMIT $1"
        )
        .bind(limit)
        .fetch_all(self.db_pool.as_ref())
        .await
        .map_err(|e| ClearingError::DatabaseError(e.to_string()))
    }

    /// Check if grace period has expired
    pub fn is_grace_period_expired(&self, window: &ClearingWindow) -> bool {
        if let Some(grace_started) = window.grace_period_started {
            let grace_duration = Duration::seconds(window.grace_period_seconds as i64);
            let now = Utc::now();
            now > grace_started + grace_duration
        } else {
            false
        }
    }

    /// Accept late transaction during grace period
    pub async fn accept_late_transaction(
        &self,
        window_id: i64,
        transaction_id: Uuid,
    ) -> Result<bool, ClearingError> {
        let window = self.get_window(window_id).await?;

        // Check if we're in grace period
        if window.status != WindowStatus::Closing.as_str() {
            return Ok(false);
        }

        if self.is_grace_period_expired(&window) {
            return Ok(false);
        }

        // Accept the transaction (implementation depends on transaction storage)
        // For now, just return success
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WindowConfig::default();
        assert_eq!(config.grace_period_minutes, 30);
        assert_eq!(config.window_duration_hours, 6);
    }

    #[test]
    fn test_grace_period_check() {
        let config = WindowConfig::default();
        let manager = WindowManager::new(
            Arc::new(sqlx::PgPool::connect_lazy("").unwrap()),
            config,
        );

        let mut window = ClearingWindow {
            id: 1,
            window_name: "TEST".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now() + Duration::hours(6),
            cutoff_time: Utc::now() + Duration::hours(5),
            status: WindowStatus::Closing.as_str().to_string(),
            region: "Global".to_string(),
            transactions_count: 0,
            obligations_count: 0,
            total_gross_value: Decimal::ZERO,
            total_net_value: Decimal::ZERO,
            saved_amount: Decimal::ZERO,
            netting_efficiency: Decimal::ZERO,
            settlement_instructions: None,
            metadata: serde_json::json!({}),
            created_at: Utc::now(),
            closed_at: Some(Utc::now()),
            processed_at: None,
            completed_at: None,
            grace_period_seconds: 1800,
            grace_period_started: Some(Utc::now() - Duration::hours(1)),
        };

        assert!(manager.is_grace_period_expired(&window));

        window.grace_period_started = Some(Utc::now());
        assert!(!manager.is_grace_period_expired(&window));
    }
}
