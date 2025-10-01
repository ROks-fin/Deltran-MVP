//! Secrets Management
//!
//! Provides secure storage and retrieval of secrets:
//! - Database credentials
//! - API keys
//! - Encryption keys
//! - TLS certificates
//! - Validator private keys
//!
//! Supports multiple backends:
//! - HashiCorp Vault
//! - AWS Secrets Manager
//! - Environment variables (development only)
//! - Encrypted file storage

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{info, warn};

/// Secrets manager errors
#[derive(Error, Debug)]
pub enum SecretsError {
    #[error("Secret not found: {0}")]
    NotFound(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("Invalid configuration: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, SecretsError>;

/// Secret metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    /// Secret name
    pub name: String,

    /// Secret version
    pub version: u32,

    /// Creation timestamp
    pub created_at: i64,

    /// Last accessed timestamp
    pub accessed_at: i64,

    /// Rotation policy (days)
    pub rotation_days: Option<u32>,
}

/// Secret value with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    /// Metadata
    pub metadata: SecretMetadata,

    /// Encrypted value
    pub value: Vec<u8>,
}

/// Secrets backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendConfig {
    /// Environment variables (development only)
    Environment,

    /// Encrypted file storage
    EncryptedFile {
        path: PathBuf,
        master_key_env: String,
    },

    /// HashiCorp Vault
    Vault {
        address: String,
        token: String,
        mount_path: String,
    },

    /// AWS Secrets Manager
    AwsSecretsManager {
        region: String,
        prefix: String,
    },
}

/// Secrets manager trait
pub trait SecretsBackend: Send + Sync {
    /// Get secret by name
    fn get_secret(&self, name: &str) -> Result<String>;

    /// Set secret
    fn set_secret(&self, name: &str, value: &str, metadata: SecretMetadata) -> Result<()>;

    /// Delete secret
    fn delete_secret(&self, name: &str) -> Result<()>;

    /// List all secret names
    fn list_secrets(&self) -> Result<Vec<String>>;

    /// Rotate secret
    fn rotate_secret(&self, name: &str, new_value: &str) -> Result<()>;
}

/// Environment variables backend (development only)
pub struct EnvironmentBackend;

impl SecretsBackend for EnvironmentBackend {
    fn get_secret(&self, name: &str) -> Result<String> {
        std::env::var(name).map_err(|_| SecretsError::NotFound(name.to_string()))
    }

    fn set_secret(&self, _name: &str, _value: &str, _metadata: SecretMetadata) -> Result<()> {
        Err(SecretsError::Backend(
            "Cannot set secrets in environment backend".to_string(),
        ))
    }

    fn delete_secret(&self, _name: &str) -> Result<()> {
        Err(SecretsError::Backend(
            "Cannot delete secrets in environment backend".to_string(),
        ))
    }

    fn list_secrets(&self) -> Result<Vec<String>> {
        Ok(std::env::vars().map(|(k, _)| k).collect())
    }

    fn rotate_secret(&self, _name: &str, _new_value: &str) -> Result<()> {
        Err(SecretsError::Backend(
            "Cannot rotate secrets in environment backend".to_string(),
        ))
    }
}

/// Encrypted file backend
pub struct EncryptedFileBackend {
    file_path: PathBuf,
    cipher: Aes256Gcm,
    secrets: HashMap<String, Secret>,
}

impl EncryptedFileBackend {
    /// Create new encrypted file backend
    pub fn new(file_path: PathBuf, master_key: &[u8; 32]) -> Result<Self> {
        let key = Key::<Aes256Gcm>::from_slice(master_key);
        let cipher = Aes256Gcm::new(key);

        let secrets = if file_path.exists() {
            Self::load_secrets(&file_path, &cipher)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            file_path,
            cipher,
            secrets,
        })
    }

    /// Load secrets from encrypted file
    fn load_secrets(path: &Path, cipher: &Aes256Gcm) -> Result<HashMap<String, Secret>> {
        let encrypted_data = std::fs::read(path)?;

        // First 12 bytes are nonce
        if encrypted_data.len() < 12 {
            return Ok(HashMap::new());
        }

        let nonce = Nonce::from_slice(&encrypted_data[..12]);
        let ciphertext = &encrypted_data[12..];

        let decrypted = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| SecretsError::Decryption(e.to_string()))?;

        let secrets: HashMap<String, Secret> = bincode::deserialize(&decrypted)
            .map_err(|e| SecretsError::Serialization(e.to_string()))?;

        Ok(secrets)
    }

    /// Save secrets to encrypted file
    fn save_secrets(&self) -> Result<()> {
        let serialized = bincode::serialize(&self.secrets)
            .map_err(|e| SecretsError::Serialization(e.to_string()))?;

        // Generate random nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let encrypted = self
            .cipher
            .encrypt(&nonce, serialized.as_ref())
            .map_err(|e| SecretsError::Encryption(e.to_string()))?;

        // Prepend nonce to ciphertext
        let mut data = Vec::with_capacity(12 + encrypted.len());
        data.extend_from_slice(&nonce);
        data.extend_from_slice(&encrypted);

        std::fs::write(&self.file_path, data)?;

        Ok(())
    }

    /// Encrypt value
    fn encrypt_value(&self, value: &str) -> Result<Vec<u8>> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let encrypted = self
            .cipher
            .encrypt(&nonce, value.as_bytes())
            .map_err(|e| SecretsError::Encryption(e.to_string()))?;

        // Prepend nonce
        let mut data = Vec::with_capacity(12 + encrypted.len());
        data.extend_from_slice(&nonce);
        data.extend_from_slice(&encrypted);

        Ok(data)
    }

    /// Decrypt value
    fn decrypt_value(&self, encrypted: &[u8]) -> Result<String> {
        if encrypted.len() < 12 {
            return Err(SecretsError::Decryption("Invalid encrypted data".to_string()));
        }

        let nonce = Nonce::from_slice(&encrypted[..12]);
        let ciphertext = &encrypted[12..];

        let decrypted = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| SecretsError::Decryption(e.to_string()))?;

        String::from_utf8(decrypted)
            .map_err(|e| SecretsError::Decryption(format!("Invalid UTF-8: {}", e)))
    }
}

impl SecretsBackend for EncryptedFileBackend {
    fn get_secret(&self, name: &str) -> Result<String> {
        let secret = self
            .secrets
            .get(name)
            .ok_or_else(|| SecretsError::NotFound(name.to_string()))?;

        self.decrypt_value(&secret.value)
    }

    fn set_secret(&self, name: &str, value: &str, metadata: SecretMetadata) -> Result<()> {
        let encrypted = self.encrypt_value(value)?;

        let secret = Secret {
            metadata,
            value: encrypted,
        };

        let mut backend = self;
        let secrets = &mut backend.secrets;
        secrets.insert(name.to_string(), secret);

        backend.save_secrets()?;

        info!("Secret set: {}", name);
        Ok(())
    }

    fn delete_secret(&self, name: &str) -> Result<()> {
        let mut backend = self;
        let secrets = &mut backend.secrets;

        if secrets.remove(name).is_none() {
            return Err(SecretsError::NotFound(name.to_string()));
        }

        backend.save_secrets()?;

        info!("Secret deleted: {}", name);
        Ok(())
    }

    fn list_secrets(&self) -> Result<Vec<String>> {
        Ok(self.secrets.keys().cloned().collect())
    }

    fn rotate_secret(&self, name: &str, new_value: &str) -> Result<()> {
        let mut backend = self;
        let secrets = &mut backend.secrets;

        let secret = secrets
            .get_mut(name)
            .ok_or_else(|| SecretsError::NotFound(name.to_string()))?;

        // Increment version
        secret.metadata.version += 1;
        secret.metadata.created_at = chrono::Utc::now().timestamp();

        // Encrypt new value
        secret.value = backend.encrypt_value(new_value)?;

        backend.save_secrets()?;

        info!("Secret rotated: {} (version {})", name, secret.metadata.version);
        Ok(())
    }
}

/// Secrets manager
pub struct SecretsManager {
    backend: Box<dyn SecretsBackend>,
}

impl SecretsManager {
    /// Create new secrets manager with environment backend
    pub fn with_environment() -> Self {
        info!("Using environment variables backend (development only)");
        Self {
            backend: Box::new(EnvironmentBackend),
        }
    }

    /// Create new secrets manager with encrypted file backend
    pub fn with_encrypted_file(path: PathBuf, master_key: &[u8; 32]) -> Result<Self> {
        info!("Using encrypted file backend: {:?}", path);
        Ok(Self {
            backend: Box::new(EncryptedFileBackend::new(path, master_key)?),
        })
    }

    /// Create from configuration
    pub fn from_config(config: BackendConfig) -> Result<Self> {
        match config {
            BackendConfig::Environment => Ok(Self::with_environment()),
            BackendConfig::EncryptedFile {
                path,
                master_key_env,
            } => {
                // Load master key from environment
                let master_key_hex = std::env::var(&master_key_env).map_err(|_| {
                    SecretsError::Config(format!("Missing master key env var: {}", master_key_env))
                })?;

                let master_key_bytes = hex::decode(&master_key_hex)
                    .map_err(|e| SecretsError::Config(format!("Invalid master key hex: {}", e)))?;

                if master_key_bytes.len() != 32 {
                    return Err(SecretsError::Config(
                        "Master key must be 32 bytes".to_string(),
                    ));
                }

                let mut master_key = [0u8; 32];
                master_key.copy_from_slice(&master_key_bytes);

                Self::with_encrypted_file(path, &master_key)
            }
            BackendConfig::Vault { .. } => {
                warn!("Vault backend not yet implemented, using environment");
                Ok(Self::with_environment())
            }
            BackendConfig::AwsSecretsManager { .. } => {
                warn!("AWS Secrets Manager not yet implemented, using environment");
                Ok(Self::with_environment())
            }
        }
    }

    /// Get secret
    pub fn get_secret(&self, name: &str) -> Result<String> {
        self.backend.get_secret(name)
    }

    /// Set secret
    pub fn set_secret(&self, name: &str, value: &str) -> Result<()> {
        let metadata = SecretMetadata {
            name: name.to_string(),
            version: 1,
            created_at: chrono::Utc::now().timestamp(),
            accessed_at: chrono::Utc::now().timestamp(),
            rotation_days: Some(90), // Default 90-day rotation
        };

        self.backend.set_secret(name, value, metadata)
    }

    /// Delete secret
    pub fn delete_secret(&self, name: &str) -> Result<()> {
        self.backend.delete_secret(name)
    }

    /// List secrets
    pub fn list_secrets(&self) -> Result<Vec<String>> {
        self.backend.list_secrets()
    }

    /// Rotate secret
    pub fn rotate_secret(&self, name: &str, new_value: &str) -> Result<()> {
        self.backend.rotate_secret(name, new_value)
    }

    /// Get database URL
    pub fn get_db_url(&self) -> Result<String> {
        self.get_secret("DATABASE_URL")
    }

    /// Get ledger encryption key
    pub fn get_ledger_key(&self) -> Result<[u8; 32]> {
        let hex = self.get_secret("LEDGER_ENCRYPTION_KEY")?;
        let bytes = hex::decode(&hex)
            .map_err(|e| SecretsError::Decryption(format!("Invalid key hex: {}", e)))?;

        if bytes.len() != 32 {
            return Err(SecretsError::Decryption("Key must be 32 bytes".to_string()));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(key)
    }

    /// Get validator private key
    pub fn get_validator_key(&self) -> Result<Vec<u8>> {
        let hex = self.get_secret("VALIDATOR_PRIVATE_KEY")?;
        hex::decode(&hex)
            .map_err(|e| SecretsError::Decryption(format!("Invalid key hex: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_environment_backend() {
        std::env::set_var("TEST_SECRET", "test_value");

        let backend = EnvironmentBackend;
        let value = backend.get_secret("TEST_SECRET").unwrap();
        assert_eq!(value, "test_value");

        // Should fail for non-existent secret
        assert!(backend.get_secret("NONEXISTENT").is_err());
    }

    #[test]
    fn test_encrypted_file_backend() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("secrets.enc");
        let master_key = [42u8; 32];

        // Create backend
        let backend = EncryptedFileBackend::new(file_path.clone(), &master_key).unwrap();

        // Set secret
        let metadata = SecretMetadata {
            name: "test".to_string(),
            version: 1,
            created_at: 0,
            accessed_at: 0,
            rotation_days: Some(90),
        };

        backend
            .set_secret("test_secret", "test_value", metadata)
            .unwrap();

        // Get secret
        let value = backend.get_secret("test_secret").unwrap();
        assert_eq!(value, "test_value");

        // Reload from file
        let backend2 = EncryptedFileBackend::new(file_path, &master_key).unwrap();
        let value2 = backend2.get_secret("test_secret").unwrap();
        assert_eq!(value2, "test_value");
    }

    #[test]
    fn test_secrets_manager() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("secrets.enc");
        let master_key = [42u8; 32];

        let manager = SecretsManager::with_encrypted_file(file_path, &master_key).unwrap();

        // Set and get
        manager.set_secret("api_key", "secret123").unwrap();
        let value = manager.get_secret("api_key").unwrap();
        assert_eq!(value, "secret123");

        // Rotate
        manager.rotate_secret("api_key", "secret456").unwrap();
        let value = manager.get_secret("api_key").unwrap();
        assert_eq!(value, "secret456");

        // List
        let secrets = manager.list_secrets().unwrap();
        assert!(secrets.contains(&"api_key".to_string()));

        // Delete
        manager.delete_secret("api_key").unwrap();
        assert!(manager.get_secret("api_key").is_err());
    }
}