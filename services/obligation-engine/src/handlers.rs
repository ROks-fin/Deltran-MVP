use crate::errors::ObligationEngineError;
use crate::models::{CreateInstantObligationRequest, SettleObligationsRequest};
use crate::services::ObligationService;
use actix_web::{web, HttpResponse};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Health check endpoint
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "status": "healthy",
        "service": "obligation-engine",
        "version": "1.0.0"
    }))
}

/// Create instant obligation
pub async fn create_instant_obligation(
    service: web::Data<Arc<ObligationService>>,
    request: web::Json<CreateInstantObligationRequest>,
) -> Result<HttpResponse, ObligationEngineError> {
    let response = service.create_instant_obligation(request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

/// Get obligation by ID
pub async fn get_obligation(
    service: web::Data<Arc<ObligationService>>,
    obligation_id: web::Path<Uuid>,
) -> Result<HttpResponse, ObligationEngineError> {
    let obligation = service.get_obligation(*obligation_id).await?;
    Ok(HttpResponse::Ok().json(obligation))
}

/// Get obligations for a clearing window
pub async fn get_obligations_by_window(
    service: web::Data<Arc<ObligationService>>,
    clearing_window: web::Path<i64>,
) -> Result<HttpResponse, ObligationEngineError> {
    let obligations = service.get_obligations_by_window(*clearing_window).await?;

    Ok(HttpResponse::Ok().json(json!({
        "clearing_window": *clearing_window,
        "total_obligations": obligations.len(),
        "obligations": obligations
    })))
}

/// Calculate netting for a clearing window
pub async fn calculate_netting(
    service: web::Data<Arc<ObligationService>>,
    clearing_window: web::Path<i64>,
) -> Result<HttpResponse, ObligationEngineError> {
    let result = service.calculate_netting(*clearing_window).await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Settle obligations for a clearing window
pub async fn settle_obligations(
    service: web::Data<Arc<ObligationService>>,
    request: web::Json<SettleObligationsRequest>,
) -> Result<HttpResponse, ObligationEngineError> {
    let result = service.settle_obligations(request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Get current clearing window info
pub async fn get_current_window(
    service: web::Data<Arc<ObligationService>>,
) -> Result<HttpResponse, ObligationEngineError> {
    // Get current window from database
    let current_window = service.db.get_current_clearing_window();

    let window_info = service.db.get_clearing_window_info(current_window).await?;

    Ok(HttpResponse::Ok().json(json!({
        "current_window": current_window,
        "window_info": window_info
    })))
}

/// Configure routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/obligations")
            .route("/health", web::get().to(health_check))
            .route("/create", web::post().to(create_instant_obligation))
            .route("/{id}", web::get().to(get_obligation))
            .route("/window/{clearing_window}", web::get().to(get_obligations_by_window))
            .route("/netting/{clearing_window}", web::post().to(calculate_netting))
            .route("/settle", web::post().to(settle_obligations))
            .route("/current-window", web::get().to(get_current_window)),
    );
}