// Scheduler Module - Manages cron-based window scheduling

use super::{WindowConfig, WindowManager};
use crate::errors::ClearingError;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};
use uuid::Uuid;

/// Window Scheduler handles automatic window creation and closing
pub struct WindowScheduler {
    scheduler: JobScheduler,
    window_manager: Arc<WindowManager>,
    config: WindowConfig,
}

impl WindowScheduler {
    /// Create new scheduler
    pub async fn new(
        window_manager: Arc<WindowManager>,
        config: WindowConfig,
    ) -> Result<Self, ClearingError> {
        let scheduler = JobScheduler::new()
            .await
            .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;

        Ok(Self {
            scheduler,
            window_manager,
            config,
        })
    }

    /// Start the scheduler and register jobs
    pub async fn start(&mut self) -> Result<(), ClearingError> {
        info!("Starting clearing window scheduler");

        // Job 1: Open new windows (00:00, 06:00, 12:00, 18:00 UTC)
        let window_manager = self.window_manager.clone();
        let open_job = Job::new_async(self.config.schedule.as_str(), move |_uuid, _lock| {
            let wm = window_manager.clone();
            Box::pin(async move {
                info!("Scheduled window opening triggered");
                match wm.create_window().await {
                    Ok(window) => {
                        info!("Opened new clearing window: {} (ID: {})", window.window_name, window.id);
                    }
                    Err(e) => {
                        error!("Failed to open window: {:?}", e);
                    }
                }
            })
        })
        .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;

        self.scheduler
            .add(open_job)
            .await
            .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;

        // Job 2: Close windows at cutoff time (every 5 minutes check)
        let window_manager = self.window_manager.clone();
        let close_job = Job::new_async("0 */5 * * * *", move |_uuid, _lock| {
            let wm = window_manager.clone();
            Box::pin(async move {
                if let Some(current) = wm.get_current_window().await {
                    let now = chrono::Utc::now();
                    if now >= current.cutoff_time && current.status == "Open" {
                        info!("Cutoff time reached for window {}, initiating close", current.id);
                        match wm.close_window(current.id).await {
                            Ok(_) => {
                                info!("Window {} closed successfully", current.id);
                            }
                            Err(e) => {
                                error!("Failed to close window {}: {:?}", current.id, e);
                            }
                        }
                    }
                }
            })
        })
        .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;

        self.scheduler
            .add(close_job)
            .await
            .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;

        // Job 3: Grace period expiry check (every minute)
        let window_manager = self.window_manager.clone();
        let grace_job = Job::new_async("0 * * * * *", move |_uuid, _lock| {
            let wm = window_manager.clone();
            Box::pin(async move {
                if let Some(current) = wm.get_current_window().await {
                    if current.status == "Closing" && wm.is_grace_period_expired(&current) {
                        info!("Grace period expired for window {}, moving to Processing", current.id);
                        match wm.update_status(current.id, crate::models::WindowStatus::Processing).await {
                            Ok(_) => {
                                info!("Window {} moved to Processing state", current.id);
                                // Trigger clearing process here
                            }
                            Err(e) => {
                                error!("Failed to update window {} status: {:?}", current.id, e);
                            }
                        }
                    }
                }
            })
        })
        .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;

        self.scheduler
            .add(grace_job)
            .await
            .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;

        // Start the scheduler
        self.scheduler
            .start()
            .await
            .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;

        info!("Window scheduler started successfully");
        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&mut self) -> Result<(), ClearingError> {
        self.scheduler
            .shutdown()
            .await
            .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;
        info!("Window scheduler stopped");
        Ok(())
    }

    /// Add custom job to scheduler
    pub async fn add_custom_job(&self, cron: &str, job_fn: Box<dyn Fn() + Send + Sync>) -> Result<Uuid, ClearingError> {
        let job = Job::new(cron, move |_uuid, _lock| {
            job_fn();
        })
        .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;

        let uuid = job.guid();

        self.scheduler
            .add(job)
            .await
            .map_err(|e| ClearingError::SchedulerError(e.to_string()))?;

        Ok(uuid)
    }

    /// Get next execution time for a job
    pub async fn next_tick(&mut self, job_id: Uuid) -> Result<Option<chrono::DateTime<chrono::Utc>>, ClearingError> {
        self.scheduler
            .next_tick_for_job(job_id)
            .await
            .map_err(|e| ClearingError::SchedulerError(e.to_string()))
    }
}

/// Helper function to create human-readable schedule
pub fn create_schedule(hours: Vec<u8>) -> String {
    let hours_str = hours
        .iter()
        .map(|h| h.to_string())
        .collect::<Vec<_>>()
        .join(",");

    format!("0 {} * * *", hours_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_schedule() {
        let schedule = create_schedule(vec![0, 6, 12, 18]);
        assert_eq!(schedule, "0 0,6,12,18 * * *");
    }

    #[test]
    fn test_custom_schedule() {
        let schedule = create_schedule(vec![9, 17]);
        assert_eq!(schedule, "0 9,17 * * *");
    }
}
