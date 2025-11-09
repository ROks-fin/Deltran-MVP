use crate::errors::Result;
use crate::models::{Token, TokenBalance};
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Pool, Postgres};
use std::time::Duration;
use uuid::Uuid;

pub struct Database {
    pool: Pool<Postgres>,
}

impl Database {
    pub async fn new(database_url: &str, max_connections: u32) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await?;

        Ok(Database { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Create a new token (mint)
    pub async fn create_token(
        &self,
        currency: &str,
        amount: Decimal,
        bank_id: Uuid,
        clearing_window: i64,
        reference: &str,
    ) -> Result<Token> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();

        let token = sqlx::query_as::<_, Token>(
            r#"
            INSERT INTO tokens (id, currency, amount, bank_id, status, clearing_window, created_at, reference)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(format!("x{}", currency.to_uppercase()))
        .bind(amount)
        .bind(bank_id)
        .bind("ACTIVE")
        .bind(clearing_window)
        .bind(created_at)
        .bind(reference)
        .fetch_one(&self.pool)
        .await?;

        Ok(token)
    }

    /// Get token by ID
    pub async fn get_token(&self, token_id: Uuid) -> Result<Option<Token>> {
        let token = sqlx::query_as::<_, Token>(
            r#"
            SELECT * FROM tokens WHERE id = $1
            "#,
        )
        .bind(token_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    /// Burn a token
    pub async fn burn_token(&self, token_id: Uuid, amount: Decimal) -> Result<Token> {
        let burned_at = Utc::now();

        let token = sqlx::query_as::<_, Token>(
            r#"
            UPDATE tokens
            SET amount = amount - $1,
                status = CASE
                    WHEN amount - $1 <= 0 THEN 'BURNED'
                    ELSE status
                END,
                burned_at = CASE
                    WHEN amount - $1 <= 0 THEN $2
                    ELSE burned_at
                END
            WHERE id = $3 AND amount >= $1
            RETURNING *
            "#,
        )
        .bind(amount)
        .bind(burned_at)
        .bind(token_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(token)
    }

    /// Transfer tokens between banks
    pub async fn transfer_tokens(
        &self,
        from_bank_id: Uuid,
        to_bank_id: Uuid,
        currency: &str,
        amount: Decimal,
        clearing_window: i64,
        reference: &str,
    ) -> Result<(Token, Token)> {
        let mut tx = self.pool.begin().await?;

        // Deduct from sender
        let from_token = sqlx::query_as::<_, Token>(
            r#"
            UPDATE tokens
            SET amount = amount - $1
            WHERE bank_id = $2 AND currency = $3 AND amount >= $1 AND status = 'ACTIVE'
            RETURNING *
            "#,
        )
        .bind(amount)
        .bind(from_bank_id)
        .bind(format!("x{}", currency.to_uppercase()))
        .fetch_one(&mut *tx)
        .await?;

        // Credit to receiver (create new token or update existing)
        let to_token = sqlx::query_as::<_, Token>(
            r#"
            INSERT INTO tokens (id, currency, amount, bank_id, status, clearing_window, created_at, reference)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (bank_id, currency, clearing_window)
            DO UPDATE SET amount = tokens.amount + $3
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(format!("x{}", currency.to_uppercase()))
        .bind(amount)
        .bind(to_bank_id)
        .bind("ACTIVE")
        .bind(clearing_window)
        .bind(Utc::now())
        .bind(reference)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok((from_token, to_token))
    }

    /// Convert tokens from one currency to another
    pub async fn convert_tokens(
        &self,
        bank_id: Uuid,
        from_currency: &str,
        to_currency: &str,
        from_amount: Decimal,
        to_amount: Decimal,
        clearing_window: i64,
        reference: &str,
    ) -> Result<(Token, Token)> {
        let mut tx = self.pool.begin().await?;

        // Burn source currency tokens
        let from_token = sqlx::query_as::<_, Token>(
            r#"
            UPDATE tokens
            SET amount = amount - $1
            WHERE bank_id = $2 AND currency = $3 AND amount >= $1 AND status = 'ACTIVE'
            RETURNING *
            "#,
        )
        .bind(from_amount)
        .bind(bank_id)
        .bind(format!("x{}", from_currency.to_uppercase()))
        .fetch_one(&mut *tx)
        .await?;

        // Mint target currency tokens
        let to_token = sqlx::query_as::<_, Token>(
            r#"
            INSERT INTO tokens (id, currency, amount, bank_id, status, clearing_window, created_at, reference)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (bank_id, currency, clearing_window)
            DO UPDATE SET amount = tokens.amount + $3
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(format!("x{}", to_currency.to_uppercase()))
        .bind(to_amount)
        .bind(bank_id)
        .bind("ACTIVE")
        .bind(clearing_window)
        .bind(Utc::now())
        .bind(reference)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok((from_token, to_token))
    }

    /// Get token balance for a bank
    pub async fn get_balance(&self, bank_id: Uuid, currency: Option<&str>) -> Result<Vec<TokenBalance>> {
        let query = if let Some(curr) = currency {
            sqlx::query_as::<_, TokenBalance>(
                r#"
                SELECT
                    bank_id,
                    currency,
                    SUM(CASE WHEN status = 'ACTIVE' THEN amount ELSE 0 END) as available_balance,
                    SUM(CASE WHEN status = 'LOCKED' THEN amount ELSE 0 END) as locked_balance,
                    SUM(amount) as total_balance
                FROM tokens
                WHERE bank_id = $1 AND currency = $2 AND status NOT IN ('BURNED')
                GROUP BY bank_id, currency
                "#,
            )
            .bind(bank_id)
            .bind(format!("x{}", curr.to_uppercase()))
        } else {
            sqlx::query_as::<_, TokenBalance>(
                r#"
                SELECT
                    bank_id,
                    currency,
                    SUM(CASE WHEN status = 'ACTIVE' THEN amount ELSE 0 END) as available_balance,
                    SUM(CASE WHEN status = 'LOCKED' THEN amount ELSE 0 END) as locked_balance,
                    SUM(amount) as total_balance
                FROM tokens
                WHERE bank_id = $1 AND status NOT IN ('BURNED')
                GROUP BY bank_id, currency
                "#,
            )
            .bind(bank_id)
        };

        let balances = query.fetch_all(&self.pool).await?;

        Ok(balances)
    }

    /// Lock tokens for a transaction
    pub async fn lock_tokens(&self, bank_id: Uuid, currency: &str, amount: Decimal) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE tokens
            SET status = 'LOCKED'
            WHERE bank_id = $1 AND currency = $2 AND amount >= $3 AND status = 'ACTIVE'
            "#,
        )
        .bind(bank_id)
        .bind(format!("x{}", currency.to_uppercase()))
        .bind(amount)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Unlock tokens
    pub async fn unlock_tokens(&self, bank_id: Uuid, currency: &str, amount: Decimal) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE tokens
            SET status = 'ACTIVE'
            WHERE bank_id = $1 AND currency = $2 AND amount >= $3 AND status = 'LOCKED'
            "#,
        )
        .bind(bank_id)
        .bind(format!("x{}", currency.to_uppercase()))
        .bind(amount)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get current clearing window
    pub fn get_current_clearing_window(&self) -> i64 {
        let now = Utc::now();
        let hours = now.timestamp() / 3600;
        // Round down to nearest 6-hour window
        (hours / 6) * 6
    }
}