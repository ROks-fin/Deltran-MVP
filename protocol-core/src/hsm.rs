//! HSM (Hardware Security Module) interface
//!
//! MVP: Mock implementation with Ed25519
//! Production: PKCS#11 interface to AWS CloudHSM
//!
//! The interface is designed to be swappable:
//! - Development: HsmMock (software Ed25519)
//! - Production: HsmPkcs11 (real HSM via PKCS#11)

use crate::{Error, Result};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// HSM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsmConfig {
    /// HSM type
    pub hsm_type: HsmType,
    /// Key ID (for CloudHSM) or path (for mock)
    pub key_id: String,
    /// Key epoch (for rotation)
    pub key_epoch: String,
    /// Algorithm
    pub algorithm: HsmAlgorithm,
}

/// HSM type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HsmType {
    /// Mock (software signing, for development)
    Mock,
    /// PKCS#11 (real HSM)
    Pkcs11 {
        /// Library path (e.g., /opt/cloudhsm/lib/libcloudhsm_pkcs11.so)
        library_path: String,
        /// Slot ID
        slot_id: u64,
        /// PIN (should come from env/vault)
        pin: String,
    },
}

/// HSM algorithm
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HsmAlgorithm {
    /// Ed25519 (default for MVP)
    Ed25519,
    /// ECDSA P-256 (for production)
    EcdsaP256,
    /// RSA-PSS 4096 (for legacy compatibility)
    RsaPss4096,
}

/// HSM trait (interface)
pub trait Hsm: Send + Sync {
    /// Sign data
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// Get public key
    fn public_key(&self) -> Result<Vec<u8>>;

    /// Get key ID
    fn key_id(&self) -> &str;

    /// Get key epoch
    fn key_epoch(&self) -> &str;

    /// Get algorithm
    fn algorithm(&self) -> HsmAlgorithm;
}

// =========================================================================
// MOCK HSM (for development)
// =========================================================================

/// Mock HSM (software Ed25519 signing)
pub struct HsmMock {
    signing_key: SigningKey,
    key_id: String,
    key_epoch: String,
}

impl HsmMock {
    /// Create new mock HSM with random key
    pub fn new(key_id: String, key_epoch: String) -> Self {
        // Generate random 32-byte seed
        let mut seed = [0u8; 32];
        rand::RngCore::fill_bytes(&mut OsRng, &mut seed);
        let signing_key = SigningKey::from_bytes(&seed);

        Self {
            signing_key,
            key_id,
            key_epoch,
        }
    }

    /// Create from existing seed (for testing determinism)
    pub fn from_seed(seed: [u8; 32], key_id: String, key_epoch: String) -> Self {
        let signing_key = SigningKey::from_bytes(&seed);

        Self {
            signing_key,
            key_id,
            key_epoch,
        }
    }
}

impl Hsm for HsmMock {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        let signature: Signature = self.signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }

    fn public_key(&self) -> Result<Vec<u8>> {
        let verifying_key: VerifyingKey = self.signing_key.verifying_key();
        Ok(verifying_key.to_bytes().to_vec())
    }

    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn key_epoch(&self) -> &str {
        &self.key_epoch
    }

    fn algorithm(&self) -> HsmAlgorithm {
        HsmAlgorithm::Ed25519
    }
}

// =========================================================================
// PKCS#11 HSM (for production)
// =========================================================================

/// PKCS#11 HSM (real CloudHSM)
///
/// NOTE: This is a skeleton for production. Full implementation requires:
/// 1. cryptoki crate for PKCS#11 bindings
/// 2. CloudHSM SDK installation
/// 3. Proper session management and error handling
#[allow(dead_code)]
pub struct HsmPkcs11 {
    key_id: String,
    key_epoch: String,
    // pkcs11_context: Pkcs11,  // TODO: Add when implementing real HSM
    // session: Session,
}

#[allow(dead_code)]
impl HsmPkcs11 {
    /// Initialize PKCS#11 HSM
    pub fn new(
        _library_path: &str,
        _slot_id: u64,
        _pin: &str,
        key_id: String,
        key_epoch: String,
    ) -> Result<Self> {
        // TODO: Real PKCS#11 initialization
        //
        // 1. Load library: Pkcs11::new(library_path)
        // 2. Initialize: pkcs11.initialize()
        // 3. Open session: pkcs11.open_session(slot_id)
        // 4. Login: session.login(UserType::User, Some(pin))
        // 5. Find signing key by label/ID

        Ok(Self {
            key_id,
            key_epoch,
        })
    }
}

impl Hsm for HsmPkcs11 {
    fn sign(&self, _data: &[u8]) -> Result<Vec<u8>> {
        // TODO: Real HSM signing
        //
        // 1. Find key handle: session.find_objects(&[Attribute::Label(key_id)])
        // 2. Sign: session.sign(&Mechanism::Ecdsa, key_handle, data)

        Err(Error::HsmError(
            "PKCS#11 HSM not implemented in MVP".into(),
        ))
    }

    fn public_key(&self) -> Result<Vec<u8>> {
        Err(Error::HsmError(
            "PKCS#11 HSM not implemented in MVP".into(),
        ))
    }

    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn key_epoch(&self) -> &str {
        &self.key_epoch
    }

    fn algorithm(&self) -> HsmAlgorithm {
        HsmAlgorithm::EcdsaP256 // CloudHSM typically uses ECDSA
    }
}

// =========================================================================
// HSM FACTORY
// =========================================================================

/// Create HSM from config
pub fn create_hsm(config: &HsmConfig) -> Result<Box<dyn Hsm>> {
    match &config.hsm_type {
        HsmType::Mock => {
            let hsm = HsmMock::new(config.key_id.clone(), config.key_epoch.clone());
            Ok(Box::new(hsm))
        }
        HsmType::Pkcs11 {
            library_path,
            slot_id,
            pin,
        } => {
            let hsm = HsmPkcs11::new(
                library_path,
                *slot_id,
                pin,
                config.key_id.clone(),
                config.key_epoch.clone(),
            )?;
            Ok(Box::new(hsm))
        }
    }
}

// =========================================================================
// SIGNATURE VERIFICATION
// =========================================================================

/// Verify HSM signature (Ed25519)
pub fn verify_hsm_signature(
    data: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<()> {
    // Parse public key
    let pk_bytes: [u8; 32] = public_key
        .try_into()
        .map_err(|_| Error::SignatureInvalid("Invalid public key length".into()))?;
    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|e| Error::SignatureInvalid(e.to_string()))?;

    // Parse signature
    let sig_bytes: [u8; 64] = signature
        .try_into()
        .map_err(|_| Error::SignatureInvalid("Invalid signature length".into()))?;
    let signature = Signature::from_bytes(&sig_bytes);

    // Verify
    use ed25519_dalek::Verifier;
    verifying_key
        .verify(data, &signature)
        .map_err(|e| Error::SignatureInvalid(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hsm_mock_sign_verify() {
        let hsm = HsmMock::new("test-key-1".into(), "2025-Q1".into());

        let data = b"checkpoint data";
        let signature = hsm.sign(data).unwrap();
        let public_key = hsm.public_key().unwrap();

        // Verify signature
        assert!(verify_hsm_signature(data, &signature, &public_key).is_ok());

        // Tampered data should fail
        let bad_data = b"tampered data";
        assert!(verify_hsm_signature(bad_data, &signature, &public_key).is_err());
    }

    #[test]
    fn test_hsm_mock_determinism() {
        let seed = [42u8; 32];
        let hsm1 = HsmMock::from_seed(seed, "key1".into(), "2025-Q1".into());
        let hsm2 = HsmMock::from_seed(seed, "key1".into(), "2025-Q1".into());

        let data = b"test";
        let sig1 = hsm1.sign(data).unwrap();
        let sig2 = hsm2.sign(data).unwrap();

        // Same seed = same signature
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_hsm_factory() {
        let config = HsmConfig {
            hsm_type: HsmType::Mock,
            key_id: "coordinator-key".into(),
            key_epoch: "2025-Q1".into(),
            algorithm: HsmAlgorithm::Ed25519,
        };

        let hsm = create_hsm(&config).unwrap();

        assert_eq!(hsm.key_id(), "coordinator-key");
        assert_eq!(hsm.key_epoch(), "2025-Q1");

        let data = b"test";
        let sig = hsm.sign(data).unwrap();
        assert_eq!(sig.len(), 64); // Ed25519 signature size
    }
}