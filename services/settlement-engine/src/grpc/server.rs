use crate::accounts::{NostroAccountManager, ReconciliationEngine, VostroAccountManager};
use crate::integration::PaymentRail;
use crate::settlement::SettlementExecutor;
use crate::settlement::executor::{SettlementPriority, SettlementRequest as InternalSettlementRequest};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};
use tracing::{error, info};
use uuid::Uuid;

// Include generated protobuf code
pub mod settlement {
    tonic::include_proto!("settlement");
}

use settlement::settlement_service_server::SettlementService;
use settlement::*;

pub struct SettlementGrpcServer {
    executor: Arc<SettlementExecutor>,
    nostro_manager: Arc<NostroAccountManager>,
    vostro_manager: Arc<VostroAccountManager>,
    reconciliation_engine: Arc<ReconciliationEngine>,
    event_tx: broadcast::Sender<SettlementEvent>,
}

impl SettlementGrpcServer {
    pub fn new(
        executor: Arc<SettlementExecutor>,
        nostro_manager: Arc<NostroAccountManager>,
        vostro_manager: Arc<VostroAccountManager>,
        reconciliation_engine: Arc<ReconciliationEngine>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            executor,
            nostro_manager,
            vostro_manager,
            reconciliation_engine,
            event_tx,
        }
    }
}

#[tonic::async_trait]
impl SettlementService for SettlementGrpcServer {
    async fn execute_settlement(
        &self,
        request: Request<SettlementRequest>,
    ) -> Result<Response<SettlementResponse>, Status> {
        let req = request.into_inner();

        info!(
            "Received settlement request for obligation {}",
            req.obligation_id
        );

        // Parse request
        let obligation_id = Uuid::parse_str(&req.obligation_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid obligation_id: {}", e)))?;

        let amount = rust_decimal::Decimal::from_str(&req.amount)
            .map_err(|e| Status::invalid_argument(format!("Invalid amount: {}", e)))?;

        let priority = match req.priority {
            0 => SettlementPriority::Normal,
            1 => SettlementPriority::High,
            2 => SettlementPriority::Urgent,
            _ => SettlementPriority::Normal,
        };

        let method = match req.method {
            0 => PaymentRail::SWIFT,
            1 => PaymentRail::SEPA,
            2 => PaymentRail::LocalACH,
            3 => PaymentRail::Mock,
            _ => PaymentRail::Mock,
        };

        // Create settlement request
        let settlement_req = InternalSettlementRequest {
            id: None,
            obligation_id,
            from_bank: req.from_bank,
            to_bank: req.to_bank,
            amount,
            currency: req.currency,
            settlement_date: chrono::Utc::now()
                + chrono::Duration::seconds(req.settlement_date),
            priority,
            method,
            metadata: serde_json::from_str(&serde_json::to_string(&req.metadata).unwrap())
                .unwrap_or(serde_json::json!({})),
        };

        // Execute settlement
        match self.executor.execute_settlement(settlement_req).await {
            Ok(result) => {
                // Broadcast event
                let _ = self.event_tx.send(SettlementEvent {
                    event_id: Uuid::new_v4().to_string(),
                    settlement_id: result.settlement_id.to_string(),
                    event_type: EventType::SettlementCompleted as i32,
                    timestamp: chrono::Utc::now().timestamp(),
                    message: "Settlement completed successfully".to_string(),
                    data: Default::default(),
                });

                Ok(Response::new(SettlementResponse {
                    settlement_id: result.settlement_id.to_string(),
                    status: settlement::SettlementStatus::Completed as i32,
                    reference: result.external_reference.unwrap_or_default(),
                    completed_at: result.completed_at.map(|t| t.timestamp()).unwrap_or(0),
                    confirmation_code: result.bank_confirmation.unwrap_or_default(),
                    message: "Settlement executed successfully".to_string(),
                }))
            }
            Err(e) => {
                error!("Settlement execution failed: {}", e);

                // Broadcast failure event
                let _ = self.event_tx.send(SettlementEvent {
                    event_id: Uuid::new_v4().to_string(),
                    settlement_id: Uuid::new_v4().to_string(),
                    event_type: EventType::SettlementFailed as i32,
                    timestamp: chrono::Utc::now().timestamp(),
                    message: e.to_string(),
                    data: Default::default(),
                });

                Err(Status::internal(format!("Settlement failed: {}", e)))
            }
        }
    }

    async fn get_settlement_status(
        &self,
        request: Request<SettlementStatusRequest>,
    ) -> Result<Response<SettlementStatusResponse>, Status> {
        let req = request.into_inner();

        let settlement_id = Uuid::parse_str(&req.settlement_id)
            .map_err(|e| Status::invalid_argument(format!("Invalid settlement_id: {}", e)))?;

        match self.executor.get_settlement_status(settlement_id).await {
            Ok(result) => Ok(Response::new(SettlementStatusResponse {
                settlement_id: result.settlement_id.to_string(),
                status: settlement::SettlementStatus::Completed as i32,
                from_bank: String::new(),
                to_bank: String::new(),
                amount: String::new(),
                currency: String::new(),
                created_at: 0,
                completed_at: result.completed_at.map(|t| t.timestamp()).unwrap_or(0),
                error_message: result.error_message.unwrap_or_default(),
                checkpoints: vec![],
            })),
            Err(e) => Err(Status::not_found(format!("Settlement not found: {}", e))),
        }
    }

    async fn reconcile_accounts(
        &self,
        _request: Request<ReconcileRequest>,
    ) -> Result<Response<ReconcileResponse>, Status> {
        info!("Received reconciliation request");

        match self.reconciliation_engine.reconcile_all_accounts().await {
            Ok(report) => {
                let discrepancies: Vec<AccountDiscrepancy> = report
                    .discrepancies
                    .iter()
                    .map(|d| AccountDiscrepancy {
                        account_id: d.account_id.to_string(),
                        account_type: match d.account_type {
                            crate::accounts::reconciliation::AccountType::Nostro => {
                                "nostro".to_string()
                            }
                            crate::accounts::reconciliation::AccountType::Vostro => {
                                "vostro".to_string()
                            }
                        },
                        bank: d.bank.clone(),
                        currency: d.currency.clone(),
                        internal_balance: d.internal_balance.to_string(),
                        external_balance: d.external_balance.to_string(),
                        discrepancy: d.discrepancy.to_string(),
                        status: match d.status {
                            crate::accounts::reconciliation::ReconciliationStatus::Balanced => {
                                ReconciliationStatus::Balanced as i32
                            }
                            crate::accounts::reconciliation::ReconciliationStatus::Unresolved => {
                                ReconciliationStatus::Unresolved as i32
                            }
                            crate::accounts::reconciliation::ReconciliationStatus::Identified => {
                                ReconciliationStatus::Identified as i32
                            }
                            crate::accounts::reconciliation::ReconciliationStatus::InvestigationRequired => {
                                ReconciliationStatus::InvestigationRequired as i32
                            }
                        },
                    })
                    .collect();

                Ok(Response::new(ReconcileResponse {
                    report_id: report.id.to_string(),
                    timestamp: report.report_date.timestamp(),
                    total_accounts: report.total_accounts,
                    balanced_accounts: report.balanced_accounts,
                    discrepancy_accounts: report.discrepancy_accounts.as_i64().unwrap_or(0) as i32,
                    total_discrepancy: report.total_discrepancy.to_string(),
                    discrepancies,
                }))
            }
            Err(e) => {
                error!("Reconciliation failed: {}", e);
                Err(Status::internal(format!("Reconciliation failed: {}", e)))
            }
        }
    }

    type StreamSettlementEventsStream =
        std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<SettlementEvent, Status>> + Send>>;

    async fn stream_settlement_events(
        &self,
        _request: Request<StreamRequest>,
    ) -> Result<Response<Self::StreamSettlementEventsStream>, Status> {
        let rx = self.event_tx.subscribe();
        let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
            .map(|result| result.map_err(|_| Status::internal("Stream error")));

        Ok(Response::new(Box::pin(stream)))
    }

    async fn get_nostro_balance(
        &self,
        request: Request<AccountBalanceRequest>,
    ) -> Result<Response<AccountBalanceResponse>, Status> {
        let req = request.into_inner();

        match self
            .nostro_manager
            .get_account_by_bank_currency(&req.bank, &req.currency)
            .await
        {
            Ok(account) => Ok(Response::new(AccountBalanceResponse {
                account_id: account.id.to_string(),
                bank: account.bank,
                currency: account.currency,
                ledger_balance: account.ledger_balance.to_string(),
                available_balance: account.available_balance.to_string(),
                locked_balance: account.locked_balance.to_string(),
                is_active: account.is_active.unwrap_or(true),
                last_reconciled: account
                    .last_reconciled
                    .map(|t| t.timestamp())
                    .unwrap_or(0),
            })),
            Err(e) => Err(Status::not_found(format!("Nostro account not found: {}", e))),
        }
    }

    async fn get_vostro_balance(
        &self,
        request: Request<AccountBalanceRequest>,
    ) -> Result<Response<AccountBalanceResponse>, Status> {
        let req = request.into_inner();

        match self
            .vostro_manager
            .get_account_by_bank_currency(&req.bank, &req.currency)
            .await
        {
            Ok(account) => Ok(Response::new(AccountBalanceResponse {
                account_id: account.id.to_string(),
                bank: account.bank,
                currency: account.currency,
                ledger_balance: account.ledger_balance.to_string(),
                available_balance: account.ledger_balance.to_string(), // Vostro has no locked balance
                locked_balance: "0".to_string(),
                is_active: account.is_active.unwrap_or(true),
                last_reconciled: 0, // Vostro doesn't track reconciliation
            })),
            Err(e) => Err(Status::not_found(format!("Vostro account not found: {}", e))),
        }
    }
}
