use crate::circuit::CircuitBreaker;
use crate::errors::RiskError;
use crate::limits::LimitsManager;
use crate::models::*;
use crate::scoring::RiskScorer;
use actix_web::{web, HttpResponse};
use serde_json::json;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// ===== Health Check =====
pub async fn health_check(pool: web::Data<PgPool>) -> HttpResponse {
    // Check database connection
    let _db_status = match sqlx::query("SELECT 1").execute(pool.get_ref()).await {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    let uptime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
    })
}

// ===== Evaluate Risk =====
pub async fn evaluate_risk(
    req: web::Json<RiskEvaluationRequest>,
    scorer: web::Data<Arc<RiskScorer>>,
    circuit_breaker: web::Data<Arc<CircuitBreaker>>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, RiskError> {
    let request = req.into_inner();

    // Use circuit breaker to protect the service
    let risk_score = circuit_breaker
        .call(|| {
            let scorer = scorer.clone();
            let pool = pool.clone();
            let req = request.clone();
            async move { scorer.calculate_risk_score(&req, &pool).await }
        })
        .await?;

    // Save the score to database
    scorer.save_risk_score(&risk_score, &pool).await?;

    Ok(HttpResponse::Ok().json(RiskEvaluationResponse::from(risk_score)))
}

// ===== Get Limits for Bank =====
pub async fn get_limits(
    path: web::Path<(Uuid, String)>,
    limits_mgr: web::Data<Arc<LimitsManager>>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, RiskError> {
    let (bank_id, corridor) = path.into_inner();

    let limit = limits_mgr.get_limit(bank_id, &corridor, &pool).await?;

    Ok(HttpResponse::Ok().json(limit))
}

// ===== Update Limits for Bank =====
pub async fn update_limits(
    path: web::Path<Uuid>,
    req: web::Json<UpdateLimitRequest>,
    limits_mgr: web::Data<Arc<LimitsManager>>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, RiskError> {
    let bank_id = path.into_inner();

    limits_mgr
        .update_limit(bank_id, req.into_inner(), &pool)
        .await?;

    Ok(HttpResponse::Ok().json(json!({"status": "updated"})))
}

// ===== Get Risk Metrics =====
pub async fn get_risk_metrics(pool: web::Data<PgPool>) -> Result<HttpResponse, RiskError> {
    // Get overall statistics
    let stats = sqlx::query(
        "SELECT
            COUNT(*) as total,
            COUNT(*) FILTER (WHERE decision = 'Approve') as approved,
            COUNT(*) FILTER (WHERE decision = 'Reject') as rejected,
            COUNT(*) FILTER (WHERE decision = 'Review') as under_review,
            CAST(AVG(overall_score) AS DOUBLE PRECISION) as avg_score
         FROM risk_scores
         WHERE calculated_at > NOW() - INTERVAL '24 hours'"
    )
    .fetch_one(pool.get_ref())
    .await?;

    // Get high risk corridors
    let corridors: Vec<_> = sqlx::query(
        "SELECT
            t.sender_country || '-' || t.sent_currency || ' to ' ||
            t.receiver_country || '-' || t.received_currency as corridor,
            COUNT(*) as transaction_count,
            CAST(AVG(rs.overall_score) AS DOUBLE PRECISION) as avg_risk_score,
            CAST(COUNT(*) FILTER (WHERE rs.decision = 'Reject') * 100.0 / COUNT(*) AS DOUBLE PRECISION) as rejection_rate
         FROM risk_scores rs
         JOIN transactions t ON t.id = rs.transaction_id
         WHERE rs.calculated_at > NOW() - INTERVAL '24 hours'
         GROUP BY corridor
         HAVING AVG(rs.overall_score) > 50.0
         ORDER BY avg_risk_score DESC
         LIMIT 10"
    )
    .fetch_all(pool.get_ref())
    .await?
    .into_iter()
    .map(|row| CorridorRisk {
        corridor: row.try_get("corridor").unwrap_or_default(),
        risk_level: if row.try_get::<f64, _>("avg_risk_score").unwrap_or(0.0) > 75.0 {
            "High".to_string()
        } else {
            "Medium".to_string()
        },
        transaction_count: row.try_get::<i64, _>("transaction_count").unwrap_or(0) as u64,
        rejection_rate: row.try_get::<f64, _>("rejection_rate").unwrap_or(0.0),
    })
    .collect();

    let metrics = RiskMetrics {
        total_evaluated: stats.try_get::<i64, _>("total").unwrap_or(0) as u64,
        approved: stats.try_get::<i64, _>("approved").unwrap_or(0) as u64,
        rejected: stats.try_get::<i64, _>("rejected").unwrap_or(0) as u64,
        under_review: stats.try_get::<i64, _>("under_review").unwrap_or(0) as u64,
        average_score: stats.try_get::<f64, _>("avg_score").unwrap_or(0.0),
        high_risk_corridors: corridors,
    };

    Ok(HttpResponse::Ok().json(metrics))
}

// ===== Get Circuit Breaker Status =====
pub async fn get_circuit_status(
    circuit_breaker: web::Data<Arc<CircuitBreaker>>,
) -> HttpResponse {
    let state = circuit_breaker.get_state().await;
    HttpResponse::Ok().json(state)
}

// ===== Reset Circuit Breaker =====
pub async fn reset_circuit(circuit_breaker: web::Data<Arc<CircuitBreaker>>) -> HttpResponse {
    circuit_breaker.reset().await;
    HttpResponse::Ok().json(json!({"status": "reset"}))
}

// ===== Configure Routes =====
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/risk")
            .route("/evaluate", web::post().to(evaluate_risk))
            .route("/limits/{bank_id}/{corridor}", web::get().to(get_limits))
            .route("/limits/{bank_id}", web::put().to(update_limits))
            .route("/metrics", web::get().to(get_risk_metrics))
            .route(
                "/circuit-breaker/status",
                web::get().to(get_circuit_status),
            )
            .route("/circuit-breaker/reset", web::post().to(reset_circuit)),
    )
    .route("/health", web::get().to(health_check));
}
