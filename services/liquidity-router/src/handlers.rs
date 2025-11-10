use crate::models::LiquidityPredictionRequest;
use crate::optimizer::ConversionOptimizer;
use crate::predictor::LiquidityPredictor;
use actix_web::{web, HttpResponse};
use serde_json::json;
use std::sync::{Arc, Mutex};

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "status": "healthy",
        "service": "liquidity-router",
        "version": "1.0.0"
    }))
}

pub async fn predict_liquidity(
    predictor: web::Data<Arc<Mutex<LiquidityPredictor>>>,
    request: web::Json<LiquidityPredictionRequest>,
) -> HttpResponse {
    let predictor = predictor.lock().unwrap();
    let prediction = predictor.predict_instant_settlement(&request.corridor, request.amount);

    HttpResponse::Ok().json(prediction)
}

pub async fn optimize_conversion(
    optimizer: web::Data<Arc<ConversionOptimizer>>,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (from, to) = path.into_inner();

    match optimizer.find_optimal_path(&from, &to) {
        Some(path) => HttpResponse::Ok().json(path),
        None => HttpResponse::NotFound().json(json!({
            "error": "No conversion path found"
        })),
    }
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg
        // Root-level health endpoint for monitoring
        .route("/health", web::get().to(health_check))
        .route("/metrics", web::get().to(health_check)) // Placeholder for Prometheus metrics
        .service(
            web::scope("/api/v1/liquidity")
                .route("/health", web::get().to(health_check))
                .route("/predict", web::post().to(predict_liquidity))
                .route("/optimize/{from}/{to}", web::get().to(optimize_conversion)),
        );
}