use crate::database::Database;
use crate::errors::{Result, TokenEngineError};
use crate::nats::NatsProducer;
use crate::models::{
    BurnTokenRequest, ConvertTokenRequest, MintTokenRequest, TokenBalance, TokenEvent,
    TokenEventType, TokenResponse, TransferTokenRequest,
};
use chrono::Utc;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

pub struct TokenService {
    db: Arc<Database>,
    nats: Arc<NatsProducer>,
    redis: ConnectionManager,
}

impl TokenService {
    pub async fn new(db: Arc<Database>, nats: Arc<NatsProducer>, redis: ConnectionManager) -> Self {
        TokenService { db, nats, redis }
    }

    /// Mint new tokens
    pub async fn mint_tokens(&self, request: MintTokenRequest) -> Result<TokenResponse> {
        // Validate request
        validator::Validate::validate(&request)
            .map_err(|e| TokenEngineError::Validation(e.to_string()))?;

        // Check bank exists (would normally query banks table)
        self.verify_bank_exists(request.bank_id).await?;

        // Get current clearing window
        let clearing_window = self.db.get_current_clearing_window();

        // Create token in database
        let token = self
            .db
            .create_token(
                &request.currency,
                request.amount,
                request.bank_id,
                clearing_window,
                &request.reference,
            )
            .await?;

        // Publish event to NATS
        let event = TokenEvent {
            event_type: TokenEventType::Minted,
            token_id: token.id,
            bank_id: request.bank_id,
            currency: token.currency.clone(),
            amount: request.amount,
            reference: request.reference.clone(),
            timestamp: Utc::now(),
            metadata: request.metadata,
        };

        if let Err(e) = self.nats.publish_token_event(&event).await {
            error!("Failed to publish mint event: {}", e);
        }

        // Update cache
        self.update_balance_cache(request.bank_id, &token.currency)
            .await?;

        info!(
            "Minted {} {} for bank {} (token: {})",
            request.amount, token.currency, request.bank_id, token.id
        );

        Ok(TokenResponse {
            id: token.id,
            currency: token.currency,
            amount: token.amount,
            bank_id: token.bank_id,
            status: token.status,
            created_at: token.created_at,
            transaction_reference: request.reference,
        })
    }

    /// Burn tokens
    pub async fn burn_tokens(&self, request: BurnTokenRequest) -> Result<TokenResponse> {
        // Validate request
        validator::Validate::validate(&request)
            .map_err(|e| TokenEngineError::Validation(e.to_string()))?;

        // Get token
        let token = self
            .db
            .get_token(request.token_id)
            .await?
            .ok_or_else(|| TokenEngineError::TokenNotFound(request.token_id))?;

        // Check sufficient balance
        if token.amount < request.amount {
            return Err(TokenEngineError::InsufficientBalance {
                required: request.amount.to_string(),
                available: token.amount.to_string(),
            });
        }

        // Burn token
        let updated_token = self.db.burn_token(request.token_id, request.amount).await?;

        // Publish event
        let event = TokenEvent {
            event_type: TokenEventType::Burned,
            token_id: request.token_id,
            bank_id: token.bank_id,
            currency: token.currency.clone(),
            amount: request.amount,
            reference: request.reference.clone(),
            timestamp: Utc::now(),
            metadata: Some(serde_json::json!({
                "destination_account": request.destination_account,
            })),
        };

        if let Err(e) = self.nats.publish_token_event(&event).await {
            error!("Failed to publish burn event: {}", e);
        }

        // Update cache
        self.update_balance_cache(token.bank_id, &token.currency)
            .await?;

        info!(
            "Burned {} {} from token {} for bank {}",
            request.amount, token.currency, request.token_id, token.bank_id
        );

        Ok(TokenResponse {
            id: updated_token.id,
            currency: updated_token.currency,
            amount: updated_token.amount,
            bank_id: updated_token.bank_id,
            status: updated_token.status,
            created_at: updated_token.created_at,
            transaction_reference: request.reference,
        })
    }

    /// Transfer tokens between banks
    pub async fn transfer_tokens(&self, request: TransferTokenRequest) -> Result<TokenResponse> {
        // Validate request
        validator::Validate::validate(&request)
            .map_err(|e| TokenEngineError::Validation(e.to_string()))?;

        // Verify banks exist
        self.verify_bank_exists(request.from_bank_id).await?;
        self.verify_bank_exists(request.to_bank_id).await?;

        // Check balance
        let balance = self.get_balance(request.from_bank_id, Some(&request.currency)).await?;
        if balance.is_empty() || balance[0].available_balance < request.amount {
            return Err(TokenEngineError::InsufficientBalance {
                required: request.amount.to_string(),
                available: balance
                    .first()
                    .map(|b| b.available_balance.to_string())
                    .unwrap_or_else(|| "0".to_string()),
            });
        }

        // Get clearing window
        let clearing_window = self.db.get_current_clearing_window();

        // Execute transfer
        let (from_token, to_token) = self
            .db
            .transfer_tokens(
                request.from_bank_id,
                request.to_bank_id,
                &request.currency,
                request.amount,
                clearing_window,
                &request.reference,
            )
            .await?;

        // Publish transfer event
        let event = TokenEvent {
            event_type: TokenEventType::Transferred,
            token_id: to_token.id,
            bank_id: request.to_bank_id,
            currency: to_token.currency.clone(),
            amount: request.amount,
            reference: request.reference.clone(),
            timestamp: Utc::now(),
            metadata: Some(serde_json::json!({
                "from_bank_id": request.from_bank_id,
                "to_bank_id": request.to_bank_id,
            })),
        };

        if let Err(e) = self.nats.publish_token_event(&event).await {
            error!("Failed to publish transfer event: {}", e);
        }

        // Update caches
        self.update_balance_cache(request.from_bank_id, &from_token.currency)
            .await?;
        self.update_balance_cache(request.to_bank_id, &to_token.currency)
            .await?;

        info!(
            "Transferred {} {} from bank {} to bank {}",
            request.amount, request.currency, request.from_bank_id, request.to_bank_id
        );

        Ok(TokenResponse {
            id: to_token.id,
            currency: to_token.currency,
            amount: request.amount,
            bank_id: request.to_bank_id,
            status: to_token.status,
            created_at: to_token.created_at,
            transaction_reference: request.reference,
        })
    }

    /// Convert tokens from one currency to another
    pub async fn convert_tokens(&self, request: ConvertTokenRequest) -> Result<TokenResponse> {
        // Validate request
        validator::Validate::validate(&request)
            .map_err(|e| TokenEngineError::Validation(e.to_string()))?;

        // Verify bank exists
        self.verify_bank_exists(request.bank_id).await?;

        // Get FX rate (would normally call FX service)
        let fx_rate = self.get_fx_rate(&request.from_currency, &request.to_currency).await?;
        let to_amount = request.amount * fx_rate;

        // Check balance
        let balance = self.get_balance(request.bank_id, Some(&request.from_currency)).await?;
        if balance.is_empty() || balance[0].available_balance < request.amount {
            return Err(TokenEngineError::InsufficientBalance {
                required: request.amount.to_string(),
                available: balance
                    .first()
                    .map(|b| b.available_balance.to_string())
                    .unwrap_or_else(|| "0".to_string()),
            });
        }

        // Get clearing window
        let clearing_window = self.db.get_current_clearing_window();

        // Execute conversion
        let (from_token, to_token) = self
            .db
            .convert_tokens(
                request.bank_id,
                &request.from_currency,
                &request.to_currency,
                request.amount,
                to_amount,
                clearing_window,
                &request.reference,
            )
            .await?;

        // Publish conversion event
        let event = TokenEvent {
            event_type: TokenEventType::Converted,
            token_id: to_token.id,
            bank_id: request.bank_id,
            currency: to_token.currency.clone(),
            amount: to_amount,
            reference: request.reference.clone(),
            timestamp: Utc::now(),
            metadata: Some(serde_json::json!({
                "from_currency": request.from_currency,
                "to_currency": request.to_currency,
                "from_amount": request.amount,
                "to_amount": to_amount,
                "fx_rate": fx_rate,
            })),
        };

        if let Err(e) = self.nats.publish_token_event(&event).await {
            error!("Failed to publish conversion event: {}", e);
        }

        // Update caches
        self.update_balance_cache(request.bank_id, &from_token.currency)
            .await?;
        self.update_balance_cache(request.bank_id, &to_token.currency)
            .await?;

        info!(
            "Converted {} {} to {} {} for bank {} (rate: {})",
            request.amount, request.from_currency, to_amount, request.to_currency, request.bank_id, fx_rate
        );

        Ok(TokenResponse {
            id: to_token.id,
            currency: to_token.currency,
            amount: to_amount,
            bank_id: request.bank_id,
            status: to_token.status,
            created_at: to_token.created_at,
            transaction_reference: request.reference,
        })
    }

    /// Get token balance for a bank
    pub async fn get_balance(&self, bank_id: Uuid, currency: Option<&str>) -> Result<Vec<TokenBalance>> {
        // Try cache first
        if let Some(curr) = currency {
            let cache_key = format!("balance:{}:{}", bank_id, curr);
            if let Ok(cached) = self.redis.clone().get::<String, String>(cache_key).await {
                if let Ok(balance) = serde_json::from_str::<TokenBalance>(&cached) {
                    return Ok(vec![balance]);
                }
            }
        }

        // Get from database
        let balances = self.db.get_balance(bank_id, currency).await?;

        // Cache the result
        for balance in &balances {
            let cache_key = format!("balance:{}:{}", bank_id, balance.currency);
            let cached = serde_json::to_string(&balance)
                .map_err(|e| TokenEngineError::Internal(e.to_string()))?;
            let _: () = self
                .redis
                .clone()
                .set_ex(cache_key, cached, 60)
                .await
                .map_err(|e| TokenEngineError::Redis(e))?;
        }

        Ok(balances)
    }

    /// Helper: Update balance cache
    async fn update_balance_cache(&self, bank_id: Uuid, currency: &str) -> Result<()> {
        let cache_key = format!("balance:{}:{}", bank_id, currency);
        let _: () = self
            .redis
            .clone()
            .del(cache_key)
            .await
            .map_err(|e| TokenEngineError::Redis(e))?;
        Ok(())
    }

    /// Helper: Verify bank exists
    async fn verify_bank_exists(&self, _bank_id: Uuid) -> Result<()> {
        // In production, this would query the banks table
        // For now, we'll accept all UUIDs
        Ok(())
    }

    /// Helper: Get FX rate
    async fn get_fx_rate(&self, from: &str, to: &str) -> Result<Decimal> {
        // In production, this would call the FX service
        // For now, return mock rates
        let rate = match (from, to) {
            ("INR", "AED") => Decimal::from_str("0.044")?,
            ("AED", "INR") => Decimal::from_str("22.73")?,
            ("USD", "AED") => Decimal::from_str("3.67")?,
            ("AED", "USD") => Decimal::from_str("0.27")?,
            ("INR", "USD") => Decimal::from_str("0.012")?,
            ("USD", "INR") => Decimal::from_str("83.33")?,
            _ => {
                return Err(TokenEngineError::InvalidCurrencyPair {
                    from: from.to_string(),
                    to: to.to_string(),
                })
            }
        };

        Ok(rate)
    }
}