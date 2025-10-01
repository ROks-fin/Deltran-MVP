//! TLS/mTLS Configuration for Service Communication
//!
//! Provides mutual TLS authentication between services:
//! - Gateway <-> Ledger (gRPC)
//! - Gateway <-> Risk Engine (gRPC)
//! - Gateway <-> Settlement (gRPC)
//! - Consensus nodes <-> Consensus nodes (P2P)

use rustls::{Certificate, PrivateKey, RootCertStore, ServerConfig, ClientConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// TLS configuration errors
#[derive(Error, Debug)]
pub enum TlsError {
    #[error("Failed to read certificate: {0}")]
    CertificateRead(#[from] std::io::Error),

    #[error("Invalid certificate format: {0}")]
    InvalidCertificate(String),

    #[error("Invalid private key format")]
    InvalidPrivateKey,

    #[error("TLS configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, TlsError>;

/// TLS certificate configuration
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to server certificate (PEM)
    pub cert_path: String,

    /// Path to server private key (PEM)
    pub key_path: String,

    /// Path to CA certificate for client verification (PEM)
    pub ca_cert_path: String,

    /// Require client certificates (mTLS)
    pub require_client_cert: bool,

    /// Allowed cipher suites
    pub cipher_suites: Vec<String>,

    /// Minimum TLS version
    pub min_tls_version: TlsVersion,
}

/// TLS version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: "./certs/server.crt".to_string(),
            key_path: "./certs/server.key".to_string(),
            ca_cert_path: "./certs/ca.crt".to_string(),
            require_client_cert: true,
            cipher_suites: vec![
                // TLS 1.3 (recommended)
                "TLS13_AES_256_GCM_SHA384".to_string(),
                "TLS13_AES_128_GCM_SHA256".to_string(),
                "TLS13_CHACHA20_POLY1305_SHA256".to_string(),
                // TLS 1.2 (fallback)
                "TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384".to_string(),
                "TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384".to_string(),
            ],
            min_tls_version: TlsVersion::Tls12,
        }
    }
}

impl TlsConfig {
    /// Load from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        if let Ok(cert_path) = std::env::var("TLS_CERT_PATH") {
            config.cert_path = cert_path;
        }

        if let Ok(key_path) = std::env::var("TLS_KEY_PATH") {
            config.key_path = key_path;
        }

        if let Ok(ca_cert_path) = std::env::var("TLS_CA_CERT_PATH") {
            config.ca_cert_path = ca_cert_path;
        }

        if let Ok(require_client) = std::env::var("TLS_REQUIRE_CLIENT_CERT") {
            config.require_client_cert = require_client.parse().unwrap_or(true);
        }

        Ok(config)
    }

    /// Build server TLS configuration
    pub fn build_server_config(&self) -> Result<Arc<ServerConfig>> {
        // Load server certificate
        let cert_file = File::open(&self.cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain: Vec<Certificate> = certs(&mut cert_reader)
            .map_err(|e| TlsError::InvalidCertificate(e.to_string()))?
            .into_iter()
            .map(Certificate)
            .collect();

        if cert_chain.is_empty() {
            return Err(TlsError::InvalidCertificate(
                "No certificates found".to_string(),
            ));
        }

        // Load private key
        let key_file = File::open(&self.key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = pkcs8_private_keys(&mut key_reader)
            .map_err(|_| TlsError::InvalidPrivateKey)?;

        if keys.is_empty() {
            return Err(TlsError::InvalidPrivateKey);
        }

        let private_key = PrivateKey(keys.remove(0));

        // Load CA certificate for client verification
        let ca_cert_file = File::open(&self.ca_cert_path)?;
        let mut ca_cert_reader = BufReader::new(ca_cert_file);
        let ca_certs = certs(&mut ca_cert_reader)
            .map_err(|e| TlsError::InvalidCertificate(e.to_string()))?;

        let mut root_store = RootCertStore::empty();
        for cert in ca_certs {
            root_store
                .add(&Certificate(cert))
                .map_err(|e| TlsError::Config(format!("Failed to add CA cert: {}", e)))?;
        }

        // Build server config
        let mut config = ServerConfig::builder()
            .with_safe_defaults()
            .with_client_cert_verifier(Arc::new(
                rustls::server::AllowAnyAuthenticatedClient::new(root_store),
            ))
            .with_single_cert(cert_chain, private_key)
            .map_err(|e| TlsError::Config(e.to_string()))?;

        // Configure ALPN for gRPC
        config.alpn_protocols = vec![b"h2".to_vec()];

        Ok(Arc::new(config))
    }

    /// Build client TLS configuration
    pub fn build_client_config(&self) -> Result<Arc<ClientConfig>> {
        // Load CA certificate
        let ca_cert_file = File::open(&self.ca_cert_path)?;
        let mut ca_cert_reader = BufReader::new(ca_cert_file);
        let ca_certs = certs(&mut ca_cert_reader)
            .map_err(|e| TlsError::InvalidCertificate(e.to_string()))?;

        let mut root_store = RootCertStore::empty();
        for cert in ca_certs {
            root_store
                .add(&Certificate(cert))
                .map_err(|e| TlsError::Config(format!("Failed to add CA cert: {}", e)))?;
        }

        // Load client certificate and key
        let cert_file = File::open(&self.cert_path)?;
        let mut cert_reader = BufReader::new(cert_file);
        let cert_chain: Vec<Certificate> = certs(&mut cert_reader)
            .map_err(|e| TlsError::InvalidCertificate(e.to_string()))?
            .into_iter()
            .map(Certificate)
            .collect();

        let key_file = File::open(&self.key_path)?;
        let mut key_reader = BufReader::new(key_file);
        let mut keys = pkcs8_private_keys(&mut key_reader)
            .map_err(|_| TlsError::InvalidPrivateKey)?;

        if keys.is_empty() {
            return Err(TlsError::InvalidPrivateKey);
        }

        let private_key = PrivateKey(keys.remove(0));

        // Build client config
        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_client_auth_cert(cert_chain, private_key)
            .map_err(|e| TlsError::Config(e.to_string()))?;

        Ok(Arc::new(config))
    }
}

/// Certificate generator for development/testing
pub struct CertificateGenerator;

impl CertificateGenerator {
    /// Generate self-signed CA certificate
    pub fn generate_ca(
        output_dir: impl AsRef<Path>,
        common_name: &str,
    ) -> Result<()> {
        let output_dir = output_dir.as_ref();
        std::fs::create_dir_all(output_dir)?;

        // Generate CA key pair
        let ca_key = rcgen::KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)
            .map_err(|e| TlsError::Config(e.to_string()))?;

        // CA certificate parameters
        let mut ca_params = rcgen::CertificateParams::new(vec![common_name.to_string()]);
        ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        ca_params.key_usages = vec![
            rcgen::KeyUsagePurpose::KeyCertSign,
            rcgen::KeyUsagePurpose::CrlSign,
        ];
        ca_params.not_before = time::OffsetDateTime::now_utc();
        ca_params.not_after = time::OffsetDateTime::now_utc() + time::Duration::days(3650); // 10 years

        // Generate CA certificate
        let ca_cert = rcgen::Certificate::from_params(ca_params)
            .map_err(|e| TlsError::Config(e.to_string()))?;

        // Save CA certificate
        let ca_cert_pem = ca_cert
            .serialize_pem()
            .map_err(|e| TlsError::Config(e.to_string()))?;
        std::fs::write(output_dir.join("ca.crt"), ca_cert_pem)?;

        // Save CA private key
        std::fs::write(output_dir.join("ca.key"), ca_key.serialize_pem())?;

        Ok(())
    }

    /// Generate service certificate signed by CA
    pub fn generate_service_cert(
        output_dir: impl AsRef<Path>,
        ca_cert_path: impl AsRef<Path>,
        ca_key_path: impl AsRef<Path>,
        service_name: &str,
        dns_names: Vec<String>,
    ) -> Result<()> {
        let output_dir = output_dir.as_ref();
        std::fs::create_dir_all(output_dir)?;

        // Load CA certificate
        let ca_cert_pem = std::fs::read_to_string(ca_cert_path)?;
        let ca_key_pem = std::fs::read_to_string(ca_key_path)?;

        let ca_key = rcgen::KeyPair::from_pem(&ca_key_pem)
            .map_err(|e| TlsError::Config(e.to_string()))?;

        let ca_params = rcgen::CertificateParams::from_ca_cert_pem(&ca_cert_pem, ca_key)
            .map_err(|e| TlsError::Config(e.to_string()))?;

        let ca_cert = rcgen::Certificate::from_params(ca_params)
            .map_err(|e| TlsError::Config(e.to_string()))?;

        // Generate service key pair
        let service_key = rcgen::KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)
            .map_err(|e| TlsError::Config(e.to_string()))?;

        // Service certificate parameters
        let mut service_params = rcgen::CertificateParams::new(dns_names);
        service_params.key_usages = vec![
            rcgen::KeyUsagePurpose::DigitalSignature,
            rcgen::KeyUsagePurpose::KeyEncipherment,
        ];
        service_params.extended_key_usages = vec![
            rcgen::ExtendedKeyUsagePurpose::ServerAuth,
            rcgen::ExtendedKeyUsagePurpose::ClientAuth,
        ];
        service_params.not_before = time::OffsetDateTime::now_utc();
        service_params.not_after = time::OffsetDateTime::now_utc() + time::Duration::days(365); // 1 year

        // Generate service certificate
        let service_cert = rcgen::Certificate::from_params(service_params)
            .map_err(|e| TlsError::Config(e.to_string()))?;

        // Sign with CA
        let service_cert_pem = service_cert
            .serialize_pem_with_signer(&ca_cert)
            .map_err(|e| TlsError::Config(e.to_string()))?;

        // Save service certificate
        std::fs::write(
            output_dir.join(format!("{}.crt", service_name)),
            service_cert_pem,
        )?;

        // Save service private key
        std::fs::write(
            output_dir.join(format!("{}.key", service_name)),
            service_key.serialize_pem(),
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_ca() {
        let temp_dir = tempdir().unwrap();
        let result = CertificateGenerator::generate_ca(temp_dir.path(), "DelTran CA");
        assert!(result.is_ok());

        assert!(temp_dir.path().join("ca.crt").exists());
        assert!(temp_dir.path().join("ca.key").exists());
    }

    #[test]
    fn test_generate_service_cert() {
        let temp_dir = tempdir().unwrap();

        // Generate CA
        CertificateGenerator::generate_ca(temp_dir.path(), "DelTran CA").unwrap();

        // Generate service cert
        let result = CertificateGenerator::generate_service_cert(
            temp_dir.path(),
            temp_dir.path().join("ca.crt"),
            temp_dir.path().join("ca.key"),
            "gateway",
            vec!["gateway.deltran.local".to_string(), "localhost".to_string()],
        );
        assert!(result.is_ok());

        assert!(temp_dir.path().join("gateway.crt").exists());
        assert!(temp_dir.path().join("gateway.key").exists());
    }

    #[test]
    fn test_load_tls_config() {
        let temp_dir = tempdir().unwrap();

        // Generate CA and service cert
        CertificateGenerator::generate_ca(temp_dir.path(), "DelTran CA").unwrap();
        CertificateGenerator::generate_service_cert(
            temp_dir.path(),
            temp_dir.path().join("ca.crt"),
            temp_dir.path().join("ca.key"),
            "test",
            vec!["localhost".to_string()],
        )
        .unwrap();

        // Create TLS config
        let config = TlsConfig {
            cert_path: temp_dir.path().join("test.crt").to_string_lossy().to_string(),
            key_path: temp_dir.path().join("test.key").to_string_lossy().to_string(),
            ca_cert_path: temp_dir.path().join("ca.crt").to_string_lossy().to_string(),
            require_client_cert: true,
            cipher_suites: vec![],
            min_tls_version: TlsVersion::Tls12,
        };

        // Build server config
        let server_config = config.build_server_config();
        assert!(server_config.is_ok());

        // Build client config
        let client_config = config.build_client_config();
        assert!(client_config.is_ok());
    }
}