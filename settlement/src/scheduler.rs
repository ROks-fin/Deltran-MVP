//! Advanced settlement scheduler with cron-like schedules
//!
//! Supports configurable settlement windows:
//! - Default: 4 times per day (06:00, 12:00, 18:00, 00:00 UTC)
//! - Pilot: 2 times per day (configurable)
//! - Ad-hoc: Manual trigger for ops

use crate::{Error, Result};
use chrono::{DateTime, Duration, NaiveTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Settlement schedule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// Times of day (UTC) when settlement windows open
    /// E.g., ["06:00", "12:00", "18:00", "00:00"]
    pub window_times: Vec<String>,

    /// Window duration in minutes (default: 60)
    pub window_duration_mins: u64,

    /// Grace period after window close before settlement starts (minutes)
    pub grace_period_mins: u64,

    /// Enable automatic settlement
    pub auto_settle: bool,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            window_times: vec![
                "00:00".to_string(),
                "06:00".to_string(),
                "12:00".to_string(),
                "18:00".to_string(),
            ],
            window_duration_mins: 60,
            grace_period_mins: 5,
            auto_settle: true,
        }
    }
}

impl ScheduleConfig {
    /// Pilot configuration (2 windows per day)
    pub fn pilot() -> Self {
        Self {
            window_times: vec!["06:00".to_string(), "18:00".to_string()],
            window_duration_mins: 60,
            grace_period_mins: 5,
            auto_settle: true,
        }
    }

    /// Parse window times into NaiveTime
    fn parse_times(&self) -> Result<Vec<NaiveTime>> {
        self.window_times
            .iter()
            .map(|time_str| {
                NaiveTime::parse_from_str(time_str, "%H:%M").map_err(|e| {
                    Error::Config(format!("Invalid time format '{}': {}", time_str, e))
                })
            })
            .collect()
    }

    /// Calculate next settlement window time from now
    pub fn next_window_time(&self, now: DateTime<Utc>) -> Result<DateTime<Utc>> {
        let times = self.parse_times()?;
        let current_time = now.time();

        // Find next window time today
        for window_time in &times {
            if current_time < *window_time {
                let next = now
                    .date_naive()
                    .and_time(*window_time)
                    .and_local_timezone(Utc)
                    .single()
                    .ok_or_else(|| Error::Config("Invalid timezone conversion".to_string()))?;
                return Ok(next);
            }
        }

        // No more windows today, get first window tomorrow
        let tomorrow = (now + Duration::days(1)).date_naive();
        let first_window = times.first().ok_or_else(|| {
            Error::Config("No window times configured".to_string())
        })?;

        let next = tomorrow
            .and_time(*first_window)
            .and_local_timezone(Utc)
            .single()
            .ok_or_else(|| Error::Config("Invalid timezone conversion".to_string()))?;

        Ok(next)
    }

    /// Check if current time matches any window start time (within 1 minute tolerance)
    pub fn is_window_start_time(&self, now: DateTime<Utc>) -> Result<bool> {
        let times = self.parse_times()?;
        let current_time = now.time();

        for window_time in times {
            let diff_secs = (current_time.num_seconds_from_midnight() as i64
                - window_time.num_seconds_from_midnight() as i64)
                .abs();

            // Within 60 seconds tolerance
            if diff_secs < 60 {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Settlement event for triggering actions
#[derive(Debug, Clone, PartialEq)]
pub enum SettlementEvent {
    /// Window opened
    WindowOpened { window_id: String, start_time: DateTime<Utc> },

    /// Window closed (ready for netting)
    WindowClosed { window_id: String, end_time: DateTime<Utc> },

    /// Settlement started
    SettlementStarted { window_id: String },

    /// Settlement completed
    SettlementCompleted { window_id: String },

    /// Settlement failed
    SettlementFailed { window_id: String, reason: String },

    /// Ad-hoc settlement triggered
    AdHocTriggered { requester: String },
}

/// Advanced settlement scheduler with cron-like capabilities
pub struct AdvancedScheduler {
    config: Arc<RwLock<ScheduleConfig>>,
    current_window_id: Arc<RwLock<Option<String>>>,
    last_window_start: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl AdvancedScheduler {
    /// Create new advanced scheduler
    pub fn new(config: ScheduleConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            current_window_id: Arc::new(RwLock::new(None)),
            last_window_start: Arc::new(RwLock::new(None)),
        }
    }

    /// Start scheduler loop
    pub async fn start(self: Arc<Self>) {
        info!("Starting advanced settlement scheduler");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

        loop {
            interval.tick().await;

            if let Err(e) = self.check_schedule().await {
                warn!("Scheduler check failed: {}", e);
            }
        }
    }

    /// Check if it's time to open/close window
    async fn check_schedule(&self) -> Result<()> {
        let now = Utc::now();
        let config = self.config.read().await;

        // Check if we should open a new window
        if config.is_window_start_time(now)? {
            let last_start = self.last_window_start.read().await;

            // Avoid opening duplicate windows (check if already opened in last 2 minutes)
            if let Some(last) = *last_start {
                let diff = (now - last).num_seconds();
                if diff < 120 {
                    debug!("Window already opened recently, skipping");
                    return Ok(());
                }
            }
            drop(last_start);

            self.open_window(now).await?;
        }

        // Check if current window should close
        if let Some(window_id) = self.current_window_id.read().await.as_ref() {
            if let Some(last_start) = *self.last_window_start.read().await {
                let window_end = last_start + Duration::minutes(config.window_duration_mins as i64);

                if now >= window_end {
                    self.close_window(window_id.clone(), now).await?;

                    // Trigger settlement after grace period
                    if config.auto_settle {
                        let grace_end = window_end + Duration::minutes(config.grace_period_mins as i64);
                        if now >= grace_end {
                            self.trigger_settlement(window_id.clone()).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Open new settlement window
    async fn open_window(&self, start_time: DateTime<Utc>) -> Result<()> {
        let window_id = format!("WINDOW_{}", start_time.format("%Y%m%d_%H%M"));

        info!(
            "Opening settlement window {} at {}",
            window_id,
            start_time.to_rfc3339()
        );

        *self.current_window_id.write().await = Some(window_id.clone());
        *self.last_window_start.write().await = Some(start_time);

        self.emit_event(SettlementEvent::WindowOpened { window_id, start_time }).await;

        Ok(())
    }

    /// Close current settlement window
    async fn close_window(&self, window_id: String, end_time: DateTime<Utc>) -> Result<()> {
        info!(
            "Closing settlement window {} at {}",
            window_id,
            end_time.to_rfc3339()
        );

        self.emit_event(SettlementEvent::WindowClosed { window_id, end_time }).await;

        Ok(())
    }

    /// Trigger settlement process
    async fn trigger_settlement(&self, window_id: String) -> Result<()> {
        info!("Triggering settlement for window {}", window_id);

        self.emit_event(SettlementEvent::SettlementStarted { window_id: window_id.clone() }).await;

        // TODO: Actually trigger settlement engine here
        // For now, just mark as completed
        self.emit_event(SettlementEvent::SettlementCompleted { window_id }).await;

        // Clear current window
        *self.current_window_id.write().await = None;

        Ok(())
    }

    /// Trigger ad-hoc settlement (manual)
    pub async fn trigger_adhoc_settlement(&self, requester: String) -> Result<()> {
        info!("Ad-hoc settlement triggered by {}", requester);

        self.emit_event(SettlementEvent::AdHocTriggered { requester }).await;

        // Close current window if open
        if let Some(window_id) = self.current_window_id.read().await.as_ref() {
            let now = Utc::now();
            self.close_window(window_id.clone(), now).await?;
            self.trigger_settlement(window_id.clone()).await?;
        }

        Ok(())
    }

    /// Emit settlement event (placeholder for event bus integration)
    async fn emit_event(&self, event: SettlementEvent) {
        debug!("Settlement event: {:?}", event);
        // TODO: Publish to message bus (NATS JetStream)
    }

    /// Update schedule configuration
    pub async fn update_config(&self, new_config: ScheduleConfig) {
        *self.config.write().await = new_config;
        info!("Settlement schedule configuration updated");
    }

    /// Get next scheduled window time
    pub async fn get_next_window_time(&self) -> Result<DateTime<Utc>> {
        let config = self.config.read().await;
        config.next_window_time(Utc::now())
    }

    /// Get current window info
    pub async fn get_current_window(&self) -> Option<(String, DateTime<Utc>)> {
        let window_id = self.current_window_id.read().await.clone()?;
        let start_time = (*self.last_window_start.read().await)?;
        Some((window_id, start_time))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_config_default() {
        let config = ScheduleConfig::default();
        assert_eq!(config.window_times.len(), 4);

        let times = config.parse_times().unwrap();
        assert_eq!(times.len(), 4);
    }

    #[test]
    fn test_schedule_config_pilot() {
        let config = ScheduleConfig::pilot();
        assert_eq!(config.window_times.len(), 2);

        let times = config.parse_times().unwrap();
        assert_eq!(times[0].hour(), 6);
        assert_eq!(times[1].hour(), 18);
    }

    #[test]
    fn test_next_window_time() {
        let config = ScheduleConfig::pilot(); // 06:00 and 18:00

        // Current time: 10:00 UTC
        let now = Utc::now()
            .date_naive()
            .and_hms_opt(10, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        let next = config.next_window_time(now).unwrap();
        assert_eq!(next.hour(), 18); // Next window at 18:00
    }

    #[test]
    fn test_next_window_time_wrap_to_tomorrow() {
        let config = ScheduleConfig::pilot(); // 06:00 and 18:00

        // Current time: 20:00 UTC (after last window)
        let now = Utc::now()
            .date_naive()
            .and_hms_opt(20, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        let next = config.next_window_time(now).unwrap();
        assert_eq!(next.hour(), 6); // Next window at 06:00 tomorrow
        assert!(next > now);
    }

    #[tokio::test]
    async fn test_scheduler_lifecycle() {
        let config = ScheduleConfig::default();
        let scheduler = Arc::new(AdvancedScheduler::new(config));

        // No current window initially
        assert!(scheduler.get_current_window().await.is_none());

        // Get next window time
        let next = scheduler.get_next_window_time().await.unwrap();
        assert!(next > Utc::now());
    }

    #[tokio::test]
    async fn test_adhoc_trigger() {
        let config = ScheduleConfig::default();
        let scheduler = Arc::new(AdvancedScheduler::new(config));

        // Open a window manually
        scheduler.open_window(Utc::now()).await.unwrap();
        assert!(scheduler.get_current_window().await.is_some());

        // Trigger ad-hoc settlement
        scheduler.trigger_adhoc_settlement("ops@deltran.com".to_string()).await.unwrap();

        // Window should be cleared
        assert!(scheduler.get_current_window().await.is_none());
    }
}
