// NATS Consumer for Compliance Engine
// Listens to deltran.compliance.check and processes payment compliance

use async_nats::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, error, warn};
use uuid::Uuid;
use futures_util::StreamExt;

use compliance_engine::aml::AmlScorer;
use compliance_engine::sanctions::SanctionsMatcher;
use compliance_engine::pep::PepChecker;
use compliance_engine::models::{SanctionsResult, PepResult};

#[derive(Debug, Deserialize, Serialize)]
pub struct CanonicalPayment {
    pub deltran_tx_id: Uuid,
    pub uetr: Option<Uuid>,
    pub end_to_end_id: String,
    pub instruction_id: String,
    pub debtor: Party,
    pub creditor: Party,
    pub instructed_amount: rust_decimal::Decimal,
    pub currency: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Party {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ComplianceCheckResult {
    pub deltran_tx_id: Uuid,
    pub decision: ComplianceDecision,
    pub sanctions_score: f64,
    pub aml_score: f64,
    pub pep_matched: bool,
    pub risk_level: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ComplianceDecision {
    Allow,
    Reject,
}

pub async fn start_compliance_consumer(nats_url: &str) -> anyhow::Result<()> {
    info!("🔒 Starting Compliance Engine NATS consumer...");

    // Connect to NATS
    let nats_client = async_nats::connect(nats_url).await?;
    info!("✅ Connected to NATS: {}", nats_url);

    // Subscribe to compliance check topic
    let mut subscriber = nats_client.subscribe("deltran.compliance.check").await?;
    info!("📡 Subscribed to: deltran.compliance.check");

    // Spawn consumer task
    tokio::spawn(async move {
        info!("🔄 Compliance consumer task started");

        while let Some(msg) = subscriber.next().await {
            // Parse CanonicalPayment from message
            match serde_json::from_slice::<CanonicalPayment>(&msg.payload) {
                Ok(payment) => {
                    info!("🔍 Received compliance check request for: {} (E2E: {})",
                          payment.deltran_tx_id, payment.end_to_end_id);

                    // Run compliance checks
                    let result = run_compliance_checks(&payment).await;

                    match result.decision {
                        ComplianceDecision::Allow => {
                            info!("✅ ALLOW: Payment {} passed compliance (AML: {:.2}, Sanctions: {:.2})",
                                  payment.deltran_tx_id, result.aml_score, result.sanctions_score);

                            // Publish to Obligation Engine (next in chain)
                            if let Err(e) = publish_to_obligation_engine(&nats_client, &payment).await {
                                error!("Failed to route to Obligation Engine: {}", e);
                            }
                        }
                        ComplianceDecision::Reject => {
                            warn!("❌ REJECT: Payment {} failed compliance (AML: {:.2}, Sanctions: {:.2}, PEP: {})",
                                  payment.deltran_tx_id, result.aml_score, result.sanctions_score, result.pep_matched);

                            // Publish rejection
                            if let Err(e) = publish_compliance_rejection(&nats_client, &result).await {
                                error!("Failed to publish rejection: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to parse CanonicalPayment from NATS message: {}", e);
                }
            }
        }

        warn!("⚠️ Compliance consumer task ended");
    });

    info!("✅ Compliance consumer started successfully");

    Ok(())
}

async fn run_compliance_checks(payment: &CanonicalPayment) -> ComplianceCheckResult {
    // Initialize checkers
    let sanctions_matcher = SanctionsMatcher::new();
    let pep_checker = PepChecker::new();

    // Check sanctions for both parties (using empty string for country as Party doesn't have it)
    let debtor_sanctions = sanctions_matcher
        .check_sanctions(&payment.debtor.name, "")
        .unwrap_or_else(|_| SanctionsResult {
            is_sanctioned: false,
            match_details: vec![],
            lists_matched: vec![],
            confidence: 0.0,
        });

    let creditor_sanctions = sanctions_matcher
        .check_sanctions(&payment.creditor.name, "")
        .unwrap_or_else(|_| SanctionsResult {
            is_sanctioned: false,
            match_details: vec![],
            lists_matched: vec![],
            confidence: 0.0,
        });

    let sanctions_score = (debtor_sanctions.confidence + creditor_sanctions.confidence) / 2.0;

    // Check PEP (using empty string for country as Party doesn't have it)
    let debtor_pep = pep_checker
        .check_pep(&payment.debtor.name, "")
        .unwrap_or_else(|_| PepResult {
            is_pep: false,
            pep_type: None,
            position: None,
            country: None,
            risk_level: None,
        });

    let creditor_pep = pep_checker
        .check_pep(&payment.creditor.name, "")
        .unwrap_or_else(|_| PepResult {
            is_pep: false,
            pep_type: None,
            position: None,
            country: None,
            risk_level: None,
        });

    let pep_matched = debtor_pep.is_pep || creditor_pep.is_pep;

    // Simple AML score based on amount (MVP)
    let aml_score = if payment.instructed_amount > rust_decimal::Decimal::from(100000) {
        75.0
    } else if payment.instructed_amount > rust_decimal::Decimal::from(10000) {
        50.0
    } else {
        25.0
    };

    // Determine risk level
    let risk_level = if sanctions_score > 80.0 || pep_matched {
        "HIGH"
    } else if aml_score > 70.0 {
        "MEDIUM"
    } else {
        "LOW"
    };

    // Compliance decision
    let decision = if sanctions_score > 80.0 || pep_matched || aml_score > 90.0 {
        ComplianceDecision::Reject
    } else {
        ComplianceDecision::Allow
    };

    ComplianceCheckResult {
        deltran_tx_id: payment.deltran_tx_id,
        decision,
        sanctions_score,
        aml_score,
        pep_matched,
        risk_level: risk_level.to_string(),
    }
}

async fn publish_to_obligation_engine(nats_client: &Client, payment: &CanonicalPayment) -> anyhow::Result<()> {
    let subject = "deltran.obligation.create";
    let payload = serde_json::to_vec(payment)?;

    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Routed to Obligation Engine: {}", payment.deltran_tx_id);

    Ok(())
}

async fn publish_compliance_rejection(nats_client: &Client, result: &ComplianceCheckResult) -> anyhow::Result<()> {
    let subject = "deltran.compliance.reject";
    let payload = serde_json::to_vec(result)?;

    nats_client.publish(subject, payload.into()).await?;

    info!("📤 Published compliance rejection: {}", result.deltran_tx_id);

    Ok(())
}
