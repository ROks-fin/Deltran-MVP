//! Event-Driven Reporting Service
//!
//! Consumes events from bronze/silver/gold layers and triggers report generation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum EventReporterError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Event processing error: {0}")]
    ProcessingError(String),

    #[error("Channel error: {0}")]
    ChannelError(String),
}

/// Reporting event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReportingEvent {
    /// Transaction completed
    TransactionCompleted {
        transaction_id: Uuid,
        amount: rust_decimal::Decimal,
        currency: String,
        timestamp: DateTime<Utc>,
    },

    /// Settlement batch completed
    SettlementBatchCompleted {
        batch_id: Uuid,
        participant_count: i32,
        total_amount: rust_decimal::Decimal,
        currency: String,
        timestamp: DateTime<Utc>,
    },

    /// Safeguarding event
    SafeguardingEvent {
        client_account: String,
        event_type: String,
        amount: rust_decimal::Decimal,
        currency: String,
        timestamp: DateTime<Utc>,
    },

    /// AML alert
    AmlAlert {
        alert_id: Uuid,
        transaction_id: Option<Uuid>,
        risk_score: rust_decimal::Decimal,
        timestamp: DateTime<Utc>,
    },

    /// Regulatory deadline approaching
    DeadlineApproaching {
        regulation_type: String,
        report_period: String,
        deadline_date: DateTime<Utc>,
        days_remaining: i32,
    },

    /// Report generation requested
    ReportGenerationRequested {
        report_type: String,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
        requester: String,
    },

    /// Report generated
    ReportGenerated {
        report_id: Uuid,
        report_type: String,
        status: String,
        timestamp: DateTime<Utc>,
    },
}

/// Event reporter service
pub struct EventReporter {
    pool: PgPool,
    event_tx: mpsc::UnboundedSender<ReportingEvent>,
    event_rx: Option<mpsc::UnboundedReceiver<ReportingEvent>>,
}

impl EventReporter {
    /// Create new event reporter
    pub fn new(pool: PgPool) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Self {
            pool,
            event_tx,
            event_rx: Some(event_rx),
        }
    }

    /// Get event sender (for publishing events)
    pub fn get_sender(&self) -> mpsc::UnboundedSender<ReportingEvent> {
        self.event_tx.clone()
    }

    /// Start event processing loop
    pub async fn start(mut self) -> Result<(), EventReporterError> {
        let mut event_rx = self.event_rx.take().ok_or_else(|| {
            EventReporterError::ChannelError("Event receiver already taken".to_string())
        })?;

        info!("Event reporter started");

        while let Some(event) = event_rx.recv().await {
            if let Err(e) = self.process_event(event).await {
                error!(error = %e, "Failed to process reporting event");
            }
        }

        info!("Event reporter stopped");
        Ok(())
    }

    /// Process a reporting event
    async fn process_event(&self, event: ReportingEvent) -> Result<(), EventReporterError> {
        match &event {
            ReportingEvent::TransactionCompleted {
                transaction_id,
                amount,
                currency,
                timestamp,
            } => {
                info!(
                    transaction_id = %transaction_id,
                    amount = %amount,
                    currency = currency,
                    "Processing transaction completed event"
                );

                // Insert into bronze layer
                self.insert_tx_event(transaction_id, amount, currency, timestamp).await?;
            }

            ReportingEvent::SettlementBatchCompleted {
                batch_id,
                participant_count,
                total_amount,
                currency,
                timestamp,
            } => {
                info!(
                    batch_id = %batch_id,
                    participants = participant_count,
                    total_amount = %total_amount,
                    "Processing settlement batch completed event"
                );

                self.insert_settlement_event(
                    batch_id,
                    *participant_count,
                    total_amount,
                    currency,
                    timestamp,
                )
                .await?;
            }

            ReportingEvent::SafeguardingEvent {
                client_account,
                event_type,
                amount,
                currency,
                timestamp,
            } => {
                info!(
                    client_account = client_account,
                    event_type = event_type,
                    amount = %amount,
                    "Processing safeguarding event"
                );

                self.insert_safeguarding_event(
                    client_account,
                    event_type,
                    amount,
                    currency,
                    timestamp,
                )
                .await?;
            }

            ReportingEvent::AmlAlert {
                alert_id,
                transaction_id,
                risk_score,
                timestamp,
            } => {
                info!(
                    alert_id = %alert_id,
                    risk_score = %risk_score,
                    "Processing AML alert event"
                );

                self.insert_aml_event(alert_id, transaction_id, risk_score, timestamp)
                    .await?;
            }

            ReportingEvent::DeadlineApproaching {
                regulation_type,
                report_period,
                deadline_date,
                days_remaining,
            } => {
                warn!(
                    regulation_type = regulation_type,
                    report_period = report_period,
                    days_remaining = days_remaining,
                    "Regulatory deadline approaching"
                );

                // Trigger auto-generation if configured
                self.check_auto_generation(regulation_type, report_period).await?;
            }

            ReportingEvent::ReportGenerationRequested {
                report_type,
                period_start,
                period_end,
                requester,
            } => {
                info!(
                    report_type = report_type,
                    requester = requester,
                    "Report generation requested"
                );

                // This would trigger the actual report generation
                // Implementation depends on report type
            }

            ReportingEvent::ReportGenerated {
                report_id,
                report_type,
                status,
                timestamp,
            } => {
                info!(
                    report_id = %report_id,
                    report_type = report_type,
                    status = status,
                    "Report generated"
                );

                self.update_report_status(report_id, status).await?;
            }
        }

        Ok(())
    }

    /// Insert transaction event into bronze layer
    async fn insert_tx_event(
        &self,
        transaction_id: &Uuid,
        amount: &rust_decimal::Decimal,
        currency: &str,
        timestamp: &DateTime<Utc>,
    ) -> Result<(), EventReporterError> {
        sqlx::query(
            r#"
            INSERT INTO bronze.tx_events
            (transaction_id, event_type, event_timestamp, event_data, source_service)
            VALUES ($1, 'transaction_completed', $2, $3, 'protocol-core')
            "#,
        )
        .bind(transaction_id)
        .bind(timestamp)
        .bind(serde_json::json!({
            "amount": amount,
            "currency": currency,
        }))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert settlement event
    async fn insert_settlement_event(
        &self,
        batch_id: &Uuid,
        participant_count: i32,
        total_amount: &rust_decimal::Decimal,
        currency: &str,
        timestamp: &DateTime<Utc>,
    ) -> Result<(), EventReporterError> {
        sqlx::query(
            r#"
            INSERT INTO bronze.settlement_events
            (settlement_batch_id, event_type, event_timestamp, event_data,
             participant_count, total_amount, currency)
            VALUES ($1, 'batch_completed', $2, $3, $4, $5, $6)
            "#,
        )
        .bind(batch_id)
        .bind(timestamp)
        .bind(serde_json::json!({}))
        .bind(participant_count)
        .bind(total_amount)
        .bind(currency)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert safeguarding event
    async fn insert_safeguarding_event(
        &self,
        client_account: &str,
        event_type: &str,
        amount: &rust_decimal::Decimal,
        currency: &str,
        timestamp: &DateTime<Utc>,
    ) -> Result<(), EventReporterError> {
        sqlx::query(
            r#"
            INSERT INTO bronze.safeguarding_events
            (client_account, event_type, event_timestamp, amount, currency, event_data)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(client_account)
        .bind(event_type)
        .bind(timestamp)
        .bind(amount)
        .bind(currency)
        .bind(serde_json::json!({}))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Insert AML event
    async fn insert_aml_event(
        &self,
        alert_id: &Uuid,
        transaction_id: &Option<Uuid>,
        risk_score: &rust_decimal::Decimal,
        timestamp: &DateTime<Utc>,
    ) -> Result<(), EventReporterError> {
        sqlx::query(
            r#"
            INSERT INTO bronze.aml_events
            (transaction_id, event_type, event_timestamp, alert_type,
             risk_score, event_data)
            VALUES ($1, 'alert', $2, 'risk_threshold_exceeded', $3, $4)
            "#,
        )
        .bind(transaction_id)
        .bind(timestamp)
        .bind(risk_score)
        .bind(serde_json::json!({ "alert_id": alert_id }))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Check if auto-generation is configured for a report type
    async fn check_auto_generation(
        &self,
        regulation_type: &str,
        report_period: &str,
    ) -> Result<(), EventReporterError> {
        let auto_enabled: bool = sqlx::query_scalar(
            r#"
            SELECT COALESCE(auto_generate_enabled, false)
            FROM compliance.regulatory_calendar
            WHERE regulation_type = $1
              AND report_period = $2
            LIMIT 1
            "#,
        )
        .bind(regulation_type)
        .bind(report_period)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(false);

        if auto_enabled {
            info!(
                regulation_type = regulation_type,
                report_period = report_period,
                "Auto-generation enabled, triggering report generation"
            );

            // Send report generation event
            let event = ReportingEvent::ReportGenerationRequested {
                report_type: regulation_type.to_string(),
                period_start: Utc::now(), // Placeholder
                period_end: Utc::now(),
                requester: "system_auto".to_string(),
            };

            self.event_tx.send(event).map_err(|e| {
                EventReporterError::ChannelError(format!("Failed to send event: {}", e))
            })?;
        }

        Ok(())
    }

    /// Update report status
    async fn update_report_status(
        &self,
        report_id: &Uuid,
        status: &str,
    ) -> Result<(), EventReporterError> {
        sqlx::query(
            r#"
            UPDATE compliance.regulatory_reports
            SET status = $1, updated_at = NOW()
            WHERE report_id = $2
            "#,
        )
        .bind(status)
        .bind(report_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Refresh materialized views (triggered by events)
    pub async fn refresh_materialized_views(&self) -> Result<(), EventReporterError> {
        info!("Refreshing materialized views");

        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY gold.v_pru_monthly")
            .execute(&self.pool)
            .await?;

        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY gold.v_safeguarding")
            .execute(&self.pool)
            .await?;

        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY gold.v_payment_stats_q")
            .execute(&self.pool)
            .await?;

        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY gold.v_aml_kpis_y")
            .execute(&self.pool)
            .await?;

        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY gold.v_tech_risk_q")
            .execute(&self.pool)
            .await?;

        sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY gold.v_model_validation_y")
            .execute(&self.pool)
            .await?;

        info!("Materialized views refreshed successfully");

        Ok(())
    }
}
