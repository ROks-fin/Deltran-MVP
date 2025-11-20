// API Handlers for Reconciliation Service

use crate::errors::{Result, TokenEngineError};
use crate::reconciliation::{
    ReconciliationService,
    camt053_processor::Camt053Statement,
    camt054_processor::Camt054Notification,
};
use actix_web::{get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> HttpResponse {
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            data: Some(data),
            error: None,
        })
    }

    fn error(message: String) -> HttpResponse {
        HttpResponse::BadRequest().json(ApiResponse::<T> {
            success: false,
            data: None,
            error: Some(message),
        })
    }
}

// ========== TIER 1: CAMT.054 Processing ==========

#[post("/reconciliation/camt054")]
async fn process_camt054(
    service: web::Data<Arc<ReconciliationService>>,
    notification: web::Json<Camt054Notification>,
) -> HttpResponse {
    match service.process_camt054_notification(notification.into_inner()).await {
        Ok(result) => {
            let response = serde_json::json!({
                "account_id": result.account_id,
                "ledger_balance": result.ledger_balance,
                "bank_balance": result.bank_balance,
                "difference": result.difference,
                "threshold_level": format!("{:?}", result.threshold_level),
                "action_taken": result.action_taken,
            });
            ApiResponse::ok(response)
        }
        Err(e) => ApiResponse::<serde_json::Value>::error(e.to_string()),
    }
}

// ========== TIER 2: Intradey Reconciliation ==========

#[post("/reconciliation/intradey/{account_id}")]
async fn run_intradey_reconciliation(
    service: web::Data<Arc<ReconciliationService>>,
    account_id: web::Path<Uuid>,
) -> HttpResponse {
    match service.run_intradey_reconciliation(*account_id).await {
        Ok(result) => {
            let response = serde_json::json!({
                "account_id": result.account_id,
                "account_number": result.account_number,
                "currency": result.currency,
                "ledger_balance": result.ledger_balance,
                "bank_balance": result.bank_balance,
                "difference": result.difference,
                "threshold_level": format!("{:?}", result.threshold_level),
                "timestamp": result.timestamp,
            });
            ApiResponse::ok(response)
        }
        Err(e) => ApiResponse::<serde_json::Value>::error(e.to_string()),
    }
}

#[post("/reconciliation/intradey/all")]
async fn run_intradey_reconciliation_all(
    service: web::Data<Arc<ReconciliationService>>,
) -> HttpResponse {
    match service.run_intradey_reconciliation_all().await {
        Ok(results) => {
            let response: Vec<_> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "account_id": r.account_id,
                        "account_number": r.account_number,
                        "currency": r.currency,
                        "ledger_balance": r.ledger_balance,
                        "bank_balance": r.bank_balance,
                        "difference": r.difference,
                        "threshold_level": format!("{:?}", r.threshold_level),
                    })
                })
                .collect();
            ApiResponse::ok(response)
        }
        Err(e) => ApiResponse::<serde_json::Value>::error(e.to_string()),
    }
}

// ========== TIER 3: EOD CAMT.053 Processing ==========

#[post("/reconciliation/eod")]
async fn process_eod_statement(
    service: web::Data<Arc<ReconciliationService>>,
    statement: web::Json<Camt053Statement>,
) -> HttpResponse {
    match service.process_eod_statement(statement.into_inner()).await {
        Ok(result) => {
            let response = serde_json::json!({
                "snapshot_id": result.snapshot_id,
                "account_id": result.account_id,
                "statement_date": result.statement_date,
                "ledger_balance": result.ledger_balance,
                "bank_balance": result.bank_balance,
                "difference": result.difference,
                "reconciled": result.reconciled,
                "entries_matched": result.entries_matched,
                "entries_unmatched": result.entries_unmatched,
                "threshold_level": format!("{:?}", result.threshold_level),
            });
            ApiResponse::ok(response)
        }
        Err(e) => ApiResponse::<serde_json::Value>::error(e.to_string()),
    }
}

// ========== Monitoring & Status ==========

#[get("/reconciliation/summary")]
async fn get_reconciliation_summary(
    service: web::Data<Arc<ReconciliationService>>,
) -> HttpResponse {
    match service.get_reconciliation_summary().await {
        Ok(summary) => {
            let response = serde_json::json!({
                "total_accounts": summary.total_accounts,
                "accounts_ok": summary.accounts_ok,
                "accounts_mismatch": summary.accounts_mismatch,
                "open_discrepancies": summary.open_discrepancies,
                "critical_discrepancies": summary.critical_discrepancies,
                "timestamp": summary.timestamp,
                "health_percentage": if summary.total_accounts > 0 {
                    (summary.accounts_ok as f64 / summary.total_accounts as f64) * 100.0
                } else {
                    100.0
                },
            });
            ApiResponse::ok(response)
        }
        Err(e) => ApiResponse::<serde_json::Value>::error(e.to_string()),
    }
}

#[get("/reconciliation/health")]
async fn health_check(
    service: web::Data<Arc<ReconciliationService>>,
) -> HttpResponse {
    match service.get_reconciliation_summary().await {
        Ok(summary) => {
            let health_status = if summary.critical_discrepancies > 0 {
                "CRITICAL"
            } else if summary.accounts_mismatch > 0 {
                "WARNING"
            } else {
                "HEALTHY"
            };

            let response = serde_json::json!({
                "status": health_status,
                "total_accounts": summary.total_accounts,
                "accounts_ok": summary.accounts_ok,
                "accounts_mismatch": summary.accounts_mismatch,
                "critical_discrepancies": summary.critical_discrepancies,
            });

            if health_status == "CRITICAL" {
                HttpResponse::ServiceUnavailable().json(response)
            } else if health_status == "WARNING" {
                HttpResponse::Ok().json(response)
            } else {
                HttpResponse::Ok().json(response)
            }
        }
        Err(e) => ApiResponse::<serde_json::Value>::error(e.to_string()),
    }
}

// ========== Route Configuration ==========

pub fn configure_reconciliation_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            // TIER 1: Real-time
            .service(process_camt054)
            // TIER 2: Intradey
            .service(run_intradey_reconciliation)
            .service(run_intradey_reconciliation_all)
            // TIER 3: EOD
            .service(process_eod_statement)
            // Monitoring
            .service(get_reconciliation_summary)
            .service(health_check)
    );
}
