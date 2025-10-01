//! ABCI Application implementation
//!
//! Implements the Application BlockChain Interface for CometBFT.

use crate::{state::*, Error, Result};
use ledger_core::Ledger;
use std::sync::Arc;
use tendermint_abci::{
    Application, CheckTxRequest, CheckTxResponse, CommitRequest, CommitResponse,
    DeliverTxRequest, DeliverTxResponse, InfoRequest, InfoResponse, InitChainRequest,
    InitChainResponse, QueryRequest, QueryResponse,
};
use tracing::{info, warn};
use uuid::Uuid;

/// Ledger ABCI Application
pub struct LedgerApp {
    /// Ledger core
    ledger: Arc<Ledger>,

    /// Consensus state
    state: Arc<ConsensusState>,
}

impl LedgerApp {
    /// Create new ABCI application
    pub async fn new(ledger: Arc<Ledger>) -> Result<Self> {
        let state = Arc::new(ConsensusState::new());

        Ok(Self { ledger, state })
    }

    /// Get consensus state
    pub fn state(&self) -> Arc<ConsensusState> {
        self.state.clone()
    }
}

impl Application for LedgerApp {
    /// Info - Return information about application state
    fn info(&self, _request: InfoRequest) -> InfoResponse {
        let height = tokio::runtime::Handle::current()
            .block_on(self.state.height());

        let app_hash = tokio::runtime::Handle::current()
            .block_on(self.state.app_hash());

        info!("Info request: height={}, app_hash={:?}", height, app_hash);

        InfoResponse {
            data: "DelTran Ledger".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            app_version: 1,
            last_block_height: height as i64,
            last_block_app_hash: app_hash.into(),
        }
    }

    /// InitChain - Initialize blockchain on genesis
    fn init_chain(&self, request: InitChainRequest) -> InitChainResponse {
        info!(
            "InitChain: chain_id={}, validators={}",
            request.chain_id,
            request.validators.len()
        );

        // Initialize genesis state
        tokio::runtime::Handle::current().block_on(async {
            self.state.set_app_hash(vec![0u8; 32]).await;
        });

        InitChainResponse {
            consensus_params: request.consensus_params,
            validators: request.validators,
            app_hash: vec![0u8; 32].into(),
        }
    }

    /// CheckTx - Validate transaction before adding to mempool
    fn check_tx(&self, request: CheckTxRequest) -> CheckTxResponse {
        // Deserialize transaction
        let tx = match Transaction::from_bytes(&request.tx) {
            Ok(tx) => tx,
            Err(e) => {
                warn!("Failed to deserialize transaction: {}", e);
                return CheckTxResponse {
                    code: 1,
                    data: vec![].into(),
                    log: format!("Invalid transaction: {}", e),
                    info: String::new(),
                    gas_wanted: 0,
                    gas_used: 0,
                    events: vec![],
                    codespace: String::new(),
                    sender: String::new(),
                    priority: 0,
                    mempool_error: String::new(),
                };
            }
        };

        // Basic validation
        if tx.event.amount <= rust_decimal::Decimal::ZERO {
            return CheckTxResponse {
                code: 2,
                log: "Amount must be positive".to_string(),
                ..Default::default()
            };
        }

        info!("CheckTx passed: tx_id={}", tx.tx_id);

        // Add to mempool
        tokio::runtime::Handle::current().block_on(async {
            self.state.add_to_mempool(tx).await;
        });

        CheckTxResponse {
            code: 0, // Success
            data: vec![].into(),
            log: "Transaction valid".to_string(),
            info: String::new(),
            gas_wanted: 1,
            gas_used: 1,
            events: vec![],
            codespace: String::new(),
            sender: String::new(),
            priority: 0,
            mempool_error: String::new(),
        }
    }

    /// DeliverTx - Execute transaction in block
    fn deliver_tx(&self, request: DeliverTxRequest) -> DeliverTxResponse {
        // Deserialize transaction
        let tx = match Transaction::from_bytes(&request.tx) {
            Ok(tx) => tx,
            Err(e) => {
                warn!("Failed to deserialize transaction: {}", e);
                return DeliverTxResponse {
                    code: 1,
                    log: format!("Invalid transaction: {}", e),
                    ..Default::default()
                };
            }
        };

        // Append event to ledger
        let result = tokio::runtime::Handle::current().block_on(async {
            self.ledger.append_event(tx.event.clone()).await
        });

        match result {
            Ok(event_id) => {
                info!("DeliverTx success: event_id={}", event_id);

                DeliverTxResponse {
                    code: 0,
                    data: event_id.as_bytes().to_vec().into(),
                    log: "Event appended successfully".to_string(),
                    info: String::new(),
                    gas_wanted: 1,
                    gas_used: 1,
                    events: vec![],
                    codespace: String::new(),
                }
            }
            Err(e) => {
                warn!("DeliverTx failed: {}", e);

                DeliverTxResponse {
                    code: 1,
                    log: format!("Failed to append event: {}", e),
                    ..Default::default()
                }
            }
        }
    }

    /// Commit - Finalize block and compute app hash
    fn commit(&self, _request: CommitRequest) -> CommitResponse {
        let result = tokio::runtime::Handle::current().block_on(async {
            // Get mempool transactions
            let txs = self.state.get_mempool().await;
            let event_ids: Vec<Uuid> = txs.iter().map(|tx| tx.event.event_id).collect();

            if event_ids.is_empty() {
                // Empty block
                return Ok::<_, Error>((vec![0u8; 32], None));
            }

            // Finalize block in ledger
            let block = self.ledger.finalize_block(event_ids).await?;

            // App hash = Merkle root
            let app_hash = block.merkle_root.to_vec();

            // Update state
            self.state.set_app_hash(app_hash.clone()).await;
            self.state.increment_height().await;
            self.state.set_last_block_id(block.block_id).await;
            self.state.clear_mempool().await;

            info!(
                "Committed block: height={}, block_id={}, event_count={}",
                block.block_height, block.block_id, block.event_count
            );

            Ok((app_hash, Some(block.block_id)))
        });

        match result {
            Ok((app_hash, _block_id)) => CommitResponse {
                data: app_hash.into(),
                retain_height: 0,
            },
            Err(e) => {
                warn!("Commit failed: {}", e);
                CommitResponse {
                    data: vec![0u8; 32].into(),
                    retain_height: 0,
                }
            }
        }
    }

    /// Query - Read-only query of application state
    fn query(&self, request: QueryRequest) -> QueryResponse {
        let path = String::from_utf8_lossy(&request.path);

        info!("Query: path={}", path);

        // Parse path: /payment/{payment_id}
        if path.starts_with("/payment/") {
            let payment_id_str = path.trim_start_matches("/payment/");
            let payment_id = match Uuid::parse_str(payment_id_str) {
                Ok(id) => id,
                Err(_) => {
                    return QueryResponse {
                        code: 1,
                        log: "Invalid payment ID".to_string(),
                        ..Default::default()
                    };
                }
            };

            // Query ledger
            let result = tokio::runtime::Handle::current().block_on(async {
                self.ledger.get_payment_state(payment_id).await
            });

            match result {
                Ok(state) => {
                    let data = bincode::serialize(&state).unwrap_or_default();

                    QueryResponse {
                        code: 0,
                        log: "Payment state found".to_string(),
                        info: String::new(),
                        index: 0,
                        key: request.data,
                        value: data.into(),
                        proof_ops: None,
                        height: 0,
                        codespace: String::new(),
                    }
                }
                Err(e) => QueryResponse {
                    code: 1,
                    log: format!("Payment not found: {}", e),
                    ..Default::default()
                },
            }
        } else {
            QueryResponse {
                code: 1,
                log: "Unknown query path".to_string(),
                ..Default::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ledger_core::Config as LedgerConfig;

    async fn create_test_app() -> LedgerApp {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = LedgerConfig::default();
        config.data_dir = temp_dir.path().to_path_buf();
        config.batching.enabled = false;

        let ledger = Arc::new(Ledger::open(config).await.unwrap());
        LedgerApp::new(ledger).await.unwrap()
    }

    #[tokio::test]
    async fn test_abci_info() {
        let app = create_test_app().await;

        let request = InfoRequest {
            version: "0.34".to_string(),
            block_version: 11,
            p2p_version: 8,
            abci_version: "0.17.0".to_string(),
        };

        let response = app.info(request);
        assert_eq!(response.last_block_height, 0);
    }

    #[tokio::test]
    async fn test_abci_init_chain() {
        let app = create_test_app().await;

        let request = InitChainRequest {
            time: None,
            chain_id: "test-chain".to_string(),
            consensus_params: None,
            validators: vec![],
            app_state_bytes: vec![].into(),
            initial_height: 0,
        };

        let response = app.init_chain(request);
        assert_eq!(response.app_hash.len(), 32);
    }
}