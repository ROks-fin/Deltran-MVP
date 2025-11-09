use crate::errors::Result;
use crate::models::{
    ClearingWindow, ClearingWindowStatus, NetPosition, Obligation,
    SettlementInstruction, SettlementStatus, SettlementType,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Pool, Postgres, Row};
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

    /// Create a new obligation for instant settlement
    pub async fn create_obligation(
        &self,
        corridor: &str,
        amount_sent: Decimal,
        amount_credited: Decimal,
        sent_currency: &str,
        credited_currency: &str,
        bank_debtor_id: Uuid,
        bank_creditor_id: Uuid,
        clearing_window: i64,
        transaction_id: Option<Uuid>,
        reference: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<Obligation> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();

        let obligation = sqlx::query_as::<_, Obligation>(
            r#"
            INSERT INTO obligations (
                id, corridor, amount_sent, amount_credited,
                sent_currency, credited_currency,
                bank_debtor_id, bank_creditor_id,
                status, clearing_window, transaction_id,
                created_at, metadata, reference
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(corridor)
        .bind(amount_sent)
        .bind(amount_credited)
        .bind(sent_currency)
        .bind(credited_currency)
        .bind(bank_debtor_id)
        .bind(bank_creditor_id)
        .bind("PENDING")
        .bind(clearing_window)
        .bind(transaction_id)
        .bind(created_at)
        .bind(metadata)
        .bind(reference)
        .fetch_one(&self.pool)
        .await?;

        Ok(obligation)
    }

    /// Get obligation by ID
    pub async fn get_obligation(&self, obligation_id: Uuid) -> Result<Option<Obligation>> {
        let obligation = sqlx::query_as::<_, Obligation>(
            r#"
            SELECT * FROM obligations WHERE id = $1
            "#,
        )
        .bind(obligation_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(obligation)
    }

    /// Get all obligations for a clearing window
    pub async fn get_obligations_by_window(&self, clearing_window: i64) -> Result<Vec<Obligation>> {
        let obligations = sqlx::query_as::<_, Obligation>(
            r#"
            SELECT * FROM obligations
            WHERE clearing_window = $1
            ORDER BY created_at
            "#,
        )
        .bind(clearing_window)
        .fetch_all(&self.pool)
        .await?;

        Ok(obligations)
    }

    /// Get pending obligations for netting
    pub async fn get_pending_obligations(&self, clearing_window: i64) -> Result<Vec<Obligation>> {
        let obligations = sqlx::query_as::<_, Obligation>(
            r#"
            SELECT * FROM obligations
            WHERE clearing_window = $1 AND status = 'PENDING'
            ORDER BY created_at
            "#,
        )
        .bind(clearing_window)
        .fetch_all(&self.pool)
        .await?;

        Ok(obligations)
    }

    /// Update obligation status
    pub async fn update_obligation_status(
        &self,
        obligation_id: Uuid,
        status: &str,
    ) -> Result<bool> {
        let settled_at = if status == "SETTLED" {
            Some(Utc::now())
        } else {
            None
        };

        let result = sqlx::query(
            r#"
            UPDATE obligations
            SET status = $1, settled_at = $2
            WHERE id = $3
            "#,
        )
        .bind(status)
        .bind(settled_at)
        .bind(obligation_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Batch update obligations to netted status
    pub async fn mark_obligations_as_netted(&self, obligation_ids: &[Uuid]) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE obligations
            SET status = 'NETTED'
            WHERE id = ANY($1) AND status = 'PENDING'
            "#,
        )
        .bind(obligation_ids)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Save net positions after netting calculation
    pub async fn save_net_positions(&self, positions: &[NetPosition]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for position in positions {
            sqlx::query(
                r#"
                INSERT INTO net_positions (
                    id, bank_id, currency, net_amount,
                    gross_inflow, gross_outflow,
                    clearing_window, calculated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (bank_id, currency, clearing_window)
                DO UPDATE SET
                    net_amount = $4,
                    gross_inflow = $5,
                    gross_outflow = $6,
                    calculated_at = $8
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(position.bank_id)
            .bind(&position.currency)
            .bind(position.net_amount)
            .bind(position.gross_inflow)
            .bind(position.gross_outflow)
            .bind(position.clearing_window)
            .bind(position.calculated_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Create settlement instruction
    pub async fn create_settlement_instruction(
        &self,
        clearing_window: i64,
        from_bank_id: Uuid,
        to_bank_id: Uuid,
        currency: &str,
        amount: Decimal,
        instruction_type: SettlementType,
    ) -> Result<SettlementInstruction> {
        let id = Uuid::new_v4();
        let created_at = Utc::now();

        let instruction = SettlementInstruction {
            id,
            clearing_window,
            from_bank_id,
            to_bank_id,
            currency: currency.to_string(),
            amount,
            instruction_type,
            status: SettlementStatus::Pending,
            created_at,
            executed_at: None,
        };

        sqlx::query(
            r#"
            INSERT INTO settlement_instructions (
                id, clearing_window, from_bank_id, to_bank_id,
                currency, amount, instruction_type, status, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(id)
        .bind(clearing_window)
        .bind(from_bank_id)
        .bind(to_bank_id)
        .bind(currency)
        .bind(amount)
        .bind(serde_json::to_value(&instruction_type.clone()).unwrap())
        .bind("PENDING")
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        Ok(instruction)
    }

    /// Get current clearing window info
    pub async fn get_clearing_window_info(&self, window_id: i64) -> Result<Option<ClearingWindow>> {
        let row = sqlx::query(
            r#"
            SELECT
                $1 as window_id,
                COUNT(DISTINCT t.id) as total_transactions,
                COUNT(DISTINCT o.id) as total_obligations,
                BOOL_OR(o.status = 'NETTED') as netting_completed
            FROM obligations o
            LEFT JOIN transactions t ON t.obligation_id = o.id
            WHERE o.clearing_window = $1
            "#,
        )
        .bind(window_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let window = ClearingWindow {
                window_id,
                start_time: DateTime::from_timestamp(window_id * 3600, 0).unwrap(),
                end_time: DateTime::from_timestamp((window_id + 6) * 3600, 0).unwrap(),
                status: ClearingWindowStatus::Open,
                total_transactions: row.get("total_transactions"),
                total_obligations: row.get("total_obligations"),
                netting_completed: row.get("netting_completed"),
            };
            Ok(Some(window))
        } else {
            Ok(None)
        }
    }

    /// Check if bank has sufficient liquidity
    pub async fn check_bank_liquidity(
        &self,
        _bank_id: Uuid,
        _currency: &str,
        _amount: Decimal,
    ) -> Result<bool> {
        // This would integrate with liquidity management system
        // For now, return true for demo
        Ok(true)
    }

    /// Get corridor statistics for ML predictions
    pub async fn get_corridor_stats(&self, corridor: &str, days: i32) -> Result<serde_json::Value> {
        let stats = sqlx::query(
            r#"
            SELECT
                $1 as corridor,
                AVG(amount_sent) as avg_volume,
                COUNT(*) as transaction_count,
                AVG(CASE WHEN status = 'SETTLED' THEN 1 ELSE 0 END) as settlement_rate
            FROM obligations
            WHERE corridor = $1
                AND created_at >= NOW() - INTERVAL '$2 days'
            "#,
        )
        .bind(corridor)
        .bind(days)
        .fetch_one(&self.pool)
        .await?;

        Ok(serde_json::json!({
            "corridor": corridor,
            "avg_volume": stats.get::<Option<Decimal>, _>("avg_volume"),
            "transaction_count": stats.get::<i64, _>("transaction_count"),
            "settlement_rate": stats.get::<Option<Decimal>, _>("settlement_rate"),
        }))
    }

    /// Get current clearing window based on time
    pub fn get_current_clearing_window(&self) -> i64 {
        let now = Utc::now();
        let hours = now.timestamp() / 3600;
        // Round down to nearest 6-hour window
        (hours / 6) * 6
    }

    /// Check if clearing window is still open
    pub fn is_window_open(&self, window_id: i64) -> bool {
        let current_window = self.get_current_clearing_window();
        window_id == current_window
    }
}
