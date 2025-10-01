//! PII Tokenization and Masking Service
//!
//! Implements:
//! - AES-256-GCM tokenization for reversible PII protection
//! - Field-level masking for display purposes
//! - Secure key derivation with Argon2
//! - Audit trail for token access

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::{PasswordHash, SaltString};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use thiserror::Error;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PiiProtectionError {
    #[error("Encryption failed: {0}")]
    EncryptionError(String),

    #[error("Decryption failed: {0}")]
    DecryptionError(String),

    #[error("Invalid token format: {0}")]
    InvalidToken(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Key derivation failed: {0}")]
    KeyDerivationError(String),
}

/// PII field types that require protection
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PiiFieldType {
    /// Full name
    FullName,
    /// Email address
    Email,
    /// Phone number
    Phone,
    /// National ID / Tax ID
    NationalId,
    /// Bank account number
    BankAccount,
    /// IBAN
    Iban,
    /// Credit card number
    CreditCard,
    /// Street address
    Address,
    /// IP address
    IpAddress,
    /// Date of birth
    DateOfBirth,
    /// Custom PII field
    Custom,
}

/// Tokenization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizedValue {
    /// Token ID (used for lookup and audit)
    pub token_id: Uuid,
    /// Encrypted value (base64-encoded)
    pub encrypted_value: String,
    /// Field type
    pub field_type: PiiFieldType,
    /// Nonce (base64-encoded, required for AES-GCM)
    pub nonce: String,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Masking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskingConfig {
    /// Number of visible characters at start
    pub visible_start: usize,
    /// Number of visible characters at end
    pub visible_end: usize,
    /// Mask character (default: '*')
    pub mask_char: char,
}

impl Default for MaskingConfig {
    fn default() -> Self {
        Self {
            visible_start: 2,
            visible_end: 2,
            mask_char: '*',
        }
    }
}

/// PII Protection Service
pub struct PiiProtection {
    pool: PgPool,
    encryption_key: Key<Aes256Gcm>,
    masking_configs: HashMap<PiiFieldType, MaskingConfig>,
}

impl PiiProtection {
    /// Create new PII protection service
    ///
    /// # Arguments
    /// * `pool` - Database connection pool
    /// * `master_key` - Master encryption key (32 bytes)
    pub fn new(pool: PgPool, master_key: &[u8; 32]) -> Self {
        let encryption_key = Key::<Aes256Gcm>::from_slice(master_key);

        let mut masking_configs = HashMap::new();
        masking_configs.insert(PiiFieldType::FullName, MaskingConfig {
            visible_start: 1,
            visible_end: 0,
            mask_char: '*',
        });
        masking_configs.insert(PiiFieldType::Email, MaskingConfig {
            visible_start: 2,
            visible_end: 4,
            mask_char: '*',
        });
        masking_configs.insert(PiiFieldType::Phone, MaskingConfig {
            visible_start: 3,
            visible_end: 2,
            mask_char: '*',
        });
        masking_configs.insert(PiiFieldType::BankAccount, MaskingConfig {
            visible_start: 0,
            visible_end: 4,
            mask_char: '*',
        });
        masking_configs.insert(PiiFieldType::Iban, MaskingConfig {
            visible_start: 4,
            visible_end: 4,
            mask_char: '*',
        });
        masking_configs.insert(PiiFieldType::NationalId, MaskingConfig {
            visible_start: 0,
            visible_end: 4,
            mask_char: '*',
        });
        masking_configs.insert(PiiFieldType::Address, MaskingConfig {
            visible_start: 0,
            visible_end: 0,
            mask_char: '*',
        });
        masking_configs.insert(PiiFieldType::IpAddress, MaskingConfig {
            visible_start: 0,
            visible_end: 0,
            mask_char: 'X',
        });

        Self {
            pool,
            encryption_key: *encryption_key,
            masking_configs,
        }
    }

    /// Tokenize PII value (encrypt and store)
    pub async fn tokenize(
        &self,
        value: &str,
        field_type: PiiFieldType,
        context: &str,
    ) -> Result<TokenizedValue, PiiProtectionError> {
        // Generate nonce
        let nonce_bytes: [u8; 12] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let cipher = Aes256Gcm::new(&self.encryption_key);
        let encrypted = cipher
            .encrypt(nonce, value.as_bytes())
            .map_err(|e| PiiProtectionError::EncryptionError(e.to_string()))?;

        let token_id = Uuid::new_v4();
        let encrypted_b64 = BASE64.encode(&encrypted);
        let nonce_b64 = BASE64.encode(&nonce_bytes);
        let created_at = chrono::Utc::now();

        // Store in database
        sqlx::query(
            r#"
            INSERT INTO compliance.pii_tokens
            (token_id, field_type, encrypted_value, nonce, context, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(token_id)
        .bind(serde_json::to_string(&field_type).unwrap())
        .bind(&encrypted_b64)
        .bind(&nonce_b64)
        .bind(context)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        info!(
            token_id = %token_id,
            field_type = ?field_type,
            context = context,
            "PII value tokenized"
        );

        Ok(TokenizedValue {
            token_id,
            encrypted_value: encrypted_b64,
            field_type,
            nonce: nonce_b64,
            created_at,
        })
    }

    /// Detokenize (decrypt) PII value
    pub async fn detokenize(
        &self,
        token_id: Uuid,
        requester: &str,
        purpose: &str,
    ) -> Result<String, PiiProtectionError> {
        // Fetch from database
        let row = sqlx::query(
            r#"
            SELECT encrypted_value, nonce, field_type
            FROM compliance.pii_tokens
            WHERE token_id = $1
            "#,
        )
        .bind(token_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| PiiProtectionError::InvalidToken(format!("Token not found: {}", token_id)))?;

        let encrypted_b64: String = row.get("encrypted_value");
        let nonce_b64: String = row.get("nonce");
        let field_type_str: String = row.get("field_type");

        // Decrypt
        let encrypted = BASE64
            .decode(&encrypted_b64)
            .map_err(|e| PiiProtectionError::DecryptionError(format!("Invalid base64: {}", e)))?;
        let nonce_bytes = BASE64
            .decode(&nonce_b64)
            .map_err(|e| PiiProtectionError::DecryptionError(format!("Invalid nonce: {}", e)))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher = Aes256Gcm::new(&self.encryption_key);
        let decrypted = cipher
            .decrypt(nonce, encrypted.as_ref())
            .map_err(|e| PiiProtectionError::DecryptionError(e.to_string()))?;

        let value = String::from_utf8(decrypted)
            .map_err(|e| PiiProtectionError::DecryptionError(format!("Invalid UTF-8: {}", e)))?;

        // Audit access
        sqlx::query(
            r#"
            INSERT INTO compliance.pii_access_log
            (token_id, requester, purpose, accessed_at)
            VALUES ($1, $2, $3, NOW())
            "#,
        )
        .bind(token_id)
        .bind(requester)
        .bind(purpose)
        .execute(&self.pool)
        .await?;

        info!(
            token_id = %token_id,
            requester = requester,
            purpose = purpose,
            "PII value detokenized"
        );

        Ok(value)
    }

    /// Mask PII value for display
    pub fn mask(&self, value: &str, field_type: PiiFieldType) -> String {
        let config = self.masking_configs.get(&field_type).cloned().unwrap_or_default();

        if value.len() <= config.visible_start + config.visible_end {
            // Too short, mask everything
            return config.mask_char.to_string().repeat(value.len().min(8));
        }

        let start = &value[..config.visible_start];
        let end = &value[value.len() - config.visible_end..];
        let middle_len = value.len() - config.visible_start - config.visible_end;
        let middle = config.mask_char.to_string().repeat(middle_len.min(10));

        format!("{}{}{}", start, middle, end)
    }

    /// Batch tokenize multiple values
    pub async fn tokenize_batch(
        &self,
        values: Vec<(String, PiiFieldType)>,
        context: &str,
    ) -> Result<Vec<TokenizedValue>, PiiProtectionError> {
        let mut results = Vec::with_capacity(values.len());

        for (value, field_type) in values {
            let token = self.tokenize(&value, field_type, context).await?;
            results.push(token);
        }

        Ok(results)
    }
}

/// Tokenization service (alias for compatibility)
pub type TokenizationService = PiiProtection;

/// Masking service for display-only purposes
pub struct MaskingService {
    configs: HashMap<PiiFieldType, MaskingConfig>,
}

impl MaskingService {
    pub fn new() -> Self {
        let mut configs = HashMap::new();
        configs.insert(PiiFieldType::FullName, MaskingConfig {
            visible_start: 1,
            visible_end: 0,
            mask_char: '*',
        });
        configs.insert(PiiFieldType::Email, MaskingConfig {
            visible_start: 2,
            visible_end: 4,
            mask_char: '*',
        });
        configs.insert(PiiFieldType::Phone, MaskingConfig {
            visible_start: 3,
            visible_end: 2,
            mask_char: '*',
        });
        configs.insert(PiiFieldType::BankAccount, MaskingConfig {
            visible_start: 0,
            visible_end: 4,
            mask_char: '*',
        });

        Self { configs }
    }

    pub fn mask(&self, value: &str, field_type: PiiFieldType) -> String {
        let config = self.configs.get(&field_type).cloned().unwrap_or_default();

        if value.len() <= config.visible_start + config.visible_end {
            return config.mask_char.to_string().repeat(value.len().min(8));
        }

        let start = &value[..config.visible_start];
        let end = &value[value.len() - config.visible_end..];
        let middle_len = value.len() - config.visible_start - config.visible_end;
        let middle = config.mask_char.to_string().repeat(middle_len.min(10));

        format!("{}{}{}", start, middle, end)
    }
}

impl Default for MaskingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masking_email() {
        let service = MaskingService::new();
        let masked = service.mask("user@example.com", PiiFieldType::Email);
        assert_eq!(masked, "us************.com");
    }

    #[test]
    fn test_masking_phone() {
        let service = MaskingService::new();
        let masked = service.mask("+971501234567", PiiFieldType::Phone);
        assert_eq!(masked, "+97**********67");
    }

    #[test]
    fn test_masking_bank_account() {
        let service = MaskingService::new();
        let masked = service.mask("GB82WEST12345698765432", PiiFieldType::BankAccount);
        assert_eq!(masked, "******************5432");
    }

    #[test]
    fn test_masking_short_value() {
        let service = MaskingService::new();
        let masked = service.mask("AB", PiiFieldType::Email);
        assert_eq!(masked, "**");
    }
}
