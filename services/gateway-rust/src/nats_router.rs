// NATS Router - Routes canonical payments to appropriate services
// This is the heart of DelTran's event-driven architecture

use async_nats::Client as NatsClient;
use serde_json;
use tracing::{info, error};
use anyhow::Result;

use crate::models::canonical::CanonicalPayment;

pub struct NatsRouter {
    client: NatsClient,
}

impl NatsRouter {
    pub fn new(client: NatsClient) -> Self {
        Self { client }
    }

    /// Route to Compliance Engine (AML/KYC/sanctions check) - FIRST IN CHAIN!
    pub async fn route_to_compliance_engine(&self, payment: &CanonicalPayment) -> Result<()> {
        let subject = "deltran.compliance.check";
        let payload = serde_json::to_vec(&payment)?;

        info!("🔒 Routing to Compliance Engine (AML/KYC/Sanctions): {} -> {}", payment.deltran_tx_id, subject);

        self.client.publish(subject, payload.into()).await?;

        Ok(())
    }

    /// Route to Obligation Engine (creates/matches obligations)
    pub async fn route_to_obligation_engine(&self, payment: &CanonicalPayment) -> Result<()> {
        let subject = "deltran.obligation.create";
        let payload = serde_json::to_vec(&payment)?;

        info!("Routing to Obligation Engine: {} -> {}", payment.deltran_tx_id, subject);

        self.client.publish(subject, payload.into()).await?;

        Ok(())
    }

    /// Route to Risk Engine (AML/sanctions/limits check)
    pub async fn route_to_risk_engine(&self, payment: &CanonicalPayment) -> Result<()> {
        let subject = "deltran.risk.check";
        let payload = serde_json::to_vec(&payment)?;

        info!("Routing to Risk Engine: {} -> {}", payment.deltran_tx_id, subject);

        self.client.publish(subject, payload.into()).await?;

        Ok(())
    }

    /// Route to Token Engine (mint tokens upon funding)
    pub async fn route_to_token_engine(&self, payment: &CanonicalPayment) -> Result<()> {
        let subject = "deltran.token.mint";
        let payload = serde_json::to_vec(&payment)?;

        info!("Routing to Token Engine: {} -> {}", payment.deltran_tx_id, subject);

        self.client.publish(subject, payload.into()).await?;

        Ok(())
    }

    /// Route to Clearing Engine (multilateral netting)
    pub async fn route_to_clearing_engine(&self, payment: &CanonicalPayment) -> Result<()> {
        let subject = "deltran.clearing.submit";
        let payload = serde_json::to_vec(&payment)?;

        info!("Routing to Clearing Engine: {} -> {}", payment.deltran_tx_id, subject);

        self.client.publish(subject, payload.into()).await?;

        Ok(())
    }

    /// Route to Settlement Engine (execute settlement)
    pub async fn route_to_settlement_engine(&self, payment: &CanonicalPayment) -> Result<()> {
        let subject = "deltran.settlement.execute";
        let payload = serde_json::to_vec(&payment)?;

        info!("Routing to Settlement Engine: {} -> {}", payment.deltran_tx_id, subject);

        self.client.publish(subject, payload.into()).await?;

        Ok(())
    }

    /// Route to Notification Engine (send updates to banks)
    pub async fn route_to_notification_engine(&self, payment: &CanonicalPayment, notification_type: &str) -> Result<()> {
        let subject = format!("deltran.notification.{}", notification_type);
        let payload = serde_json::to_vec(&payment)?;

        info!("Routing to Notification Engine: {} -> {}", payment.deltran_tx_id, subject);

        self.client.publish(subject, payload.into()).await?;

        Ok(())
    }

    /// Route to Reporting Engine (metrics/analytics)
    pub async fn route_to_reporting_engine(&self, payment: &CanonicalPayment) -> Result<()> {
        let subject = "deltran.reporting.payment";
        let payload = serde_json::to_vec(&payment)?;

        info!("Routing to Reporting Engine: {} -> {}", payment.deltran_tx_id, subject);

        self.client.publish(subject, payload.into()).await?;

        Ok(())
    }

    /// Publish event for analytics/monitoring
    pub async fn publish_event(&self, event_type: &str, data: serde_json::Value) -> Result<()> {
        let subject = format!("deltran.events.{}", event_type);
        let payload = serde_json::to_vec(&data)?;

        self.client.publish(subject, payload.into()).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nats_routing() {
        // TODO: Add NATS routing tests
    }
}
