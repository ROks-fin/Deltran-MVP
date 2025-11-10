use crate::accounts::{NostroAccountManager, ReconciliationEngine, VostroAccountManager};
use crate::config::Config;
use crate::error::Result;
use crate::grpc::server::settlement::settlement_service_server::SettlementServiceServer;
use crate::grpc::SettlementGrpcServer;
use crate::integration::BankClientManager;
use crate::recovery::{CompensationManager, RetryManager};
use crate::settlement::{AtomicController, SettlementExecutor, SettlementValidator};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tonic::transport::Server;
use tracing::{error, info};

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
    database: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccountBalanceResponse {
    account_id: String,
    bank: String,
    currency: String,
    ledger_balance: String,
    available_balance: String,
    locked_balance: String,
    is_active: bool,
}

pub struct SettlementServer {
    config: Arc<Config>,
    db_pool: Arc<sqlx::PgPool>,
}

impl SettlementServer {
    pub async fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);

        // Initialize database connection pool
        let db_pool = PgPoolOptions::new()
            .max_connections(config.database.max_connections)
            .min_connections(config.database.min_connections)
            .connect(&config.database.url)
            .await?;

        info!("Database connection pool established");

        Ok(Self {
            config,
            db_pool: Arc::new(db_pool),
        })
    }

    pub async fn start(self) -> Result<()> {
        let config = self.config.clone();
        let db_pool = self.db_pool.clone();

        // Initialize components
        let bank_clients = Arc::new(BankClientManager::new(
            config.banks.mock_latency_ms,
            config.banks.mock_success_rate,
        ));

        let atomic_controller = Arc::new(AtomicController::new(db_pool.clone()));
        let validator = Arc::new(SettlementValidator::new(db_pool.clone()));

        let executor = Arc::new(SettlementExecutor::new(
            db_pool.clone(),
            bank_clients.clone(),
            atomic_controller.clone(),
            validator.clone(),
            config.clone(),
        ));

        let nostro_manager = Arc::new(NostroAccountManager::new(db_pool.clone()));
        let vostro_manager = Arc::new(VostroAccountManager::new(db_pool.clone()));

        let reconciliation_engine = Arc::new(ReconciliationEngine::new(
            db_pool.clone(),
            bank_clients.clone(),
            config.clone(),
        ));

        let retry_manager = Arc::new(RetryManager::new(db_pool.clone(), config.clone()));
        let compensation_manager = Arc::new(CompensationManager::new(db_pool.clone()));

        // Start background tasks
        let recon_engine = reconciliation_engine.clone();
        let recon_config = config.clone();
        tokio::spawn(async move {
            Self::run_reconciliation_scheduler(recon_engine, recon_config).await;
        });

        let retry_mgr = retry_manager.clone();
        tokio::spawn(async move {
            Self::run_retry_scheduler(retry_mgr).await;
        });

        let atomic_ctrl = atomic_controller.clone();
        tokio::spawn(async move {
            Self::run_cleanup_scheduler(atomic_ctrl).await;
        });

        // Start gRPC server
        let grpc_server = SettlementGrpcServer::new(
            executor.clone(),
            nostro_manager.clone(),
            vostro_manager.clone(),
            reconciliation_engine.clone(),
        );

        let grpc_addr = format!("0.0.0.0:{}", config.server.grpc_port).parse()?;
        let grpc_config = config.clone();

        tokio::spawn(async move {
            info!(
                "Starting gRPC server on port {}",
                grpc_config.server.grpc_port
            );

            if let Err(e) = Server::builder()
                .add_service(SettlementServiceServer::new(grpc_server))
                .serve(grpc_addr)
                .await
            {
                error!("gRPC server error: {}", e);
            }
        });

        // Start HTTP server for health checks and metrics
        let http_port = config.server.http_port;
        let http_db_pool = db_pool.clone();
        let http_nostro = nostro_manager.clone();
        let http_vostro = vostro_manager.clone();

        info!("Starting HTTP server on port {}", http_port);

        HttpServer::new(move || {
            let db_pool = http_db_pool.clone();
            let nostro = http_nostro.clone();
            let vostro = http_vostro.clone();

            App::new()
                .app_data(web::Data::new(db_pool.clone()))
                .app_data(web::Data::new(nostro.clone()))
                .app_data(web::Data::new(vostro.clone()))
                .route("/health", web::get().to(Self::health_check))
                .route("/metrics", web::get().to(Self::metrics))
                .route(
                    "/api/v1/accounts/nostro",
                    web::get().to(Self::list_nostro_accounts),
                )
                .route(
                    "/api/v1/accounts/vostro",
                    web::get().to(Self::list_vostro_accounts),
                )
        })
        .bind(format!("0.0.0.0:{}", http_port))?
        .run()
        .await?;

        Ok(())
    }

    async fn health_check(db_pool: web::Data<Arc<sqlx::PgPool>>) -> impl Responder {
        let db_healthy = sqlx::query("SELECT 1")
            .fetch_one(&***db_pool)
            .await
            .is_ok();

        let status = if db_healthy { "healthy" } else { "unhealthy" };

        HttpResponse::Ok().json(HealthResponse {
            status: status.to_string(),
            service: "settlement-engine".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            database: db_healthy,
        })
    }

    async fn metrics() -> impl Responder {
        // Placeholder for Prometheus metrics
        HttpResponse::Ok().body("# Prometheus metrics\n")
    }

    async fn list_nostro_accounts(
        nostro: web::Data<Arc<NostroAccountManager>>,
    ) -> impl Responder {
        match nostro.list_accounts(None).await {
            Ok(accounts) => {
                let response: Vec<_> = accounts
                    .into_iter()
                    .map(|a| AccountBalanceResponse {
                        account_id: a.id.to_string(),
                        bank: a.bank,
                        currency: a.currency,
                        ledger_balance: a.ledger_balance.to_string(),
                        available_balance: a.available_balance.to_string(),
                        locked_balance: a.locked_balance.to_string(),
                        is_active: a.is_active.unwrap_or(true),
                    })
                    .collect();
                HttpResponse::Ok().json(response)
            }
            Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string()
            })),
        }
    }

    async fn list_vostro_accounts(vostro: web::Data<Arc<VostroAccountManager>>) -> impl Responder {
        match vostro.list_accounts(None).await {
            Ok(accounts) => {
                let response: Vec<_> = accounts
                    .into_iter()
                    .map(|a| AccountBalanceResponse {
                        account_id: a.id.to_string(),
                        bank: a.bank,
                        currency: a.currency,
                        ledger_balance: a.ledger_balance.to_string(),
                        available_balance: a.ledger_balance.to_string(),
                        locked_balance: "0".to_string(),
                        is_active: a.is_active.unwrap_or(true),
                    })
                    .collect();
                HttpResponse::Ok().json(response)
            }
            Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
                "error": e.to_string()
            })),
        }
    }

    async fn run_reconciliation_scheduler(
        engine: Arc<ReconciliationEngine>,
        config: Arc<Config>,
    ) {
        let mut interval = interval(Duration::from_secs(
            config.reconciliation.schedule_interval_hours * 3600,
        ));

        info!(
            "Reconciliation scheduler started (every {} hours)",
            config.reconciliation.schedule_interval_hours
        );

        loop {
            interval.tick().await;

            info!("Running scheduled reconciliation");

            if let Err(e) = engine.run_scheduled_reconciliation().await {
                error!("Scheduled reconciliation failed: {}", e);
            }
        }
    }

    async fn run_retry_scheduler(retry_manager: Arc<RetryManager>) {
        let mut interval = interval(Duration::from_secs(300)); // Every 5 minutes

        info!("Retry scheduler started");

        loop {
            interval.tick().await;

            match retry_manager.get_failed_settlements(10).await {
                Ok(settlements) => {
                    if !settlements.is_empty() {
                        info!("Found {} settlements to retry", settlements.len());

                        for settlement_id in settlements {
                            if let Err(e) = retry_manager.mark_for_retry(settlement_id).await {
                                error!("Failed to mark settlement {} for retry: {}", settlement_id, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to get failed settlements: {}", e);
                }
            }
        }
    }

    async fn run_cleanup_scheduler(atomic_controller: Arc<AtomicController>) {
        let mut interval = interval(Duration::from_secs(600)); // Every 10 minutes

        info!("Cleanup scheduler started");

        loop {
            interval.tick().await;

            info!("Running atomic operations cleanup");
            atomic_controller.cleanup_completed().await;
        }
    }
}
