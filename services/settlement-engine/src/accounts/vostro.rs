use crate::error::{Result, SettlementError};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VostroAccount {
    pub id: Uuid,
    pub bank: String,
    pub account_number: String,
    pub currency: String,
    pub ledger_balance: Decimal,
    pub credit_limit: Option<Decimal>,
    pub is_active: Option<bool>,
    pub created_at: DateTime<Utc>,
}

pub struct VostroAccountManager {
    db_pool: Arc<PgPool>,
}

impl VostroAccountManager {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    pub async fn create_account(
        &self,
        bank: &str,
        account_number: &str,
        currency: &str,
        credit_limit: Option<Decimal>,
    ) -> Result<VostroAccount> {
        let account_id = Uuid::new_v4();

        sqlx::query!(
            r#"
            INSERT INTO vostro_accounts (
                id, bank, account_number, currency,
                ledger_balance, credit_limit, is_active, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, true, $7)
            "#,
            account_id,
            bank,
            account_number,
            currency,
            Decimal::ZERO,
            credit_limit,
            Utc::now()
        )
        .execute(&*self.db_pool)
        .await?;

        info!(
            "Created vostro account {} for bank {} currency {}",
            account_id, bank, currency
        );

        self.get_account(&account_id).await
    }

    pub async fn get_account(&self, account_id: &Uuid) -> Result<VostroAccount> {
        let account = sqlx::query_as!(
            VostroAccount,
            r#"
            SELECT
                id, bank, account_number, currency,
                ledger_balance, credit_limit, is_active, created_at
            FROM vostro_accounts
            WHERE id = $1
            "#,
            account_id
        )
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| {
            SettlementError::AccountNotFound(format!("Vostro account {}", account_id))
        })?;

        Ok(account)
    }

    pub async fn get_account_by_bank_currency(
        &self,
        bank: &str,
        currency: &str,
    ) -> Result<VostroAccount> {
        let account = sqlx::query_as!(
            VostroAccount,
            r#"
            SELECT
                id, bank, account_number, currency,
                ledger_balance, credit_limit, is_active, created_at
            FROM vostro_accounts
            WHERE bank = $1 AND currency = $2
            "#,
            bank,
            currency
        )
        .fetch_optional(&*self.db_pool)
        .await?
        .ok_or_else(|| {
            SettlementError::AccountNotFound(format!("Vostro account {}:{}", bank, currency))
        })?;

        Ok(account)
    }

    pub async fn list_accounts(&self, bank: Option<&str>) -> Result<Vec<VostroAccount>> {
        let accounts = if let Some(bank_code) = bank {
            sqlx::query_as!(
                VostroAccount,
                r#"
                SELECT
                    id, bank, account_number, currency,
                    ledger_balance, credit_limit, is_active, created_at
                FROM vostro_accounts
                WHERE bank = $1
                ORDER BY bank, currency
                "#,
                bank_code
            )
            .fetch_all(&*self.db_pool)
            .await?
        } else {
            sqlx::query_as!(
                VostroAccount,
                r#"
                SELECT
                    id, bank, account_number, currency,
                    ledger_balance, credit_limit, is_active, created_at
                FROM vostro_accounts
                ORDER BY bank, currency
                "#
            )
            .fetch_all(&*self.db_pool)
            .await?
        };

        Ok(accounts)
    }

    pub async fn credit_account(
        &self,
        account_id: &Uuid,
        amount: Decimal,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE vostro_accounts
            SET ledger_balance = ledger_balance + $1
            WHERE id = $2
            "#,
            amount,
            account_id
        )
        .execute(&*self.db_pool)
        .await?;

        info!("Credited vostro account {} with {}", account_id, amount);

        Ok(())
    }

    pub async fn debit_account(
        &self,
        account_id: &Uuid,
        amount: Decimal,
    ) -> Result<()> {
        // Check if debit would exceed credit limit
        let account = self.get_account(account_id).await?;
        let new_balance = account.ledger_balance - amount;

        if let Some(limit) = account.credit_limit {
            if new_balance.abs() > limit {
                return Err(SettlementError::Internal(format!(
                    "Debit would exceed credit limit: {} vs {}",
                    new_balance.abs(),
                    limit
                )));
            }
        }

        sqlx::query!(
            r#"
            UPDATE vostro_accounts
            SET ledger_balance = ledger_balance - $1
            WHERE id = $2
            "#,
            amount,
            account_id
        )
        .execute(&*self.db_pool)
        .await?;

        info!("Debited vostro account {} with {}", account_id, amount);

        Ok(())
    }

    pub async fn deactivate_account(&self, account_id: &Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE vostro_accounts
            SET is_active = false
            WHERE id = $1
            "#,
            account_id
        )
        .execute(&*self.db_pool)
        .await?;

        info!("Deactivated vostro account {}", account_id);

        Ok(())
    }

    pub async fn activate_account(&self, account_id: &Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE vostro_accounts
            SET is_active = true
            WHERE id = $1
            "#,
            account_id
        )
        .execute(&*self.db_pool)
        .await?;

        info!("Activated vostro account {}", account_id);

        Ok(())
    }

    pub async fn update_credit_limit(
        &self,
        account_id: &Uuid,
        new_limit: Option<Decimal>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE vostro_accounts
            SET credit_limit = $1
            WHERE id = $2
            "#,
            new_limit,
            account_id
        )
        .execute(&*self.db_pool)
        .await?;

        info!(
            "Updated credit limit for vostro account {} to {:?}",
            account_id, new_limit
        );

        Ok(())
    }
}
