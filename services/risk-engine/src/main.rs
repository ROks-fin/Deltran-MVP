use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use dotenv::dotenv;
use risk_engine::{
    circuit::CircuitBreaker, config::Config, database, handlers, limits::LimitsManager,
    scoring::RiskScorer,
};
use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .json()
        .init();

    info!("Starting Risk Engine...");

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");
    info!("Configuration loaded successfully");

    // Create database pool
    info!("Connecting to database at {}", config.database.url);
    let pool = match database::create_pool(&config.database.url).await {
        Ok(p) => {
            info!("Database connection pool created successfully");
            p
        }
        Err(e) => {
            error!("Failed to create database pool: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Database connection failed: {}", e),
            ));
        }
    };

    // Health check database
    if let Err(e) = database::health_check(&pool).await {
        error!("Database health check failed: {}", e);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Database not accessible",
        ));
    }
    info!("Database health check passed");

    // Initialize components
    let scorer = Arc::new(RiskScorer::new());
    let limits_mgr = Arc::new(LimitsManager::new());
    let circuit_breaker = Arc::new(CircuitBreaker::with_config(
        "risk_engine".to_string(),
        config.risk.failure_threshold,
        config.risk.recovery_threshold,
        config.risk.circuit_timeout_seconds,
    ));

    info!("Risk components initialized successfully");

    let server_config = config.server.clone();

    info!(
        "Starting HTTP server on {}:{}",
        server_config.host, server_config.port
    );

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(scorer.clone()))
            .app_data(web::Data::new(limits_mgr.clone()))
            .app_data(web::Data::new(circuit_breaker.clone()))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .wrap(middleware::Logger::default())
            .configure(handlers::configure_routes)
    })
    .workers(server_config.workers)
    .bind((server_config.host, server_config.port))?
    .run()
    .await
}
