use actix_cors::Cors;
use actix_web::{middleware::Logger, web, App, HttpServer};
use compliance_engine::{
    aml::AmlScorer,
    config::Config,
    database,
    handlers,
    pep::PepChecker,
    sanctions::SanctionsMatcher,
};
use std::sync::Arc;
use tracing::{error, info};

mod nats_consumer;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("compliance_engine=debug,actix_web=info")
        .init();

    info!("Starting Compliance Engine...");

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");

    info!("Configuration loaded successfully");
    info!("Server will listen on {}:{}", config.server.host, config.server.port);

    // Create database pool
    let pool = match database::create_pool(&config.database.url).await {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to create database pool: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
    };

    info!("Database pool created");

    // Health check
    database::health_check(&pool).await?;
    info!("Database health check passed");

    // Initialize compliance components
    let sanctions_matcher = Arc::new(SanctionsMatcher::new());
    let aml_scorer = Arc::new(AmlScorer::new());
    let pep_checker = Arc::new(PepChecker::new());

    info!("Compliance components initialized");

    // Start NATS consumer for compliance checks
    let nats_url = std::env::var("NATS_URL")
        .unwrap_or_else(|_| "nats://localhost:4222".to_string());

    info!("🔒 Starting NATS consumer for compliance checks...");
    if let Err(e) = nats_consumer::start_compliance_consumer(&nats_url).await {
        error!("Failed to start NATS consumer: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
    }
    info!("✅ NATS consumer started successfully");

    // Start HTTP server
    let bind_address = format!("{}:{}", config.server.host, config.server.port);
    info!("Starting HTTP server on {}", bind_address);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(Logger::default())
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(sanctions_matcher.clone()))
            .app_data(web::Data::new(aml_scorer.clone()))
            .app_data(web::Data::new(pep_checker.clone()))
            // Routes
            .route("/health", web::get().to(handlers::health_check))
            .route("/api/v1/compliance/check", web::post().to(handlers::check_compliance))
    })
    .bind(&bind_address)?
    .run()
    .await
}
