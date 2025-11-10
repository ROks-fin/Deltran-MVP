use crate::error::{Result, SettlementError};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
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

        sqlx::query!(
            r#"
            INSERT INTO nostro_accounts (
                id, bank, account_number, currency,
                ledger_balance, available_balance, locked_balance,
                is_active, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8)
            "#,
            account_id,
            bank,
            account_number,
            currency,
            initial_balance,
            initial_balance,
            Decimal::ZERO,
            Utc::now()
        )
        .execute(&*self.db_pool)
        .await?;

        info!(
            "Created nostro account {} for bank {} currency {}",
            account_id, bank, currency
        );

        self.get_account(&account_id).await
    }

    pub async fn get_account(&self, account_id: &Uuid) -> Result<NostroAccount> {
        let account = sqlx::query_as!(
            NostroAccount,
            r#"
            SELECT
                id, bank, account_number, currency,
                ledger_balance, available_balance, locked_balance,
                last_reconciled, is_active, created_at
            FROM nostro_accounts
            WHERE id = $1
            "#,
            account_id
        )
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| {
            SettlementError::AccountNotFound(format!("Nostro account {}", account_id))
        })?;

        Ok(account)
    }

    pub async fn get_account_by_bank_currency(
        &self,
        bank: &str,
        currency: &str,
    ) -> Result<NostroAccount> {
        let account = sqlx::query_as!(
            NostroAccount,
            r#"
            SELECT
                id, bank, account_number, currency,
                ledger_balance, available_balance, locked_balance,
                last_reconciled, is_active, created_at
            FROM nostro_accounts
            WHERE bank = $1 AND currency = $2
            "#,
            bank,
            currency
        )
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| {
            SettlementError::AccountNotFound(format!("Nostro account {}:{}", bank, currency))
        })?;

        Ok(account)
    }

    pub async fn list_accounts(&self, bank: Option<&str>) -> Result<Vec<NostroAccount>> {
        let accounts = if let Some(bank_code) = bank {
            sqlx::query_as!(
                NostroAccount,
                r#"
                SELECT
                    id, bank, account_number, currency,
                    ledger_balance, available_balance, locked_balance,
                    last_reconciled, is_active, created_at
                FROM nostro_accounts
                WHERE bank = $1
                ORDER BY bank, currency
                "#,
                bank_code
            )
            .fetch_all(&*self.db_pool)
            .await?
        } else {
            sqlx::query_as!(
                NostroAccount,
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

        Ok(accounts)
    }

    pub async fn update_balance(
        &self,
        account_id: &Uuid,
        new_balance: Decimal,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE nostro_accounts
            SET ledger_balance = $1,
                available_balance = $1 - locked_balance
            WHERE id = $2
            "#,
            new_balance,
            account_id
        )
        .execute(&*self.db_pool)
        .await?;

        info!(
            "Updated nostro account {} balance to {}",
            account_id, new_balance
        );

        Ok(())
    }

    pub async fn deactivate_account(&self, account_id: &Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE nostro_accounts
            SET is_active = false
            WHERE id = $1
            "#,
            account_id
        )
        .execute(&*self.db_pool)
        .await?;

        info!("Deactivated nostro account {}", account_id);

        Ok(())
    }

    pub async fn activate_account(&self, account_id: &Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE nostro_accounts
            SET is_active = true
            WHERE id = $1
            "#,
            account_id
        )
        .execute(&*self.db_pool)
        .await?;

        info!("Activated nostro account {}", account_id);

        Ok(())
    }

    pub async fn update_reconciliation_timestamp(&self, account_id: &Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE nostro_accounts
            SET last_reconciled = $1
            WHERE id = $2
            "#,
            Utc::now(),
            account_id
        )
        .execute(&*self.db_pool)
        .await?;

        Ok(())
    }

    pub async fn get_total_balance(&self, currency: &str) -> Result<Decimal> {
        let result = sqlx::query!(
            r#"
            SELECT COALESCE(SUM(ledger_balance), 0) as total
            FROM nostro_accounts
            WHERE currency = $1 AND is_active = true
            "#,
            currency
        )
        .fetch_one(&*self.db_pool)
        .await?;

        Ok(result.total.unwrap_or(Decimal::ZERO))
    }
}
