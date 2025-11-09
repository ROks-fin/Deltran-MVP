use async_nats::Client;
use crate::models::ObligationEvent;
use tracing::info;

pub struct NatsProducer {
    client: Client,
    topic_prefix: String,
}

impl NatsProducer {
    pub async fn new(url: &str, topic_prefix: &str) -> Result<Self, String> {
        let client = async_nats::connect(url)
            .await
            .map_err(|e| e.to_string())?;

        info!("Connected to NATS at {}", url);
        Ok(Self {
            client,
            topic_prefix: topic_prefix.to_string(),
        })
    }

    pub async fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), String> {
        self.client
            .publish(subject.to_string(), payload.to_vec().into())
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn publish_obligation_event(&self, event: &ObligationEvent) -> Result<(), String> {
        let subject = format!("{}.obligation.events", self.topic_prefix);
        let payload = serde_json::to_vec(event)
            .map_err(|e| format!("Serialization error: {}", e))?;
        self.publish(&subject, &payload).await
    }

    pub async fn publish_netting_result(&self, clearing_window: i64, result: &serde_json::Value) -> Result<(), String> {
        let subject = format!("{}.netting.results.{}", self.topic_prefix, clearing_window);
        let payload = serde_json::to_vec(result)
            .map_err(|e| format!("Serialization error: {}", e))?;
        self.publish(&subject, &payload).await
    }
}
