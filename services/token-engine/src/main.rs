use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use dotenv::dotenv;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use token_engine::{
    config::Config,
    database::Database,
    handlers,
    nats::NatsProducer,
    nats_consumer::NatsConsumer,
    reconciliation::ReconciliationService,
    reconciliation_handlers,
    services::TokenService,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .init();

    let config = Config::from_env().expect("Failed to load configuration");
    config.validate().expect("Invalid configuration");

    info!("Starting Token Engine on port {}", config.server.port);

    let db = Arc::new(
        Database::new(&config.database.url, config.database.max_connections)
            .await
            .expect("Failed to connect to database")
    );

    let redis_client = redis::Client::open(config.redis.url.clone())
        .expect("Failed to create Redis client");
    let redis_conn = ConnectionManager::new(redis_client)
        .await
        .expect("Failed to connect to Redis");

    let nats_producer = Arc::new(
        NatsProducer::new(&config.nats.url, &config.nats.topic_prefix)
            .await
            .expect("Failed to connect to NATS")
    );

    let token_service = Arc::new(TokenService::new(
        db.clone(),
        nats_producer,
        redis_conn,
    ).await);

    // ========== RECONCILIATION SERVICES ==========

    info!("Initializing 3-tier reconciliation system...");

    // Initialize Reconciliation Service
    let reconciliation_service = Arc::new(ReconciliationService::new(db.pool().clone()));

    // Tier 1: Near Real-Time (CAMT.054 via NATS)
    let nats_consumer = Arc::new(
        NatsConsumer::new(
            &config.nats.url,
            "iso20022-notifications".to_string(),
            "token-engine-reconciliation".to_string(),
            reconciliation_service.clone(),
        )
        .await
        .expect("Failed to create NATS consumer")
    );

    let consumer_handle = nats_consumer.clone();
    tokio::spawn(async move {
        consumer_handle.run_forever().await;
    });

    info!("✓ Tier 1 - Near Real-Time: CAMT.054 consumer active");

    // Tier 2: Intradey Reconciliation (15-60 min interval)
    let intradey_service = reconciliation_service.clone();
    tokio::spawn(async move {
        // 30-minute interval for pilot (can be tuned per corridor)
        intradey_service.start_intradey_loop(30).await;
    });

    info!("✓ Tier 2 - Intradey: 30-minute reconciliation loop started");
    info!("✓ Tier 3 - EOD: CAMT.053 processing ready (triggered by events)");

    info!("========================================");
    info!("Token Engine with 1:1 Backing Guarantee");
    info!("All 3 reconciliation tiers operational");
    info!("========================================");

    // ========== HTTP SERVER ==========

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .app_data(web::Data::new(token_service.clone()))
            .app_data(web::Data::new(reconciliation_service.clone()))
            .configure(handlers::configure_routes)
            .configure(reconciliation_handlers::configure_reconciliation_routes)
    })
    .bind(("0.0.0.0", config.server.port))?
    .run()
    .await
}
