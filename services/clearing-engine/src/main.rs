use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use prometheus::{Encoder, TextEncoder};
use serde::{Deserialize, Serialize};
use tracing::{info, error};
use tracing_subscriber;

mod nats_consumer;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClearingWindow {
    id: i64,
    status: String,
    start_time: String,
    end_time: String,
    obligations_count: i32,
    total_volume: String,
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct WindowsResponse {
    windows: Vec<ClearingWindow>,
}

#[derive(Debug, Serialize)]
struct MetricsResponse {
    total_windows: u64,
    active_windows: u64,
    netting_efficiency: f64,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let service_port = std::env::var("SERVICE_PORT")
        .unwrap_or_else(|_| "8085".to_string())
        .parse::<u16>()
        .unwrap_or(8085);

    info!("🚀 Clearing Engine starting on port {}", service_port);

    // Start NATS consumer for clearing submissions
    let nats_url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://localhost:4222".to_string());

    info!("🔄 Starting NATS consumer for multilateral netting...");
    if let Err(e) = nats_consumer::start_clearing_consumer(&nats_url).await {
        error!("Failed to start NATS consumer: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
    }
    info!("✅ NATS consumer started successfully");

    let bind_address = format!("0.0.0.0:{}", service_port);

    HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(health_check))
            .route("/metrics", web::get().to(prometheus_metrics))
            .route("/api/v1/clearing/windows", web::get().to(get_windows))
            .route("/api/v1/clearing/windows/current", web::get().to(get_current_window))
            .route("/api/v1/clearing/metrics", web::get().to(get_metrics))
    })
    .bind(&bind_address)?
    .run()
    .await
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        service: "clearing-engine".to_string(),
        version: "0.1.0".to_string(),
    })
}

async fn get_windows() -> impl Responder {
    HttpResponse::Ok().json(WindowsResponse {
        windows: vec![],
    })
}

async fn get_current_window() -> impl Responder {
    let window = ClearingWindow {
        id: Utc::now().timestamp(),
        status: "Open".to_string(),
        start_time: Utc::now().to_rfc3339(),
        end_time: (Utc::now() + chrono::Duration::hours(6)).to_rfc3339(),
        obligations_count: 0,
        total_volume: "0.00".to_string(),
    };

    HttpResponse::Ok().json(serde_json::json!({
        "window": window
    }))
}

async fn get_metrics() -> impl Responder {
    HttpResponse::Ok().json(MetricsResponse {
        total_windows: 0,
        active_windows: 0,
        netting_efficiency: 0.0,
    })
}

async fn prometheus_metrics() -> impl Responder {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];

    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => {
            match String::from_utf8(buffer) {
                Ok(mut body) => {
                    // If no metrics are registered, return a minimal valid Prometheus format
                    if body.is_empty() {
                        body = format!(
                            "# HELP clearing_engine_up Service is running\n\
                             # TYPE clearing_engine_up gauge\n\
                             clearing_engine_up 1\n"
                        );
                    }
                    HttpResponse::Ok()
                        .content_type("text/plain; version=0.0.4")
                        .body(body)
                },
                Err(e) => HttpResponse::InternalServerError()
                    .body(format!("Failed to encode metrics: {}", e))
            }
        },
        Err(e) => HttpResponse::InternalServerError()
            .body(format!("Failed to gather metrics: {}", e))
    }
}
