use crate::error::{Result, SettlementError};
use crate::settlement::executor::SettlementRequest;
use chrono::{Datelike, Timelike, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

pub struct SettlementValidator {
    db_pool: Arc<PgPool>,
}

#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl SettlementValidator {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    pub async fn validate_settlement(&self, request: &SettlementRequest) -> Result<()> {
        let result = self.validate_settlement_detailed(
            request.obligation_id,
            &request.from_bank,
            &request.to_bank,
            &request.amount,
            &request.currency,
        ).await?;

        if !result.is_valid {
            return Err(SettlementError::Internal(format!(
                "Validation failed: {}",
                result.errors.join(", ")
            )));
        }

        Ok(())
    }

    pub async fn validate_settlement_detailed(
        &self,
        obligation_id: Uuid,
        from_bank: &str,
        to_bank: &str,
        amount: &Decimal,
        currency: &str,
    ) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Validate amount
        if *amount <= Decimal::ZERO {
            result.is_valid = false;
            result.errors.push("Amount must be greater than zero".to_string());
        }

        if *amount > Decimal::from_str("1000000000").unwrap() {
            result.is_valid = false;
            result.errors.push("Amount exceeds maximum limit".to_string());
        }

        // Validate banks are different
        if from_bank == to_bank {
            result.is_valid = false;
            result.errors.push("Source and destination banks cannot be the same".to_string());
        }

        // Check bank existence and status
        match self.check_bank_status(from_bank).await {
            Ok(is_active) => {
                if !is_active {
                    result.is_valid = false;
                    result.errors.push(format!("Source bank {} is not active", from_bank));
                }
            }
            Err(e) => {
                result.is_valid = false;
                result.errors.push(format!("Source bank {} not found: {}", from_bank, e));
            }
        }

        match self.check_bank_status(to_bank).await {
            Ok(is_active) => {
                if !is_active {
                    result.is_valid = false;
                    result.errors.push(format!("Destination bank {} is not active", to_bank));
                }
            }
            Err(e) => {
                result.is_valid = false;
                result.errors.push(format!("Destination bank {} not found: {}", to_bank, e));
            }
        }

        // Check nostro account existence and balance
        match self.check_nostro_account(from_bank, currency).await {
            Ok((exists, is_active, available_balance)) => {
                if !exists {
                    result.is_valid = false;
                    result.errors.push(format!(
                        "Nostro account not found for bank {} currency {}",
                        from_bank, currency
                    ));
                } else if !is_active {
                    result.is_valid = false;
                    result.errors.push(format!(
                        "Nostro account inactive for bank {} currency {}",
                        from_bank, currency
                    ));
                } else if available_balance < *amount {
                    result.is_valid = false;
                    result.errors.push(format!(
                        "Insufficient funds: required {}, available {}",
                        amount, available_balance
                    ));
                } else if available_balance < *amount * Decimal::from_str("1.1").unwrap() {
                    result.warnings.push(
                        "Available balance is less than 110% of settlement amount".to_string()
                    );
                }
            }
            Err(e) => {
                result.is_valid = false;
                result.errors.push(format!("Error checking nostro account: {}", e));
            }
        }

        // Check vostro account existence
        match self.check_vostro_account(to_bank, currency).await {
            Ok((exists, is_active)) => {
                if !exists {
                    result.is_valid = false;
                    result.errors.push(format!(
                        "Vostro account not found for bank {} currency {}",
                        to_bank, currency
                    ));
                } else if !is_active {
                    result.is_valid = false;
                    result.errors.push(format!(
                        "Vostro account inactive for bank {} currency {}",
                        to_bank, currency
                    ));
                }
            }
            Err(e) => {
                result.is_valid = false;
                result.errors.push(format!("Error checking vostro account: {}", e));
            }
        }

        // Check settlement window
        if let Err(e) = self.check_settlement_window(currency).await {
            result.warnings.push(format!(
                "Settlement window check: {}. Settlement may be delayed.",
                e
            ));
        }

        // Check for duplicate settlement
        if self.check_duplicate_settlement(obligation_id).await? {
            result.is_valid = false;
            result.errors.push(format!(
                "Settlement already exists for obligation {}",
                obligation_id
            ));
        }

        if result.is_valid {
            info!(
                "Validation passed for settlement {} -> {} {} {}",
                from_bank, to_bank, amount, currency
            );
        } else {
            warn!(
                "Validation failed for settlement {} -> {} {} {}: {:?}",
                from_bank, to_bank, amount, currency, result.errors
            );
        }

        Ok(result)
    }

    async fn check_bank_status(&self, bank_code: &str) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT is_active
            FROM banks
            WHERE bank_code = $1
            "#,
            bank_code
        )
        .fetch_optional(&*self.db_pool)
        .await?;

        match result {
            Some(row) => Ok(row.is_active.unwrap_or(false)),
            None => Err(SettlementError::AccountNotFound(format!(
                "Bank {} not found",
                bank_code
            ))),
        }
    }

    async fn check_nostro_account(
        &self,
        bank: &str,
        currency: &str,
    ) -> Result<(bool, bool, Decimal)> {
        let result = sqlx::query!(
            r#"
            SELECT is_active, available_balance
            FROM nostro_accounts
            WHERE bank = $1 AND currency = $2
            "#,
            bank,
            currency
        )
        .fetch_optional(&*self.db_pool)
        .await?;

        match result {
            Some(row) => Ok((
                true,
                row.is_active.unwrap_or(false),
                row.available_balance,
            )),
            None => Ok((false, false, Decimal::ZERO)),
        }
    }

    async fn check_vostro_account(&self, bank: &str, currency: &str) -> Result<(bool, bool)> {
        let result = sqlx::query!(
            r#"
            SELECT is_active
            FROM vostro_accounts
            WHERE bank = $1 AND currency = $2
            "#,
            bank,
            currency
        )
        .fetch_optional(&*self.db_pool)
        .await?;

        match result {
            Some(row) => Ok((true, row.is_active.unwrap_or(false))),
            None => Ok((false, false)),
        }
    }

    async fn check_settlement_window(&self, currency: &str) -> Result<()> {
        let now = Utc::now();
        let current_time = now.time();
        let day_of_week = now.weekday().num_days_from_monday() as i32 + 1;

        let window = sqlx::query!(
            r#"
            SELECT window_start, window_end, days_of_week
            FROM settlement_windows
            WHERE currency = $1 AND is_active = true
            LIMIT 1
            "#,
            currency
        )
        .fetch_optional(&*self.db_pool)
        .await?;

        match window {
            Some(w) => {
                // Check if today is a valid day
                if let Some(days_json_value) = w.days_of_week {
                    // Parse the JSON string to a Value
                    if let Ok(days_parsed) = serde_json::from_str::<serde_json::Value>(&days_json_value) {
                        if let Some(days_array) = days_parsed.as_array() {
                            let day_numbers: Vec<i32> = days_array
                                .iter()
                                .filter_map(|v| v.as_i64().map(|n| n as i32))
                                .collect();

                            if !day_numbers.contains(&day_of_week) {
                                return Err(SettlementError::SettlementWindowClosed(format!(
                                    "Not a valid settlement day for {}",
                                    currency
                                )));
                            }
                        }
                    }
                }

                // Check if current time is within window
                let start = w.window_start;
                let end = w.window_end;
                {
                    let current_seconds = current_time.hour() * 3600
                        + current_time.minute() * 60
                        + current_time.second();
                    let start_seconds = start.hour() * 3600 + start.minute() * 60 + start.second();
                    let end_seconds = end.hour() * 3600 + end.minute() * 60 + end.second();

                    if current_seconds < start_seconds || current_seconds > end_seconds {
                        return Err(SettlementError::SettlementWindowClosed(format!(
                            "Current time {:02}:{:02} is outside window {:02}:{:02}-{:02}:{:02}",
                            current_time.hour(),
                            current_time.minute(),
                            start.hour(),
                            start.minute(),
                            end.hour(),
                            end.minute()
                        )));
                    }
                }

                Ok(())
            }
            None => {
                // No window defined - allow settlement anytime
                Ok(())
            }
        }
    }

    async fn check_duplicate_settlement(&self, obligation_id: Uuid) -> Result<bool> {
        let count = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM settlement_transactions
            WHERE obligation_id = $1
                AND status NOT IN ('failed', 'rolled_back')
            "#,
            obligation_id
        )
        .fetch_one(&*self.db_pool)
        .await?;

        Ok(count.count.unwrap_or(0) > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_creation() {
        let result = ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
        };

        assert!(result.is_valid);
        assert_eq!(result.errors.len(), 0);
    }
}
