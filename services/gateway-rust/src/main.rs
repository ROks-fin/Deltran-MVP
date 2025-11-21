// DelTran Gateway Service - ISO 20022 Entry/Exit Point
// Handles incoming ISO 20022 messages and routes to appropriate services via NATS

use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{post, get},
    Router, Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, postgres::PgPoolOptions};
use async_nats::Client as NatsClient;
use uuid::Uuid;
use std::sync::Arc;
use tracing::{info, error, warn, debug};
use chrono::Utc;

mod models;
mod iso20022;
mod nats_router;
mod db;
mod metrics;

use models::canonical::{CanonicalPayment, PaymentStatus};
use iso20022::pain001;
use nats_router::NatsRouter;
use metrics::METRICS;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub nats: NatsClient,
    pub router: Arc<NatsRouter>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub deltran_tx_id: Uuid,
    pub status: String,
    pub message: String,
    pub timestamp: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub db_connected: bool,
    pub nats_connected: bool,
}

// Error handling
pub enum GatewayError {
    ParseError(String),
    ValidationError(String),
    DatabaseError(sqlx::Error),
    NatsError(async_nats::Error),
    InternalError(String),
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            GatewayError::ParseError(msg) => (StatusCode::BAD_REQUEST, format!("Parse error: {}", msg)),
            GatewayError::ValidationError(msg) => (StatusCode::BAD_REQUEST, format!("Validation error: {}", msg)),
            GatewayError::DatabaseError(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)),
            GatewayError::NatsError(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("NATS error: {}", e)),
            GatewayError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Internal error: {}", msg)),
        };

        (status, Json(serde_json::json!({
            "error": message,
            "timestamp": Utc::now(),
        }))).into_response()
    }
}

impl From<sqlx::Error> for GatewayError {
    fn from(err: sqlx::Error) -> Self {
        GatewayError::DatabaseError(err)
    }
}

impl From<async_nats::Error> for GatewayError {
    fn from(err: async_nats::Error) -> Self {
        GatewayError::NatsError(err)
    }
}

// Health check endpoint
async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_connected = sqlx::query("SELECT 1").fetch_optional(&state.db).await.is_ok();
    let nats_connected = !state.nats.is_closed();

    Json(HealthResponse {
        status: if db_connected && nats_connected { "healthy" } else { "degraded" },
        service: "deltran-gateway",
        version: env!("CARGO_PKG_VERSION"),
        db_connected,
        nats_connected,
    })
}

// Prometheus metrics endpoint
async fn metrics_handler() -> Result<String, GatewayError> {
    METRICS.export()
        .map_err(|e| GatewayError::InternalError(format!("Failed to export metrics: {}", e)))
}

// pain.001 - Customer Credit Transfer Initiation
async fn handle_pain001(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<Vec<MessageResponse>>, GatewayError> {
    let start = std::time::Instant::now();
    METRICS.track_iso_message("pain.001");

    info!("Received pain.001 message");

    // Parse ISO message
    let parse_start = std::time::Instant::now();
    let document = pain001::parse_pain001(&body)
        .map_err(|e| {
            METRICS.iso_parse_errors_total.inc();
            GatewayError::ParseError(e.to_string())
        })?;
    METRICS.iso_parse_duration_seconds.observe(parse_start.elapsed().as_secs_f64());

    // Convert to canonical model
    let canonical_payments = pain001::to_canonical(&document)
        .map_err(|e| GatewayError::ParseError(e.to_string()))?;

    let mut responses = Vec::new();

    for payment in canonical_payments {
        info!("Processing payment: {} (end_to_end_id: {}, UETR: {:?})",
              payment.deltran_tx_id, payment.end_to_end_id, payment.uetr);

        METRICS.payments_total.inc();
        METRICS.payments_received.inc();

        // Persist to database
        let db_start = std::time::Instant::now();
        db::insert_payment(&state.db, &payment).await.map_err(|e| {
            METRICS.db_errors_total.inc();
            e
        })?;
        METRICS.db_operations_total.inc();
        METRICS.db_operation_duration_seconds.observe(db_start.elapsed().as_secs_f64());

        // CORRECT ORDER according to DelTran architecture:
        // Gateway → Compliance (ONLY!)
        // Compliance will route to Obligation if ALLOW
        // Obligation will route based on payment type:
        //   - INTERNATIONAL → Risk Engine → Liquidity Router → Clearing → Settlement
        //   - LOCAL → Clearing (direct) → Settlement

        info!("🔒 Routing to Compliance Engine for AML/KYC/sanctions check");
        info!("   Compliance will then route to Obligation Engine if payment is ALLOWED");
        info!("   Obligation will determine: INTERNATIONAL (Risk → Liquidity → Clearing) vs LOCAL (Clearing direct)");
        state.router.route_to_compliance_engine(&payment).await?;

        responses.push(MessageResponse {
            deltran_tx_id: payment.deltran_tx_id,
            status: "RECEIVED".to_string(),
            message: format!("Payment initiated: {} (UETR: {:?})", payment.end_to_end_id, payment.uetr),
            timestamp: Utc::now(),
        });
    }

    METRICS.payment_processing_duration_seconds.observe(start.elapsed().as_secs_f64());
    Ok(Json(responses))
}

// pacs.008 - FI to FI Customer Credit Transfer
async fn handle_pacs008(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<Vec<MessageResponse>>, GatewayError> {
    let start = std::time::Instant::now();
    METRICS.track_iso_message("pacs.008");

    info!("Received pacs.008 FI-to-FI payment message");

    // Parse ISO message
    let document = iso20022::parse_pacs008(&body)
        .map_err(|e| {
            METRICS.iso_parse_errors_total.inc();
            GatewayError::ParseError(e.to_string())
        })?;

    // Convert to canonical model
    let canonical_payments = iso20022::pacs008_to_canonical(&document)
        .map_err(|e| GatewayError::ParseError(e.to_string()))?;

    let mut responses = Vec::new();

    for payment in canonical_payments {
        info!("Processing pacs.008 payment: {} (end_to_end_id: {})",
              payment.deltran_tx_id, payment.end_to_end_id);

        // Persist to database
        db::insert_payment(&state.db, &payment).await?;

        // Route to Settlement Engine (pacs.008 is settlement instruction)
        state.router.route_to_settlement_engine(&payment).await?;

        responses.push(MessageResponse {
            deltran_tx_id: payment.deltran_tx_id,
            status: "RECEIVED".to_string(),
            message: format!("Settlement instruction received: {}", payment.end_to_end_id),
            timestamp: Utc::now(),
        });
    }

    Ok(Json(responses))
}

// camt.054 - Bank to Customer Debit/Credit Notification (FUNDING!)
async fn handle_camt054(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<Vec<MessageResponse>>, GatewayError> {
    info!("🚨 Received camt.054 FUNDING notification - CRITICAL");

    // Parse ISO message
    let document = iso20022::parse_camt054(&body)
        .map_err(|e| GatewayError::ParseError(e.to_string()))?;

    // Extract funding events
    let funding_events = iso20022::extract_funding_events(&document)
        .map_err(|e| GatewayError::ParseError(e.to_string()))?;

    let mut responses = Vec::new();

    for event in funding_events {
        // Only process CREDIT events (money IN) that are BOOKED
        if !iso20022::is_credit_event(&event) {
            info!("Skipping DEBIT event (money out): {:?}", event.entry_reference);
            continue;
        }

        if !iso20022::is_booked(&event) {
            warn!("Skipping PENDING funding event: {:?}", event.entry_reference);
            continue;
        }

        info!("💰 FUNDING CONFIRMED: {} {} on account {}",
              event.amount, event.currency, event.account);

        // Try to match to existing payment by end_to_end_id or instruction_id
        if let Some(end_to_end_id) = &event.end_to_end_id {
            info!("💰 Matching funding to payment with end_to_end_id: {}", end_to_end_id);

            // Update payment status to Funded in database
            db::update_payment_status_by_e2e(&state.db, end_to_end_id, PaymentStatus::Funded).await?;

            // Retrieve the funded payment to route to Token Engine
            if let Some(payment) = db::get_payment_by_e2e(&state.db, end_to_end_id).await? {
                info!("🪙 CRITICAL: Routing to Token Engine for minting (1:1 backing guarantee)");
                info!("   Amount: {} {}", event.amount, event.currency);
                info!("   UETR: {:?}", payment.uetr);

                // Route to Token Engine for minting
                // Tokens can ONLY be minted AFTER funding is confirmed via camt.054
                // This enforces DelTran's 1:1 backing guarantee
                state.router.route_to_token_engine(&payment).await?;

                responses.push(MessageResponse {
                    deltran_tx_id: payment.deltran_tx_id,
                    status: "FUNDED".to_string(),
                    message: format!("Funding confirmed: {} {} | Token minting triggered", event.amount, event.currency),
                    timestamp: Utc::now(),
                });
            } else {
                warn!("⚠️ Payment not found for end_to_end_id: {} - cannot mint tokens", end_to_end_id);
            }
        } else {
            warn!("camt.054 entry has no end_to_end_id - cannot match to payment");
        }
    }

    Ok(Json(responses))
}

// pacs.002 - FI to FI Payment Status Report
async fn handle_pacs002(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<MessageResponse>, GatewayError> {
    info!("📊 Received pacs.002 FI-to-FI Payment Status Report");

    // Parse ISO message
    let document = iso20022::parse_pacs002(&body)
        .map_err(|e| GatewayError::ParseError(e.to_string()))?;

    // Convert to payment status reports
    let status_reports = iso20022::to_payment_status_reports(&document)
        .map_err(|e| GatewayError::ParseError(e.to_string()))?;

    info!("Parsed {} payment status reports", status_reports.len());

    // Process each status report
    for report in status_reports {
        info!("Processing status for original_message_id: {}, status: {:?}",
              report.original_message_id, report.status);

        // Update payment status in database if we have an end_to_end_id
        if let Some(end_to_end_id) = &report.end_to_end_id {
            // Map ISO status to DelTran PaymentStatus
            let payment_status = match report.status {
                iso20022::PaymentStatus::Accepted => PaymentStatus::Accepted,
                iso20022::PaymentStatus::Pending => PaymentStatus::Pending,
                iso20022::PaymentStatus::Rejected => PaymentStatus::Rejected,
                iso20022::PaymentStatus::Unknown => PaymentStatus::Pending, // Conservative default
            };

            db::update_payment_status_by_e2e(&state.db, end_to_end_id, payment_status).await?;

            // Retrieve the updated payment to route to Notification Engine
            if let Some(payment) = db::get_payment_by_e2e(&state.db, end_to_end_id).await? {
                info!("🔔 Routing status update to Notification Engine");
                state.router.route_to_notification_engine(&payment, "status_update").await?;
            }
        }
    }

    Ok(Json(MessageResponse {
        deltran_tx_id: Uuid::new_v4(), // Using new UUID for status report itself
        status: "PROCESSED".to_string(),
        message: format!("Processed {} payment status reports", status_reports.len()),
        timestamp: Utc::now(),
    }))
}

// pain.002 - Customer Payment Status Report
async fn handle_pain002(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<MessageResponse>, GatewayError> {
    info!("📊 Received pain.002 Customer Payment Status Report");

    // Parse ISO message
    let document = iso20022::parse_pain002(&body)
        .map_err(|e| GatewayError::ParseError(e.to_string()))?;

    // Convert to customer payment status reports
    let status_reports = iso20022::to_customer_payment_status(&document)
        .map_err(|e| GatewayError::ParseError(e.to_string()))?;

    info!("Parsed {} customer payment status reports", status_reports.len());

    // Process each status report
    for report in status_reports {
        info!("Processing customer status for original_message_id: {}, status: {:?}",
              report.original_message_id, report.status);

        // Update payment status in database if we have an end_to_end_id
        if let Some(end_to_end_id) = &report.end_to_end_id {
            // Map ISO status to DelTran PaymentStatus
            let payment_status = match report.status {
                iso20022::PaymentStatus::Accepted => PaymentStatus::Accepted,
                iso20022::PaymentStatus::Pending => PaymentStatus::Pending,
                iso20022::PaymentStatus::Rejected => PaymentStatus::Rejected,
                iso20022::PaymentStatus::Unknown => PaymentStatus::Pending,
            };

            db::update_payment_status_by_e2e(&state.db, end_to_end_id, payment_status).await?;

            // Retrieve the updated payment to route to Notification Engine
            if let Some(payment) = db::get_payment_by_e2e(&state.db, end_to_end_id).await? {
                info!("🔔 Routing customer status update to Notification Engine");
                state.router.route_to_notification_engine(&payment, "customer_status").await?;
            }
        }
    }

    Ok(Json(MessageResponse {
        deltran_tx_id: Uuid::new_v4(),
        status: "PROCESSED".to_string(),
        message: format!("Processed {} customer payment status reports", status_reports.len()),
        timestamp: Utc::now(),
    }))
}

// camt.053 - Bank to Customer Statement (EOD reconciliation)
async fn handle_camt053(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<MessageResponse>, GatewayError> {
    info!("📊 Received camt.053 Bank Statement for EOD reconciliation");

    // Parse ISO message
    let document = iso20022::parse_camt053(&body)
        .map_err(|e| GatewayError::ParseError(e.to_string()))?;

    // Convert to statement summaries
    let statements = iso20022::to_statement_summaries(&document)
        .map_err(|e| GatewayError::ParseError(e.to_string()))?;

    info!("Parsed {} bank statements", statements.len());

    let mut total_entries = 0;
    let mut total_reconciled = 0;

    // Process each statement for reconciliation
    for statement in &statements {
        info!("💼 Processing statement {} for account {}",
              statement.statement_id, statement.account_id);
        info!("   Opening Balance: {:?}", statement.opening_balance);
        info!("   Closing Balance: {:?}", statement.closing_balance);
        info!("   Total Credits: {}", statement.total_credits);
        info!("   Total Debits: {}", statement.total_debits);
        info!("   Entries: {}", statement.entries.len());

        total_entries += statement.entries.len();

        // Process each entry for reconciliation
        for entry in &statement.entries {
            // Try to match to existing payment by end_to_end_id
            if let Some(end_to_end_id) = &entry.end_to_end_id {
                // Check if we have this payment in our system
                if let Some(payment) = db::get_payment_by_e2e(&state.db, end_to_end_id).await? {
                    info!("✅ Reconciled payment: {} ({})",
                          end_to_end_id, entry.entry_reference);

                    // Update reconciliation status
                    // Note: In production, you'd have a reconciliation table
                    // For now, we just log successful reconciliation
                    total_reconciled += 1;

                    // Optionally route to Reporting Engine for analytics
                    state.router.route_to_reporting_engine(&payment).await?;
                } else {
                    warn!("⚠️ Unmatched statement entry: {} (not in our system)", end_to_end_id);
                }
            } else {
                debug!("Entry {} has no end_to_end_id - cannot reconcile", entry.entry_reference);
            }
        }
    }

    info!("🎯 Reconciliation complete: {}/{} entries matched", total_reconciled, total_entries);

    Ok(Json(MessageResponse {
        deltran_tx_id: Uuid::new_v4(),
        status: "RECONCILED".to_string(),
        message: format!("Processed {} statements with {}/{} entries reconciled",
                        statements.len(), total_reconciled, total_entries),
        timestamp: Utc::now(),
    }))
}

// Get payment status by DelTran TX ID
async fn get_payment_status(
    State(state): State<AppState>,
    Path(tx_id): Path<Uuid>,
) -> Result<Json<CanonicalPayment>, GatewayError> {
    info!("Retrieving payment status for: {}", tx_id);

    let payment = db::get_payment_by_id(&state.db, tx_id).await?;

    match payment {
        Some(p) => Ok(Json(p)),
        None => Err(GatewayError::ValidationError(format!("Payment not found: {}", tx_id))),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_line_number(true)
        .init();

    info!("🚀 Starting DelTran Gateway Service");

    // Load configuration from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://deltran:deltran@localhost:5432/deltran_gateway".to_string());
    let nats_url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://localhost:4222".to_string());
    let bind_addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string());

    // Connect to PostgreSQL
    info!("Connecting to database: {}", database_url);
    let db = PgPoolOptions::new()
        .max_connections(50)
        .connect(&database_url)
        .await?;

    // Run migrations
    info!("Running database migrations");
    sqlx::migrate!("./migrations").run(&db).await?;

    // Connect to NATS
    info!("Connecting to NATS: {}", nats_url);
    let nats = async_nats::connect(&nats_url).await?;

    // Initialize NATS router
    let router = Arc::new(NatsRouter::new(nats.clone()));

    // Create app state
    let state = AppState {
        db,
        nats,
        router,
    };

    // Build router with CORS and metrics
    use tower_http::cors::{CorsLayer, Any};

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        .route("/iso20022/pain.001", post(handle_pain001))
        .route("/iso20022/pacs.008", post(handle_pacs008))
        .route("/iso20022/camt.054", post(handle_camt054))
        .route("/iso20022/pacs.002", post(handle_pacs002))
        .route("/iso20022/pain.002", post(handle_pain002))
        .route("/iso20022/camt.053", post(handle_camt053))
        .route("/payment/:tx_id", get(get_payment_status))
        .layer(cors)
        .with_state(state);

    // Start server
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    info!("✅ Gateway listening on: {}", bind_addr);
    info!("📨 Ready to receive ISO 20022 messages");
    info!("   POST /iso20022/pain.001 - Customer Credit Transfer Initiation");
    info!("   POST /iso20022/pacs.008 - FI to FI Customer Credit Transfer");
    info!("   POST /iso20022/camt.054 - Funding Notification (CRITICAL)");
    info!("   POST /iso20022/pacs.002 - Payment Status Report");
    info!("   POST /iso20022/pain.002 - Customer Payment Status Report");
    info!("   POST /iso20022/camt.053 - Bank Statement (EOD)");
    info!("   GET  /payment/:tx_id - Get payment status");
    info!("   GET  /health - Health check");
    info!("   GET  /metrics - Prometheus metrics");

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_endpoint() {
        // TODO: Add integration tests
    }

    #[tokio::test]
    async fn test_pain001_parsing() {
        // TODO: Add pain.001 parsing tests
    }
}
