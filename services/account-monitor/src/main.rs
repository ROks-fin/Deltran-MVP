// Account Monitor - Real-time FIAT funding detection
// Monitors EMI accounts for incoming funds via camt.054 or Bank API

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{info, error};
use tracing_subscriber;

mod config;
mod monitor;
mod bank_client;
mod camt_parser;
mod models;

use config::Config;
use monitor::AccountMonitor;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("🏦 Account Monitor starting...");

    // Load configuration
    let config = Config::from_env();

    // Create account monitor
    let monitor = AccountMonitor::new(config.clone()).await.expect("Failed to create monitor");

    info!("✅ Account Monitor initialized");

    // Start scheduled monitoring jobs
    start_monitoring_jobs(monitor.clone()).await?;

    // Start HTTP server for health checks
    let server_port = config.server_port;
    info!("🚀 Starting HTTP server on port {}", server_port);

    HttpServer::new(|| {
        App::new()
            .route("/health", web::get().to(health_check))
            .route("/metrics", web::get().to(metrics))
    })
    .bind(("0.0.0.0", server_port))?
    .run()
    .await
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "account-monitor",
        "version": "0.1.0"
    }))
}

async fn metrics() -> impl Responder {
    HttpResponse::Ok().body("# TODO: Prometheus metrics")
}

async fn start_monitoring_jobs(monitor: AccountMonitor) -> std::io::Result<()> {
    let scheduler = JobScheduler::new().await.map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    // Job 1: Poll bank accounts every 30 seconds
    let monitor_clone = monitor.clone();
    let job_poll = Job::new_async("*/30 * * * * *", move |_uuid, _lock| {
        let monitor = monitor_clone.clone();
        Box::pin(async move {
            info!("🔄 Polling bank accounts for new transactions...");
            if let Err(e) = monitor.poll_all_accounts().await {
                error!("Failed to poll accounts: {}", e);
            }
        })
    }).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    scheduler.add(job_poll).await.map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    // Job 2: Listen for incoming camt.054 messages
    let monitor_clone2 = monitor.clone();
    tokio::spawn(async move {
        info!("📥 Starting camt.054 listener...");
        if let Err(e) = monitor_clone2.listen_camt054().await {
            error!("camt.054 listener failed: {}", e);
        }
    });

    // Start scheduler
    scheduler.start().await.map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    info!("✅ Monitoring jobs started");

    Ok(())
}
