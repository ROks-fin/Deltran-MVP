// Core account monitoring logic

use crate::bank_client::BankClient;
use crate::camt_parser::CamtParser;
use crate::config::Config;
use crate::models::{AccountTransaction, FundingEvent};
use anyhow::Result;
use async_nats::Client as NatsClient;
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, error, warn};
use uuid::Uuid;
use rust_decimal::Decimal;

#[derive(Clone)]
pub struct AccountMonitor {
    config: Config,
    db_pool: Arc<PgPool>,
    nats_client: Arc<NatsClient>,
    bank_clients: Vec<Arc<BankClient>>,
    camt_parser: Arc<CamtParser>,
}

impl AccountMonitor {
    pub async fn new(config: Config) -> Result<Self> {
        // Connect to database
        let db_pool = Arc::new(
            PgPool::connect(&config.database_url)
                .await
                .expect("Failed to connect to database")
        );

        // Connect to NATS
        let nats_client = Arc::new(
            async_nats::connect(&config.nats_url)
                .await
                .expect("Failed to connect to NATS")
        );

        // Initialize bank clients for each EMI account
        let mut bank_clients = Vec::new();
        for account_config in &config.monitored_accounts {
            let client = Arc::new(BankClient::new(account_config.clone()));
            bank_clients.push(client);
        }

        let camt_parser = Arc::new(CamtParser::new());

        info!("✅ Account Monitor initialized with {} accounts", bank_clients.len());

        Ok(Self {
            config,
            db_pool,
            nats_client,
            bank_clients,
            camt_parser,
        })
    }

    /// Poll all monitored accounts for new transactions
    pub async fn poll_all_accounts(&self) -> Result<()> {
        for (idx, bank_client) in self.bank_clients.iter().enumerate() {
            info!("🔍 Polling account {}/{}: {}",
                  idx + 1,
                  self.bank_clients.len(),
                  bank_client.get_account_id());

            match bank_client.fetch_recent_transactions().await {
                Ok(transactions) => {
                    info!("📊 Found {} transactions for account {}",
                          transactions.len(),
                          bank_client.get_account_id());

                    for transaction in transactions {
                        self.process_transaction(transaction).await?;
                    }
                }
                Err(e) => {
                    error!("Failed to fetch transactions for account {}: {}",
                           bank_client.get_account_id(), e);
                }
            }
        }

        Ok(())
    }

    /// Listen for incoming camt.054 messages (push notifications from bank)
    pub async fn listen_camt054(&self) -> Result<()> {
        let mut subscriber = self.nats_client
            .subscribe("bank.camt054.incoming")
            .await?;

        info!("📥 Listening for camt.054 messages on NATS...");

        use futures::StreamExt;
        while let Some(msg) = subscriber.next().await {
            match String::from_utf8(msg.payload.to_vec()) {
                Ok(xml) => {
                    info!("📨 Received camt.054 message ({} bytes)", xml.len());

                    match self.camt_parser.parse_camt054(&xml) {
                        Ok(transactions) => {
                            info!("✅ Parsed {} transactions from camt.054", transactions.len());

                            for transaction in transactions {
                                if let Err(e) = self.process_transaction(transaction).await {
                                    error!("Failed to process transaction: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse camt.054: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to decode camt.054 message: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Process a single transaction (credit to EMI account)
    async fn process_transaction(&self, transaction: AccountTransaction) -> Result<()> {
        // Check if this is a CREDIT (incoming funds)
        if transaction.credit_debit_indicator != "CRDT" {
            return Ok(()); // Skip debits
        }

        info!(
            "💰 Processing CREDIT transaction: {} {} (ref: {})",
            transaction.amount,
            transaction.currency,
            transaction.end_to_end_id.as_ref().unwrap_or(&"N/A".to_string())
        );

        // Check if already processed
        if self.is_transaction_processed(&transaction.transaction_id).await? {
            info!("⏭️  Transaction {} already processed, skipping", transaction.transaction_id);
            return Ok(());
        }

        // Try to match with pending payment
        let payment_id = self.match_with_pending_payment(&transaction).await?;

        match payment_id {
            Some(payment_id) => {
                info!(
                    "✅ Matched transaction {} with payment {}",
                    transaction.transaction_id,
                    payment_id
                );

                // Create funding event
                let funding_event = FundingEvent {
                    id: Uuid::new_v4(),
                    payment_id,
                    transaction_id: transaction.transaction_id.clone(),
                    account_id: transaction.account_id.clone(),
                    amount: transaction.amount,
                    currency: transaction.currency.clone(),
                    end_to_end_id: transaction.end_to_end_id.clone(),
                    debtor_name: transaction.debtor_name.clone(),
                    debtor_account: transaction.debtor_account.clone(),
                    booking_date: transaction.booking_date,
                    value_date: transaction.value_date,
                    confirmed_at: Utc::now(),
                };

                // Save to database
                self.save_funding_event(&funding_event).await?;

                // Publish to NATS for Token Engine
                self.publish_funding_confirmed(&funding_event).await?;

                info!(
                    "🚀 Funding confirmed and published for payment {}",
                    payment_id
                );
            }
            None => {
                warn!(
                    "⚠️  No matching payment found for transaction {}. Storing for manual review.",
                    transaction.transaction_id
                );

                // Store unmatched transaction
                self.save_unmatched_transaction(&transaction).await?;
            }
        }

        Ok(())
    }

    /// Check if transaction already processed
    async fn is_transaction_processed(&self, transaction_id: &str) -> Result<bool> {
        let result = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM funding_events WHERE transaction_id = $1"
        )
        .bind(transaction_id)
        .fetch_one(self.db_pool.as_ref())
        .await?;

        Ok(result > 0)
    }

    /// Match transaction with pending payment using end_to_end_id or amount+currency
    async fn match_with_pending_payment(&self, transaction: &AccountTransaction) -> Result<Option<Uuid>> {
        // Try exact match by end_to_end_id
        if let Some(ref e2e_id) = transaction.end_to_end_id {
            let payment_id: Option<Uuid> = sqlx::query_scalar(
                "SELECT deltran_tx_id FROM payments WHERE end_to_end_id = $1 AND status = 'PENDING'"
            )
            .bind(e2e_id)
            .fetch_optional(self.db_pool.as_ref())
            .await?;

            if payment_id.is_some() {
                return Ok(payment_id);
            }
        }

        // Fallback: Match by amount + currency + recent timestamp
        let payment_id: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT deltran_tx_id
            FROM payments
            WHERE settlement_amount = $1
              AND currency = $2
              AND status = 'PENDING'
              AND created_at > NOW() - INTERVAL '24 hours'
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .bind(transaction.amount)
        .bind(&transaction.currency)
        .fetch_optional(self.db_pool.as_ref())
        .await?;

        Ok(payment_id)
    }

    /// Save funding event to database
    async fn save_funding_event(&self, event: &FundingEvent) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO funding_events (
                id, payment_id, transaction_id, account_id, amount, currency,
                end_to_end_id, debtor_name, debtor_account, booking_date, value_date, confirmed_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#
        )
        .bind(&event.id)
        .bind(&event.payment_id)
        .bind(&event.transaction_id)
        .bind(&event.account_id)
        .bind(&event.amount)
        .bind(&event.currency)
        .bind(&event.end_to_end_id)
        .bind(&event.debtor_name)
        .bind(&event.debtor_account)
        .bind(&event.booking_date)
        .bind(&event.value_date)
        .bind(&event.confirmed_at)
        .execute(self.db_pool.as_ref())
        .await?;

        Ok(())
    }

    /// Save unmatched transaction for manual review
    async fn save_unmatched_transaction(&self, transaction: &AccountTransaction) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO unmatched_transactions (
                transaction_id, account_id, amount, currency, end_to_end_id,
                debtor_name, debtor_account, booking_date, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            "#
        )
        .bind(&transaction.transaction_id)
        .bind(&transaction.account_id)
        .bind(&transaction.amount)
        .bind(&transaction.currency)
        .bind(&transaction.end_to_end_id)
        .bind(&transaction.debtor_name)
        .bind(&transaction.debtor_account)
        .bind(&transaction.booking_date)
        .execute(self.db_pool.as_ref())
        .await?;

        Ok(())
    }

    /// Publish funding confirmation to Token Engine
    async fn publish_funding_confirmed(&self, event: &FundingEvent) -> Result<()> {
        let subject = "deltran.funding.confirmed";
        let payload = serde_json::to_vec(event)?;

        self.nats_client.publish(subject, payload.into()).await?;

        info!(
            "📤 Published funding confirmation: {} for payment {}",
            event.id,
            event.payment_id
        );

        Ok(())
    }
}
