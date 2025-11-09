use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpServer};
use dotenv::dotenv;
use liquidity_router::{config::Config, handlers, optimizer::ConversionOptimizer, predictor::LiquidityPredictor};
use std::sync::{Arc, Mutex};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let _subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .json()
        .init();

    info!("Starting Liquidity Router...");

    let config = Config::from_env().expect("Failed to load configuration");

    info!("Configuration loaded successfully");

    // Initialize predictor and optimizer
    let predictor = Arc::new(Mutex::new(LiquidityPredictor::new()));
    let optimizer = Arc::new(ConversionOptimizer::new());

    info!("Liquidity predictor and optimizer initialized");

    let server_config = config.server.clone();

    info!(
        "Starting HTTP server on {}:{}",
        server_config.host, server_config.port
    );

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(predictor.clone()))
            .app_data(web::Data::new(optimizer.clone()))
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
