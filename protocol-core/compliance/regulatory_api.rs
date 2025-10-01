// compliance/regulatory_api.rs
// Read-only API for regulators (FSRA, UAE FIU)
// Provides secure, audited access to compliance data

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

// ============== Request/Response Types ==============

#[derive(Debug, Deserialize)]
pub struct GetPaymentRequest {
    pub payment_id: Uuid,
    pub regulator_id: String,
}

#[derive(Debug, Serialize)]
pub struct GetPaymentResponse {
    pub payment_id: Uuid,
    pub amount: String,
    pub currency: String,
    pub created_at: chrono::NaiveDateTime,
    pub status: String,
    pub debtor: PartyDetails,
    pub creditor: PartyDetails,
    pub screening_id: Option<Uuid>,
    pub screening_result: Option<String>,
    pub travel_rule_compliant: Option<bool>,
    pub risk_score: Option<String>,
    pub risk_level: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PartyDetails {
    pub name: String,
    pub account: String,
    pub address: Option<String>,
    pub country: String,
    pub identification: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetScreeningRequest {
    pub screening_id: Uuid,
    pub regulator_id: String,
}

#[derive(Debug, Serialize)]
pub struct GetScreeningResponse {
    pub screening_id: Uuid,
    pub payment_id: Uuid,
    pub result: String,
    pub hits: Vec<ScreeningHitResponse>,
    pub screened_at: chrono::NaiveDateTime,
    pub screened_by: String,
    pub actions_taken: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ScreeningHitResponse {
    pub hit_id: Uuid,
    pub list_source: String,
    pub list_name: String,
    pub entity_name: String,
    pub match_score: String,
    pub match_reason: String,
    pub reviewed: bool,
    pub review_decision: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetSTRRequest {
    pub str_id: Uuid,
    pub regulator_id: String,
}

#[derive(Debug, Serialize)]
pub struct GetSTRResponse {
    pub str_id: Uuid,
    pub payment_ids: Vec<Uuid>,
    pub narrative: String,
    pub indicators: Vec<String>,
    pub filed_at: Option<chrono::NaiveDateTime>,
    pub reporter_name: String,
    pub status: String,
    pub amount_involved: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchPaymentsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub currency: Option<String>,
    pub country: Option<String>,
    pub min_amount: Option<String>,
    pub max_amount: Option<String>,
    pub screening_result: Option<String>,
    pub page: Option<i32>,
    pub page_size: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SearchPaymentsResponse {
    pub payments: Vec<GetPaymentResponse>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Deserialize)]
pub struct GetInstitutionStatsQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetInstitutionStatsResponse {
    pub total_payments: i64,
    pub total_volume: String,
    pub screenings_performed: i64,
    pub hits_identified: i64,
    pub payments_blocked: i64,
    pub strs_filed: i64,
    pub payments_by_currency: serde_json::Value,
    pub payments_by_country: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct GetAuditTrailQuery {
    pub entity_id: Uuid,
    pub entity_type: String,
}

#[derive(Debug, Serialize)]
pub struct GetAuditTrailResponse {
    pub events: Vec<AuditEventResponse>,
}

#[derive(Debug, Serialize)]
pub struct AuditEventResponse {
    pub event_id: Uuid,
    pub timestamp: chrono::NaiveDateTime,
    pub action: String,
    pub actor: String,
    pub changes: serde_json::Value,
    pub ip_address: Option<String>,
}

// ============== Regulatory API Service ==============

pub struct RegulatoryApiService {
    pool: PgPool,
}

impl RegulatoryApiService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create router with all regulatory endpoints
    pub fn router(self) -> Router {
        let state = Arc::new(self);

        Router::new()
            .route("/regulatory/payments/:payment_id", get(get_payment))
            .route("/regulatory/screenings/:screening_id", get(get_screening))
            .route("/regulatory/strs/:str_id", get(get_str))
            .route("/regulatory/payments/search", get(search_payments))
            .route("/regulatory/stats", get(get_institution_stats))
            .route("/regulatory/audit-trail", get(get_audit_trail))
            .with_state(state)
    }

    /// Get payment details
    async fn get_payment_internal(
        &self,
        payment_id: Uuid,
        regulator_id: &str,
    ) -> Result<GetPaymentResponse, RegulatoryApiError> {
        // Log access
        self.audit_access(regulator_id, "payment", payment_id, "read")
            .await?;

        // This is a simplified query - in production, join with actual payments table
        let row = sqlx::query(
            r#"
            SELECT
                s.payment_id,
                '1000.00' as amount, -- Placeholder
                'USD' as currency,
                s.created_at,
                'COMPLETED' as status,
                s.entity_name as debtor_name,
                '' as debtor_account,
                s.entity_country as debtor_country,
                '' as creditor_name,
                '' as creditor_account,
                '' as creditor_country,
                s.screening_id,
                s.screening_result,
                NULL::TEXT as risk_score,
                NULL::TEXT as risk_level
            FROM compliance_screenings s
            WHERE s.payment_id = $1
            LIMIT 1
            "#,
        )
        .bind(payment_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RegulatoryApiError::NotFound(format!("Payment not found: {}", e)))?;

        // Check Travel Rule compliance
        let travel_rule_compliant = self
            .check_travel_rule_compliance(payment_id)
            .await
            .ok();

        Ok(GetPaymentResponse {
            payment_id: row.get("payment_id"),
            amount: row.get("amount"),
            currency: row.get("currency"),
            created_at: row.get("created_at"),
            status: row.get("status"),
            debtor: PartyDetails {
                name: row.get("debtor_name"),
                account: row.get("debtor_account"),
                address: None,
                country: row.get("debtor_country"),
                identification: None,
            },
            creditor: PartyDetails {
                name: row.get("creditor_name"),
                account: row.get("creditor_account"),
                address: None,
                country: row.get("creditor_country"),
                identification: None,
            },
            screening_id: row.get("screening_id"),
            screening_result: row.get("screening_result"),
            travel_rule_compliant,
            risk_score: row.get("risk_score"),
            risk_level: row.get("risk_level"),
        })
    }

    /// Get screening details
    async fn get_screening_internal(
        &self,
        screening_id: Uuid,
        regulator_id: &str,
    ) -> Result<GetScreeningResponse, RegulatoryApiError> {
        // Log access
        self.audit_access(regulator_id, "screening", screening_id, "read")
            .await?;

        let row = sqlx::query(
            r#"
            SELECT
                screening_id,
                payment_id,
                screening_result,
                screened_at
            FROM compliance_screenings
            WHERE screening_id = $1
            "#,
        )
        .bind(screening_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RegulatoryApiError::NotFound(format!("Screening not found: {}", e)))?;

        // Get hits
        let hits = self.get_screening_hits(screening_id).await?;

        Ok(GetScreeningResponse {
            screening_id: row.get("screening_id"),
            payment_id: row.get("payment_id"),
            result: row.get("screening_result"),
            hits,
            screened_at: row.get("screened_at"),
            screened_by: "system".to_string(), // Placeholder
            actions_taken: vec![],
        })
    }

    /// Get screening hits
    async fn get_screening_hits(
        &self,
        screening_id: Uuid,
    ) -> Result<Vec<ScreeningHitResponse>, RegulatoryApiError> {
        let rows = sqlx::query(
            r#"
            SELECT
                hit_id,
                list_source,
                list_name,
                entity_name,
                match_score,
                match_reason,
                reviewed,
                review_decision
            FROM compliance_screening_hits
            WHERE screening_id = $1
            ORDER BY match_score DESC
            "#,
        )
        .bind(screening_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegulatoryApiError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| ScreeningHitResponse {
                hit_id: row.get("hit_id"),
                list_source: row.get("list_source"),
                list_name: row.get("list_name"),
                entity_name: row.get("entity_name"),
                match_score: row.get::<rust_decimal::Decimal, _>("match_score").to_string(),
                match_reason: row.get("match_reason"),
                reviewed: row.get("reviewed"),
                review_decision: row.get("review_decision"),
            })
            .collect())
    }

    /// Get STR details
    async fn get_str_internal(
        &self,
        str_id: Uuid,
        regulator_id: &str,
    ) -> Result<GetSTRResponse, RegulatoryApiError> {
        // Log access
        self.audit_access(regulator_id, "str", str_id, "read")
            .await?;

        let row = sqlx::query(
            r#"
            SELECT
                str_id,
                payment_ids,
                narrative,
                indicators,
                filing_date,
                reporter_name,
                status,
                amount_involved
            FROM compliance_strs
            WHERE str_id = $1
            "#,
        )
        .bind(str_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RegulatoryApiError::NotFound(format!("STR not found: {}", e)))?;

        Ok(GetSTRResponse {
            str_id: row.get("str_id"),
            payment_ids: row.get("payment_ids"),
            narrative: row.get("narrative"),
            indicators: row.get("indicators"),
            filed_at: row.get("filing_date"),
            reporter_name: row.get("reporter_name"),
            status: row.get("status"),
            amount_involved: row
                .get::<Option<rust_decimal::Decimal>, _>("amount_involved")
                .map(|d| d.to_string()),
        })
    }

    /// Search payments
    async fn search_payments_internal(
        &self,
        query: SearchPaymentsQuery,
        regulator_id: &str,
    ) -> Result<SearchPaymentsResponse, RegulatoryApiError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(50).min(1000);
        let offset = (page - 1) * page_size;

        // Log access
        info!(
            "Regulator {} searching payments: page={}, page_size={}",
            regulator_id, page, page_size
        );

        // Simplified query - in production, join with actual payments table
        let count_row = sqlx::query(
            r#"
            SELECT COUNT(DISTINCT payment_id) as total
            FROM compliance_screenings
            WHERE 1=1
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RegulatoryApiError::DatabaseError(e.to_string()))?;

        let total_count: i64 = count_row.get("total");

        // For now, return empty payments list
        // In production, implement full payment search with filters

        Ok(SearchPaymentsResponse {
            payments: vec![],
            total_count,
            page,
            page_size,
        })
    }

    /// Get institution statistics
    async fn get_institution_stats_internal(
        &self,
        query: GetInstitutionStatsQuery,
        regulator_id: &str,
    ) -> Result<GetInstitutionStatsResponse, RegulatoryApiError> {
        // Log access
        info!("Regulator {} requesting institution stats", regulator_id);

        let row = sqlx::query(
            r#"
            SELECT
                COUNT(DISTINCT payment_id) as total_payments,
                COUNT(DISTINCT screening_id) as screenings_performed,
                COUNT(DISTINCT CASE WHEN screening_result = 'HIT' THEN screening_id END) as hits_identified,
                COUNT(DISTINCT CASE WHEN screening_result = 'BLOCKED' THEN screening_id END) as payments_blocked
            FROM compliance_screenings
            WHERE 1=1
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| RegulatoryApiError::DatabaseError(e.to_string()))?;

        // Get STRs count
        let strs_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM compliance_strs
            WHERE status = 'SUBMITTED'
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(GetInstitutionStatsResponse {
            total_payments: row.get("total_payments"),
            total_volume: "0.00".to_string(), // Placeholder
            screenings_performed: row.get("screenings_performed"),
            hits_identified: row.get("hits_identified"),
            payments_blocked: row.get("payments_blocked"),
            strs_filed: strs_count,
            payments_by_currency: serde_json::json!({}),
            payments_by_country: serde_json::json!({}),
        })
    }

    /// Get audit trail
    async fn get_audit_trail_internal(
        &self,
        query: GetAuditTrailQuery,
        regulator_id: &str,
    ) -> Result<GetAuditTrailResponse, RegulatoryApiError> {
        // Log access
        self.audit_access(regulator_id, &query.entity_type, query.entity_id, "audit")
            .await?;

        let rows = sqlx::query(
            r#"
            SELECT
                event_id,
                timestamp,
                action,
                actor,
                changes,
                ip_address
            FROM compliance_audit_trail
            WHERE entity_type = $1 AND entity_id = $2
            ORDER BY timestamp DESC
            LIMIT 100
            "#,
        )
        .bind(&query.entity_type)
        .bind(query.entity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RegulatoryApiError::DatabaseError(e.to_string()))?;

        let events = rows
            .into_iter()
            .map(|row| AuditEventResponse {
                event_id: row.get("event_id"),
                timestamp: row.get("timestamp"),
                action: row.get("action"),
                actor: row.get("actor"),
                changes: row.get("changes"),
                ip_address: row.get::<Option<std::net::IpAddr>, _>("ip_address").map(|ip| ip.to_string()),
            })
            .collect();

        Ok(GetAuditTrailResponse { events })
    }

    /// Check Travel Rule compliance
    async fn check_travel_rule_compliance(
        &self,
        payment_id: Uuid,
    ) -> Result<bool, RegulatoryApiError> {
        let compliant: Option<bool> = sqlx::query_scalar(
            "SELECT compliant FROM compliance_travel_rule_checks WHERE payment_id = $1 LIMIT 1",
        )
        .bind(payment_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RegulatoryApiError::DatabaseError(e.to_string()))?;

        Ok(compliant.unwrap_or(false))
    }

    /// Audit regulator access
    async fn audit_access(
        &self,
        regulator_id: &str,
        entity_type: &str,
        entity_id: Uuid,
        action: &str,
    ) -> Result<(), RegulatoryApiError> {
        sqlx::query(
            r#"
            INSERT INTO compliance_audit_trail
            (entity_type, entity_id, action, actor, timestamp)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
        )
        .bind(entity_type)
        .bind(entity_id)
        .bind(format!("regulatory_{}", action))
        .bind(format!("regulator:{}", regulator_id))
        .execute(&self.pool)
        .await
        .map_err(|e| RegulatoryApiError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

// ============== HTTP Handlers ==============

async fn get_payment(
    State(service): State<Arc<RegulatoryApiService>>,
    Path(payment_id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<GetPaymentResponse>, RegulatoryApiError> {
    let regulator_id = params
        .get("regulator_id")
        .ok_or_else(|| RegulatoryApiError::Unauthorized("Missing regulator_id".to_string()))?;

    let response = service
        .get_payment_internal(payment_id, regulator_id)
        .await?;
    Ok(Json(response))
}

async fn get_screening(
    State(service): State<Arc<RegulatoryApiService>>,
    Path(screening_id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<GetScreeningResponse>, RegulatoryApiError> {
    let regulator_id = params
        .get("regulator_id")
        .ok_or_else(|| RegulatoryApiError::Unauthorized("Missing regulator_id".to_string()))?;

    let response = service
        .get_screening_internal(screening_id, regulator_id)
        .await?;
    Ok(Json(response))
}

async fn get_str(
    State(service): State<Arc<RegulatoryApiService>>,
    Path(str_id): Path<Uuid>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<GetSTRResponse>, RegulatoryApiError> {
    let regulator_id = params
        .get("regulator_id")
        .ok_or_else(|| RegulatoryApiError::Unauthorized("Missing regulator_id".to_string()))?;

    let response = service.get_str_internal(str_id, regulator_id).await?;
    Ok(Json(response))
}

async fn search_payments(
    State(service): State<Arc<RegulatoryApiService>>,
    Query(mut query): Query<SearchPaymentsQuery>,
) -> Result<Json<SearchPaymentsResponse>, RegulatoryApiError> {
    // Extract regulator_id from query params
    let regulator_id = "regulator_default".to_string(); // Placeholder

    let response = service
        .search_payments_internal(query, &regulator_id)
        .await?;
    Ok(Json(response))
}

async fn get_institution_stats(
    State(service): State<Arc<RegulatoryApiService>>,
    Query(query): Query<GetInstitutionStatsQuery>,
) -> Result<Json<GetInstitutionStatsResponse>, RegulatoryApiError> {
    let regulator_id = "regulator_default".to_string(); // Placeholder

    let response = service
        .get_institution_stats_internal(query, &regulator_id)
        .await?;
    Ok(Json(response))
}

async fn get_audit_trail(
    State(service): State<Arc<RegulatoryApiService>>,
    Query(query): Query<GetAuditTrailQuery>,
) -> Result<Json<GetAuditTrailResponse>, RegulatoryApiError> {
    let regulator_id = "regulator_default".to_string(); // Placeholder

    let response = service
        .get_audit_trail_internal(query, &regulator_id)
        .await?;
    Ok(Json(response))
}

// ============== Error Handling ==============

#[derive(Debug)]
pub enum RegulatoryApiError {
    NotFound(String),
    Unauthorized(String),
    DatabaseError(String),
    InternalError(String),
}

impl IntoResponse for RegulatoryApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            RegulatoryApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            RegulatoryApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            RegulatoryApiError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            RegulatoryApiError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl std::fmt::Display for RegulatoryApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegulatoryApiError::NotFound(e) => write!(f, "Not found: {}", e),
            RegulatoryApiError::Unauthorized(e) => write!(f, "Unauthorized: {}", e),
            RegulatoryApiError::DatabaseError(e) => write!(f, "Database error: {}", e),
            RegulatoryApiError::InternalError(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl std::error::Error for RegulatoryApiError {}
