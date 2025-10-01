//! Report Scheduler Service
//!
//! Handles scheduled report generation based on cron expressions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use thiserror::Error;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::report_generator::{ReportFormat, ReportGenerator, ReportType};

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Scheduling error: {0}")]
    SchedulingError(String),

    #[error("Report generation error: {0}")]
    ReportGenerationError(String),
}

/// Scheduled job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub job_id: String,
    pub name: String,
    pub description: Option<String>,
    pub report_type: String,
    pub schedule_type: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub format: String,
    pub auto_submit: bool,
    pub priority: i32,
    pub next_run: Option<DateTime<Utc>>,
    pub last_run: Option<DateTime<Utc>>,
    pub last_status: Option<String>,
}

/// Report Scheduler
pub struct ReportScheduler {
    pool: PgPool,
    report_generator: Arc<ReportGenerator>,
}

impl ReportScheduler {
    /// Create new scheduler
    pub fn new(pool: PgPool, report_generator: Arc<ReportGenerator>) -> Self {
        Self {
            pool,
            report_generator,
        }
    }

    /// Start scheduler loop
    pub async fn start(self: Arc<Self>) -> Result<(), SchedulerError> {
        info!("Report scheduler started");

        let mut check_interval = interval(Duration::from_secs(60)); // Check every minute

        loop {
            check_interval.tick().await;

            if let Err(e) = self.process_scheduled_jobs().await {
                error!(error = %e, "Failed to process scheduled jobs");
            }
        }
    }

    /// Process all scheduled jobs
    async fn process_scheduled_jobs(&self) -> Result<(), SchedulerError> {
        // Fetch jobs that are due
        let jobs = self.fetch_due_jobs().await?;

        if jobs.is_empty() {
            return Ok(());
        }

        info!(jobs_count = jobs.len(), "Processing scheduled jobs");

        for job in jobs {
            if let Err(e) = self.execute_job(&job).await {
                error!(
                    job_id = job.job_id,
                    error = %e,
                    "Failed to execute scheduled job"
                );

                self.update_job_status(&job.job_id, "failed", Some(&e.to_string()))
                    .await?;
            }
        }

        Ok(())
    }

    /// Fetch jobs that are due to run
    async fn fetch_due_jobs(&self) -> Result<Vec<ScheduledJob>, SchedulerError> {
        let rows = sqlx::query(
            r#"
            SELECT
                job_id,
                name,
                description,
                report_type,
                schedule_type,
                cron_expression,
                enabled,
                format,
                auto_submit,
                priority,
                next_run,
                last_run,
                last_status
            FROM compliance.scheduled_jobs
            WHERE enabled = true
              AND next_run IS NOT NULL
              AND next_run <= NOW()
            ORDER BY priority ASC, next_run ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let jobs = rows
            .into_iter()
            .map(|row| ScheduledJob {
                job_id: row.get("job_id"),
                name: row.get("name"),
                description: row.get("description"),
                report_type: row.get("report_type"),
                schedule_type: row.get("schedule_type"),
                cron_expression: row.get("cron_expression"),
                enabled: row.get("enabled"),
                format: row.get("format"),
                auto_submit: row.get("auto_submit"),
                priority: row.get("priority"),
                next_run: row.get("next_run"),
                last_run: row.get("last_run"),
                last_status: row.get("last_status"),
            })
            .collect();

        Ok(jobs)
    }

    /// Execute a scheduled job
    async fn execute_job(&self, job: &ScheduledJob) -> Result<(), SchedulerError> {
        info!(
            job_id = job.job_id,
            job_name = job.name,
            report_type = job.report_type,
            "Executing scheduled job"
        );

        let execution_id = Uuid::new_v4();
        let started_at = Utc::now();

        // Record execution start
        sqlx::query(
            r#"
            INSERT INTO compliance.job_execution_history
            (execution_id, job_id, started_at, status, triggered_by)
            VALUES ($1, $2, $3, 'running', 'scheduler')
            "#,
        )
        .bind(execution_id)
        .bind(&job.job_id)
        .bind(started_at)
        .execute(&self.pool)
        .await?;

        // Generate report based on type
        let result = match job.report_type.as_str() {
            "pru_monthly" => self.generate_pru_monthly_report(job).await,
            "safeguarding_monthly" => self.generate_safeguarding_report(job).await,
            _ => {
                warn!(
                    report_type = job.report_type,
                    "Unsupported report type for scheduled job"
                );
                Err(SchedulerError::SchedulingError(format!(
                    "Unsupported report type: {}",
                    job.report_type
                )))
            }
        };

        let completed_at = Utc::now();
        let duration_seconds = (completed_at - started_at).num_seconds() as i32;

        match result {
            Ok(report_id) => {
                // Update execution as completed
                sqlx::query(
                    r#"
                    UPDATE compliance.job_execution_history
                    SET completed_at = $1,
                        status = 'completed',
                        duration_seconds = $2,
                        report_id = $3
                    WHERE execution_id = $4
                    "#,
                )
                .bind(completed_at)
                .bind(duration_seconds)
                .bind(report_id)
                .bind(execution_id)
                .execute(&self.pool)
                .await?;

                self.update_job_status(&job.job_id, "completed", None).await?;

                // Calculate next run time
                self.schedule_next_run(&job.job_id, &job.schedule_type).await?;

                info!(
                    job_id = job.job_id,
                    report_id = %report_id,
                    duration_seconds = duration_seconds,
                    "Job executed successfully"
                );

                Ok(())
            }
            Err(e) => {
                // Update execution as failed
                sqlx::query(
                    r#"
                    UPDATE compliance.job_execution_history
                    SET completed_at = $1,
                        status = 'failed',
                        duration_seconds = $2,
                        error_message = $3
                    WHERE execution_id = $4
                    "#,
                )
                .bind(completed_at)
                .bind(duration_seconds)
                .bind(e.to_string())
                .bind(execution_id)
                .execute(&self.pool)
                .await?;

                Err(e)
            }
        }
    }

    /// Generate PRU monthly report
    async fn generate_pru_monthly_report(
        &self,
        job: &ScheduledJob,
    ) -> Result<Uuid, SchedulerError> {
        let now = Utc::now();
        let year = now.year();
        let month = if now.month() == 1 { 12 } else { now.month() - 1 };

        let format = match job.format.as_str() {
            "xlsx" => ReportFormat::Xlsx,
            "csv" => ReportFormat::Csv,
            _ => ReportFormat::Xlsx,
        };

        let report = self
            .report_generator
            .generate_pru_monthly(year, month, format, "scheduler")
            .await
            .map_err(|e| SchedulerError::ReportGenerationError(e.to_string()))?;

        Ok(report.report_id)
    }

    /// Generate safeguarding report
    async fn generate_safeguarding_report(
        &self,
        job: &ScheduledJob,
    ) -> Result<Uuid, SchedulerError> {
        // Placeholder - similar implementation as PRU
        info!(job_id = job.job_id, "Generating safeguarding report");
        Ok(Uuid::new_v4())
    }

    /// Update job status
    async fn update_job_status(
        &self,
        job_id: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<(), SchedulerError> {
        sqlx::query(
            r#"
            UPDATE compliance.scheduled_jobs
            SET last_run = NOW(),
                last_status = $1,
                updated_at = NOW()
            WHERE job_id = $2
            "#,
        )
        .bind(status)
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Schedule next run time
    async fn schedule_next_run(
        &self,
        job_id: &str,
        schedule_type: &str,
    ) -> Result<(), SchedulerError> {
        let next_run = match schedule_type {
            "daily" => Utc::now() + chrono::Duration::days(1),
            "weekly" => Utc::now() + chrono::Duration::weeks(1),
            "monthly" => {
                let now = Utc::now();
                let next_month = if now.month() == 12 {
                    chrono::NaiveDate::from_ymd_opt(now.year() + 1, 1, 1)
                } else {
                    chrono::NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1)
                };
                next_month.unwrap().and_hms_opt(0, 0, 0).unwrap().and_utc()
            }
            "quarterly" => Utc::now() + chrono::Duration::days(90),
            "annual" => Utc::now() + chrono::Duration::days(365),
            _ => Utc::now() + chrono::Duration::days(1),
        };

        sqlx::query(
            r#"
            UPDATE compliance.scheduled_jobs
            SET next_run = $1,
                updated_at = NOW()
            WHERE job_id = $2
            "#,
        )
        .bind(next_run)
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        info!(job_id = job_id, next_run = %next_run, "Next run scheduled");

        Ok(())
    }

    /// Create a new scheduled job
    pub async fn create_job(
        &self,
        name: &str,
        description: &str,
        report_type: &str,
        schedule_type: &str,
        cron_expression: &str,
        format: &str,
        auto_submit: bool,
    ) -> Result<String, SchedulerError> {
        let job_id = format!("{}_{}", report_type, Uuid::new_v4());

        sqlx::query(
            r#"
            INSERT INTO compliance.scheduled_jobs
            (job_id, name, description, report_type, schedule_type,
             cron_expression, enabled, format, auto_submit, priority)
            VALUES ($1, $2, $3, $4, $5, $6, true, $7, $8, 1)
            "#,
        )
        .bind(&job_id)
        .bind(name)
        .bind(description)
        .bind(report_type)
        .bind(schedule_type)
        .bind(cron_expression)
        .bind(format)
        .bind(auto_submit)
        .execute(&self.pool)
        .await?;

        // Schedule first run
        self.schedule_next_run(&job_id, schedule_type).await?;

        info!(job_id = job_id, "Scheduled job created");

        Ok(job_id)
    }
}
