use crate::error::{Result, SettlementError};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostroAccount {
    pub id: Uuid,
    pub bank: String,
    pub account_number: String,
    pub currency: String,
    pub ledger_balance: Decimal,
    pub available_balance: Decimal,
    pub locked_balance: Decimal,
    pub last_reconciled: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
}

pub struct NostroAccountManager {
    db_pool: Arc<PgPool>,
}

impl NostroAccountManager {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    pub async fn create_account(
        &self,
        bank: &str,
        account_number: &str,
        currency: &str,
        initial_balance: Decimal,
    ) -> Result<NostroAccount> {
        let account_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO nostro_accounts (
                id, bank, account_number, currency,
                ledger_balance, available_balance, locked_balance,
                is_active, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8)
            "#
        )
        .bind(account_id)
        .bind(bank)
        .bind(account_number)
        .bind(currency)
        .bind(initial_balance)
        .bind(initial_balance)
        .bind(Decimal::ZERO)
        .bind(Utc::now())
        .execute(&*self.db_pool)
        .await?;

        info!(
            "Created nostro account {} for bank {} currency {}",
            account_id, bank, currency
        );

        self.get_account(&account_id).await
    }

    pub async fn get_account(&self, account_id: &Uuid) -> Result<NostroAccount> {
        let row = sqlx::query(
            r#"
            SELECT
                id, bank, account_number, currency,
                ledger_balance, available_balance, locked_balance,
                last_reconciled, is_active, created_at
            FROM nostro_accounts
            WHERE id = $1
            "#
        )
        .bind(account_id)
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| {
            SettlementError::AccountNotFound(format!("Nostro account {}", account_id))
        })?;

        Ok(NostroAccount {
            id: row.try_get("id")?,
            bank: row.try_get("bank")?,
            account_number: row.try_get("account_number")?,
            currency: row.try_get("currency")?,
            ledger_balance: row.try_get("ledger_balance")?,
            available_balance: row.try_get("available_balance")?,
            locked_balance: row.try_get("locked_balance")?,
            last_reconciled: row.try_get("last_reconciled").ok(),
            is_active: row.try_get("is_active").ok(),
            created_at: row.try_get("created_at")?,
        })
    }

    pub async fn get_account_by_bank_currency(
        &self,
        bank: &str,
        currency: &str,
    ) -> Result<NostroAccount> {
        let row = sqlx::query(
            r#"
            SELECT
                id, bank, account_number, currency,
                ledger_balance, available_balance, locked_balance,
                last_reconciled, is_active, created_at
            FROM nostro_accounts
            WHERE bank = $1 AND currency = $2
            "#
        )
        .bind(bank)
        .bind(currency)
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| {
            SettlementError::AccountNotFound(format!("Nostro account {}:{}", bank, currency))
        })?;

        Ok(NostroAccount {
            id: row.try_get("id")?,
            bank: row.try_get("bank")?,
            account_number: row.try_get("account_number")?,
            currency: row.try_get("currency")?,
            ledger_balance: row.try_get("ledger_balance")?,
            available_balance: row.try_get("available_balance")?,
            locked_balance: row.try_get("locked_balance")?,
            last_reconciled: row.try_get("last_reconciled").ok(),
            is_active: row.try_get("is_active").ok(),
            created_at: row.try_get("created_at")?,
        })
    }

    pub async fn list_accounts(&self, bank: Option<&str>) -> Result<Vec<NostroAccount>> {
        let rows = if let Some(bank_code) = bank {
            sqlx::query(
                r#"
                SELECT
                    id, bank, account_number, currency,
                    ledger_balance, available_balance, locked_balance,
                    last_reconciled, is_active, created_at
                FROM nostro_accounts
                WHERE bank = $1
                ORDER BY bank, currency
                "#
            )
            .bind(bank_code)
            .fetch_all(&*self.db_pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT
                    id, bank, account_number, currency,
                    ledger_balance, available_balance, locked_balance,
                    last_reconciled, is_active, created_at
                FROM nostro_accounts
                ORDER BY bank, currency
                "#
            )
            .fetch_all(&*self.db_pool)
            .await?
        };

        let accounts = rows.iter().map(|row| {
            Ok(NostroAccount {
                id: row.try_get("id")?,
                bank: row.try_get("bank")?,
                account_number: row.try_get("account_number")?,
                currency: row.try_get("currency")?,
                ledger_balance: row.try_get("ledger_balance")?,
                available_balance: row.try_get("available_balance")?,
                locked_balance: row.try_get("locked_balance")?,
                last_reconciled: row.try_get("last_reconciled").ok(),
                is_active: row.try_get("is_active").ok(),
                created_at: row.try_get("created_at")?,
            })
        }).collect::<Result<Vec<NostroAccount>>>()?;

        Ok(accounts)
    }

    pub async fn update_balance(
        &self,
        account_id: &Uuid,
        new_balance: Decimal,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE nostro_accounts
            SET ledger_balance = $1,
                available_balance = $1 - locked_balance
            WHERE id = $2
            "#
        )
        .bind(new_balance)
        .bind(account_id)
        .execute(&*self.db_pool)
        .await?;

        info!(
            "Updated nostro account {} balance to {}",
            account_id, new_balance
        );

        Ok(())
    }

    pub async fn deactivate_account(&self, account_id: &Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE nostro_accounts
            SET is_active = false
            WHERE id = $1
            "#
        )
        .bind(account_id)
        .execute(&*self.db_pool)
        .await?;

        info!("Deactivated nostro account {}", account_id);

        Ok(())
    }

    pub async fn activate_account(&self, account_id: &Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE nostro_accounts
            SET is_active = true
            WHERE id = $1
            "#
        )
        .bind(account_id)
        .execute(&*self.db_pool)
        .await?;

        info!("Activated nostro account {}", account_id);

        Ok(())
    }

    pub async fn update_reconciliation_timestamp(&self, account_id: &Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE nostro_accounts
            SET last_reconciled = $1
            WHERE id = $2
            "#
        )
        .bind(Utc::now())
        .bind(account_id)
        .execute(&*self.db_pool)
        .await?;

        Ok(())
    }

    pub async fn get_total_balance(&self, currency: &str) -> Result<Decimal> {
        let result = sqlx::query(
            r#"
            SELECT COALESCE(SUM(ledger_balance), 0) as total
            FROM nostro_accounts
            WHERE currency = $1 AND is_active = true
            "#
        )
        .bind(currency)
        .fetch_one(&*self.db_pool)
        .await?;

        let total: Option<Decimal> = result.try_get("total").ok();
        Ok(total.unwrap_or(Decimal::ZERO))
    }
}
