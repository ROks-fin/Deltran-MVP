//! Kill switch mechanism per corridor

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Kill switch status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSwitchStatus {
    /// Active
    pub active: bool,
    /// Reason
    pub reason: Option<String>,
    /// Activated at
    pub activated_at: Option<DateTime<Utc>>,
    /// Activated by (user/system)
    pub activated_by: Option<String>,
}

/// Kill switch manager
pub struct KillSwitchManager {
    /// Kill switches by corridor ID
    switches: Arc<RwLock<HashMap<String, KillSwitchStatus>>>,
}

impl KillSwitchManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            switches: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if kill switch is active
    pub async fn is_active(&self, corridor_id: &str) -> bool {
        let switches = self.switches.read().await;
        switches
            .get(corridor_id)
            .map(|s| s.active)
            .unwrap_or(false)
    }

    /// Activate kill switch
    pub async fn activate(
        &self,
        corridor_id: &str,
        reason: String,
        activated_by: String,
    ) -> Result<()> {
        error!(
            "⛔ KILL SWITCH ACTIVATED for corridor {}: {} (by {})",
            corridor_id, reason, activated_by
        );

        let mut switches = self.switches.write().await;
        switches.insert(
            corridor_id.to_string(),
            KillSwitchStatus {
                active: true,
                reason: Some(reason),
                activated_at: Some(Utc::now()),
                activated_by: Some(activated_by),
            },
        );

        Ok(())
    }

    /// Deactivate kill switch
    pub async fn deactivate(&self, corridor_id: &str, deactivated_by: String) -> Result<()> {
        info!(
            "✅ Kill switch DEACTIVATED for corridor {} (by {})",
            corridor_id, deactivated_by
        );

        let mut switches = self.switches.write().await;
        switches.insert(
            corridor_id.to_string(),
            KillSwitchStatus {
                active: false,
                reason: None,
                activated_at: None,
                activated_by: None,
            },
        );

        Ok(())
    }

    /// Check if request is allowed (throw error if kill switch active)
    pub async fn check_request_allowed(&self, corridor_id: &str) -> Result<()> {
        let switches = self.switches.read().await;
        if let Some(status) = switches.get(corridor_id) {
            if status.active {
                return Err(Error::KillSwitchActive {
                    corridor_id: corridor_id.to_string(),
                    reason: status
                        .reason
                        .clone()
                        .unwrap_or_else(|| "No reason provided".to_string()),
                });
            }
        }
        Ok(())
    }

    /// Get status
    pub async fn get_status(&self, corridor_id: &str) -> KillSwitchStatus {
        let switches = self.switches.read().await;
        switches
            .get(corridor_id)
            .cloned()
            .unwrap_or(KillSwitchStatus {
                active: false,
                reason: None,
                activated_at: None,
                activated_by: None,
            })
    }

    /// Get all active kill switches
    pub async fn get_all_active(&self) -> Vec<(String, KillSwitchStatus)> {
        let switches = self.switches.read().await;
        switches
            .iter()
            .filter(|(_, status)| status.active)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

impl Default for KillSwitchManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kill_switch() {
        let manager = KillSwitchManager::new();

        // Initially inactive
        assert!(!manager.is_active("test-corridor").await);
        assert!(manager
            .check_request_allowed("test-corridor")
            .await
            .is_ok());

        // Activate
        manager
            .activate(
                "test-corridor",
                "Test activation".to_string(),
                "admin".to_string(),
            )
            .await
            .unwrap();

        assert!(manager.is_active("test-corridor").await);
        assert!(manager
            .check_request_allowed("test-corridor")
            .await
            .is_err());

        // Deactivate
        manager
            .deactivate("test-corridor", "admin".to_string())
            .await
            .unwrap();

        assert!(!manager.is_active("test-corridor").await);
        assert!(manager
            .check_request_allowed("test-corridor")
            .await
            .is_ok());
    }
}