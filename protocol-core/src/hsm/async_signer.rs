//! Async HSM Signing Queue
//!
//! Prevents HSM from blocking transaction pipeline

use super::pkcs11::Pkcs11Hsm;
use crate::{Error, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};

/// Signing request
struct SignRequest {
    data: Vec<u8>,
    response_tx: oneshot::Sender<Result<Vec<u8>>>,
}

/// Async HSM signer with queue
pub struct AsyncHsmSigner {
    request_tx: mpsc::Sender<SignRequest>,
}

impl AsyncHsmSigner {
    /// Create new async signer
    pub fn new(hsm: Arc<Pkcs11Hsm>, queue_size: usize, worker_count: usize) -> Self {
        let (request_tx, request_rx) = mpsc::channel(queue_size);

        // Spawn worker pool
        for worker_id in 0..worker_count {
            let hsm = Arc::clone(&hsm);
            let mut rx = request_rx.clone();

            tokio::spawn(async move {
                info!("HSM signing worker {} started", worker_id);

                while let Some(req) = rx.recv().await {
                    let signature = hsm.sign(&req.data).await;

                    if let Err(e) = req.response_tx.send(signature) {
                        error!("Failed to send signature response: {:?}", e);
                    }
                }

                warn!("HSM signing worker {} stopped", worker_id);
            });
        }

        Self { request_tx }
    }

    /// Sign data asynchronously
    pub async fn sign(&self, data: Vec<u8>) -> Result<Vec<u8>> {
        let (response_tx, response_rx) = oneshot::channel();

        let req = SignRequest { data, response_tx };

        self.request_tx
            .send(req)
            .await
            .map_err(|e| Error::Hsm(format!("Queue full: {}", e)))?;

        response_rx
            .await
            .map_err(|e| Error::Hsm(format!("Worker died: {}", e)))?
    }

    /// Get queue depth (for monitoring)
    pub fn queue_depth(&self) -> usize {
        self.request_tx.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_async_signing() {
        let hsm = Arc::new(Pkcs11Hsm::new(
            0,
            "test-key".to_string(),
            "1234".to_string(),
            10,
        ));
        hsm.init().await.expect("Init failed");

        let signer = AsyncHsmSigner::new(hsm, 100, 4);

        // Sign multiple messages concurrently
        let mut handles = vec![];
        for i in 0..20 {
            let signer = signer.clone();
            let handle = tokio::spawn(async move {
                let data = format!("checkpoint-{}", i).into_bytes();
                signer.sign(data).await.expect("Sign failed")
            });
            handles.push(handle);
        }

        // Wait for all signatures
        for handle in handles {
            let sig = handle.await.expect("Task panicked");
            assert_eq!(sig.len(), 64);
        }
    }
}

// Need to derive Clone for AsyncHsmSigner
impl Clone for AsyncHsmSigner {
    fn clone(&self) -> Self {
        Self {
            request_tx: self.request_tx.clone(),
        }
    }
}
