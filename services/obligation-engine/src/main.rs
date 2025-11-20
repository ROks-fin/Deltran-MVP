use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use dotenv::dotenv;
use redis::aio::ConnectionManager;
use std::sync::Arc;
use obligation_engine::{
    config::Config,
    database::Database,
    handlers,
    nats::NatsProducer,
    services::ObligationService,
    token_client::TokenEngineClient,
};
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

mod nats_consumer;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .json()
        .init();

    info!("Starting Obligation Engine...");

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");
    config.validate().expect("Invalid configuration");

    info!("Configuration loaded successfully");

    // Initialize database
    let db = Arc::new(
        Database::new(&config.database.url, config.database.max_connections)
            .await
            .expect("Failed to connect to database"),
    );

    info!("Database connected successfully");

    // Initialize Redis
    let redis_client = redis::Client::open(config.redis.url.clone())
        .expect("Failed to create Redis client");
    let redis_conn = ConnectionManager::new(redis_client)
        .await
        .expect("Failed to connect to Redis");

    info!("Redis connected successfully");

    // Initialize NATS
    let nats = Arc::new(
        NatsProducer::new(&config.nats.url, &config.nats.topic_prefix)
            .await
            .expect("Failed to create NATS producer"),
    );

    info!("NATS producer initialized successfully");

    // Initialize Token Engine client
    let token_client = Arc::new(TokenEngineClient::new(
        config.token_engine.base_url.clone(),
        config.token_engine.timeout_secs,
    ));

    info!("Token Engine client initialized");

    // Start NATS consumer for obligation creation
    let nats_url = config.nats.url.clone();
    info!("📋 Starting NATS consumer for obligation creation...");
    if let Err(e) = nats_consumer::start_obligation_consumer(&nats_url).await {
        error!("Failed to start NATS consumer: {}", e);
        return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
    }
    info!("✅ NATS consumer started successfully");

    // Initialize service
    let service = Arc::new(ObligationService::new(
        db.clone(),
        nats,
        token_client,
        redis_conn,
        config.obligation.netting_efficiency_target,
        config.obligation.liquidity_confidence_threshold,
    ));

    info!("Obligation service initialized successfully");

    // Start HTTP server
    let server_config = config.server.clone();
    let service_data = web::Data::new(service);

    info!(
        "Starting HTTP server on {}:{}",
        server_config.host, server_config.port
    );

    HttpServer::new(move || {
        App::new()
            .app_data(service_data.clone())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .configure(handlers::configure_routes)
    })
    .workers(server_config.workers)
    .bind((server_config.host, server_config.port))?
    .run()
    .await
}
