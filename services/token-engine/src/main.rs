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
        db,
        nats_producer,
        redis_conn,
    ).await);

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .wrap(middleware::NormalizePath::trim())
            .app_data(web::Data::new(token_service.clone()))
            .configure(handlers::configure_routes)
    })
    .bind(("0.0.0.0", config.server.port))?
    .run()
    .await
}
