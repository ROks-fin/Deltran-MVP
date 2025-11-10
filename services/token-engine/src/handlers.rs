use crate::errors::TokenEngineError;
use crate::models::{
    BurnTokenRequest, ConvertTokenRequest, MintTokenRequest,
    TransferTokenRequest,
};
use crate::services::TokenService;
use crate::metrics;
use actix_web::{web, HttpResponse};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Health check endpoint
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "status": "healthy",
        "service": "token-engine",
        "version": "1.0.0"
    }))
}

/// Mint tokens endpoint
pub async fn mint_tokens(
    service: web::Data<Arc<TokenService>>,
    request: web::Json<MintTokenRequest>,
) -> Result<HttpResponse, TokenEngineError> {
    let response = service.mint_tokens(request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

/// Burn tokens endpoint
pub async fn burn_tokens(
    service: web::Data<Arc<TokenService>>,
    request: web::Json<BurnTokenRequest>,
) -> Result<HttpResponse, TokenEngineError> {
    let response = service.burn_tokens(request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

/// Transfer tokens endpoint
pub async fn transfer_tokens(
    service: web::Data<Arc<TokenService>>,
    request: web::Json<TransferTokenRequest>,
) -> Result<HttpResponse, TokenEngineError> {
    let response = service.transfer_tokens(request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

/// Convert tokens endpoint
pub async fn convert_tokens(
    service: web::Data<Arc<TokenService>>,
    request: web::Json<ConvertTokenRequest>,
) -> Result<HttpResponse, TokenEngineError> {
    let response = service.convert_tokens(request.into_inner()).await?;
    Ok(HttpResponse::Ok().json(response))
}

/// Get balance endpoint
pub async fn get_balance(
    service: web::Data<Arc<TokenService>>,
    bank_id: web::Path<Uuid>,
    query: web::Query<BalanceQuery>,
) -> Result<HttpResponse, TokenEngineError> {
    let balances = service
        .get_balance(*bank_id, query.currency.as_deref())
        .await?;

    Ok(HttpResponse::Ok().json(json!({
        "bank_id": bank_id.to_string(),
        "balances": balances
    })))
}

/// Get all balances for a bank
pub async fn get_all_balances(
    service: web::Data<Arc<TokenService>>,
    bank_id: web::Path<Uuid>,
) -> Result<HttpResponse, TokenEngineError> {
    let balances = service.get_balance(*bank_id, None).await?;

    Ok(HttpResponse::Ok().json(json!({
        "bank_id": bank_id.to_string(),
        "balances": balances,
        "total_currencies": balances.len()
    })))
}

#[derive(serde::Deserialize)]
pub struct BalanceQuery {
    currency: Option<String>,
}

/// Prometheus metrics endpoint
pub async fn metrics_endpoint() -> HttpResponse {
    match metrics::metrics_handler() {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4")
            .body(body),
        Err(e) => HttpResponse::InternalServerError()
            .json(json!({
                "error": "Failed to gather metrics",
                "details": e.to_string()
            }))
    }
}

/// Configure routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/tokens")
            .route("/health", web::get().to(health_check))
            .route("/mint", web::post().to(mint_tokens))
            .route("/burn", web::post().to(burn_tokens))
            .route("/transfer", web::post().to(transfer_tokens))
            .route("/convert", web::post().to(convert_tokens))
            .route("/balance/{bank_id}", web::get().to(get_balance))
            .route("/balances/{bank_id}", web::get().to(get_all_balances)),
    )
    .route("/metrics", web::get().to(metrics_endpoint))
    .route("/health", web::get().to(health_check));
}