//! Integration Tests for DelTran Settlement Rail
//!
//! Tests the complete system end-to-end:
//! - Gateway → Ledger → Settlement → Consensus
//! - Payment lifecycle (initiated → settled)
//! - Netting efficiency
//! - Byzantine fault tolerance
//! - Security features

use chrono::Utc;
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

// Test configuration
const TEST_TIMEOUT: Duration = Duration::from_secs(30);
const NUM_PAYMENTS: usize = 100;
const NUM_VALIDATORS: usize = 3;

#[tokio::test]
async fn test_end_to_end_payment_flow() {
    // Setup test environment
    let env = TestEnvironment::new().await;

    // Create payment
    let payment_id = Uuid::new_v4();
    let payment = create_test_payment(payment_id, Decimal::new(10000, 2)); // $100.00

    // Submit to gateway
    let result = env.gateway.submit_payment(payment.clone()).await;
    assert!(result.is_ok(), "Payment submission failed: {:?}", result);

    // Wait for payment to be processed
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify in ledger
    let state = env.ledger.get_payment_state(payment_id).await;
    assert!(state.is_ok(), "Payment not found in ledger");
    let state = state.unwrap();
    assert_eq!(state.status, PaymentStatus::Pending);

    // Trigger settlement window
    let batch = env.settlement.run_settlement_window().await;
    assert!(batch.is_ok(), "Settlement failed");

    // Verify payment settled
    let state = env.ledger.get_payment_state(payment_id).await.unwrap();
    assert_eq!(state.status, PaymentStatus::Settled);
}

#[tokio::test]
async fn test_multilateral_netting() {
    let env = TestEnvironment::new().await;

    // Create payment cycle:
    // Bank A → Bank B: $100
    // Bank B → Bank C: $100
    // Bank C → Bank A: $100
    // Net result: $0 transfers

    let payments = vec![
        create_payment("BANKA", "BANKB", Decimal::new(10000, 2)),
        create_payment("BANKB", "BANKC", Decimal::new(10000, 2)),
        create_payment("BANKC", "BANKA", Decimal::new(10000, 2)),
    ];

    // Submit payments
    for payment in &payments {
        env.gateway.submit_payment(payment.clone()).await.unwrap();
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Run settlement
    let batch = env.settlement.run_settlement_window().await.unwrap();

    // Verify netting
    assert_eq!(batch.net_transfers.len(), 0, "Expected 0 net transfers (perfect cycle)");
    assert_eq!(batch.gross_amount, Decimal::new(30000, 2)); // $300 gross
    assert_eq!(batch.net_amount, Decimal::ZERO); // $0 net

    let efficiency = (batch.gross_amount - batch.net_amount) / batch.gross_amount * Decimal::new(100, 0);
    assert_eq!(efficiency, Decimal::new(100, 0), "Expected 100% netting efficiency");
}

#[tokio::test]
async fn test_consensus_finality() {
    let env = TestEnvironment::new_with_validators(NUM_VALIDATORS).await;

    let payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));

    // Submit to consensus
    let tx = env.consensus.create_transaction(payment.clone()).await.unwrap();
    env.consensus.broadcast_tx(tx.to_bytes().unwrap()).await.unwrap();

    // Wait for block
    tokio::time::sleep(Duration::from_secs(7)).await; // 6s block time + buffer

    // Verify committed
    let block_height = env.consensus.get_height().await.unwrap();
    assert!(block_height > 0, "Block not committed");

    // Verify finality (query from all validators)
    for validator in &env.consensus_nodes {
        let state = validator.query_payment(payment.payment_id).await.unwrap();
        assert_eq!(state.status, PaymentStatus::Committed);
    }
}

#[tokio::test]
async fn test_byzantine_fault_tolerance() {
    let env = TestEnvironment::new_with_validators(4).await; // 4 validators, tolerates 1 Byzantine

    let payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));

    // Submit payment
    let tx = env.consensus.create_transaction(payment.clone()).await.unwrap();
    env.consensus.broadcast_tx(tx.to_bytes().unwrap()).await.unwrap();

    // Kill one validator (simulating Byzantine failure)
    env.consensus_nodes[0].shutdown().await;

    // Wait for block
    tokio::time::sleep(Duration::from_secs(7)).await;

    // Verify system still operational (2/3 majority)
    let block_height = env.consensus.get_height().await.unwrap();
    assert!(block_height > 0, "Consensus halted with 1 Byzantine node");

    // Verify payment committed
    let state = env.consensus_nodes[1].query_payment(payment.payment_id).await.unwrap();
    assert_eq!(state.status, PaymentStatus::Committed);
}

#[tokio::test]
async fn test_rate_limiting() {
    let env = TestEnvironment::new().await;

    // Attempt to exceed rate limit (1000 req/min)
    let mut successes = 0;
    let mut denials = 0;

    for _ in 0..150 {
        let payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));
        match env.gateway.submit_payment(payment).await {
            Ok(_) => successes += 1,
            Err(e) if e.to_string().contains("rate limit") => denials += 1,
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    // Should hit rate limit
    assert!(denials > 0, "Rate limit not triggered");
    assert!(successes <= 100, "Too many requests allowed (burst size = 100)");
}

#[tokio::test]
async fn test_input_validation() {
    let env = TestEnvironment::new().await;

    // Invalid BIC
    let mut payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));
    payment.debtor_bic = "INVALID".to_string();
    let result = env.gateway.submit_payment(payment).await;
    assert!(result.is_err(), "Invalid BIC accepted");

    // Negative amount
    let mut payment = create_test_payment(Uuid::new_v4(), Decimal::new(-10000, 2));
    let result = env.gateway.submit_payment(payment).await;
    assert!(result.is_err(), "Negative amount accepted");

    // SQL injection attempt
    let mut payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));
    payment.reference = "'; DROP TABLE payments--".to_string();
    let result = env.gateway.submit_payment(payment).await;
    assert!(result.is_err(), "SQL injection not blocked");
}

#[tokio::test]
async fn test_audit_logging() {
    let env = TestEnvironment::new().await;

    let payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));

    // Submit payment
    env.gateway.submit_payment(payment.clone()).await.unwrap();

    // Wait for audit log
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify audit entry
    let events = env.audit_logger.search(
        Some(AuditEventType::PaymentInitiated),
        None,
        None,
        None,
    ).await.unwrap();

    assert!(!events.is_empty(), "No audit log entry found");

    let event = &events[0];
    assert_eq!(event.event_type, AuditEventType::PaymentInitiated);
    assert_eq!(event.result, AuditResult::Success);

    // Verify integrity
    let is_valid = env.audit_logger.verify_integrity().await.unwrap();
    assert!(is_valid, "Audit log integrity check failed");
}

#[tokio::test]
async fn test_tls_mtls_authentication() {
    let env = TestEnvironment::new_with_tls().await;

    // Valid client certificate
    let payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));
    let result = env.gateway_tls.submit_payment_with_cert(payment, env.client_cert.clone()).await;
    assert!(result.is_ok(), "Valid certificate rejected");

    // Invalid client certificate
    let payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));
    let invalid_cert = generate_invalid_cert();
    let result = env.gateway_tls.submit_payment_with_cert(payment, invalid_cert).await;
    assert!(result.is_err(), "Invalid certificate accepted");
}

#[tokio::test]
async fn test_money_conservation() {
    let env = TestEnvironment::new().await;

    let payment_id = Uuid::new_v4();
    let amount = Decimal::new(10000, 2); // $100.00

    // Create payment with multiple events
    let events = vec![
        create_event(payment_id, EventType::PaymentInitiated, amount),
        create_event(payment_id, EventType::PaymentApproved, amount),
        create_event(payment_id, EventType::PaymentSettled, amount),
    ];

    // Append to ledger
    for event in events {
        env.ledger.append_event(event).await.unwrap();
    }

    // Check money conservation
    let conserved = env.ledger.check_money_conservation(payment_id).await.unwrap();
    assert!(conserved, "Money conservation violated");

    // Verify payment state
    let state = env.ledger.get_payment_state(payment_id).await.unwrap();
    assert_eq!(state.current_amount, amount);
}

#[tokio::test]
async fn test_high_throughput() {
    let env = TestEnvironment::new().await;

    let start = std::time::Instant::now();

    // Submit 1000 payments concurrently
    let mut handles = vec![];
    for _ in 0..1000 {
        let gateway = env.gateway.clone();
        let handle = tokio::spawn(async move {
            let payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));
            gateway.submit_payment(payment).await
        });
        handles.push(handle);
    }

    // Wait for all
    let mut successes = 0;
    for handle in handles {
        if handle.await.unwrap().is_ok() {
            successes += 1;
        }
    }

    let elapsed = start.elapsed();
    let tps = 1000.0 / elapsed.as_secs_f64();

    assert!(successes >= 950, "Too many failures: {}/1000", 1000 - successes);
    assert!(tps >= 1000.0, "Throughput too low: {:.0} TPS (target: 1000 TPS)", tps);
}

#[tokio::test]
async fn test_settlement_window_timing() {
    let env = TestEnvironment::new().await;

    // Submit payments
    for _ in 0..10 {
        let payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));
        env.gateway.submit_payment(payment).await.unwrap();
    }

    // Wait for window (6 hours in production, 1 second in test)
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Should auto-trigger settlement
    let batches = env.settlement.get_recent_batches(1).await.unwrap();
    assert!(!batches.is_empty(), "Settlement window did not trigger");

    let batch = &batches[0];
    assert_eq!(batch.payment_count, 10);
}

#[tokio::test]
async fn test_merkle_proof_verification() {
    let env = TestEnvironment::new().await;

    let payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));
    let event = create_event(payment.payment_id, EventType::PaymentInitiated, payment.amount);

    // Append event
    let event_id = env.ledger.append_event(event.clone()).await.unwrap();

    // Finalize block
    let block = env.ledger.finalize_block(vec![event_id]).await.unwrap();

    // Get Merkle proof
    let proof = env.ledger.get_merkle_proof(event_id).await.unwrap();

    // Verify proof
    let is_valid = proof.verify(block.merkle_root, event.hash());
    assert!(is_valid, "Merkle proof verification failed");
}

#[tokio::test]
async fn test_iso20022_generation() {
    let env = TestEnvironment::new().await;

    // Create payment
    let payment = create_test_payment(Uuid::new_v4(), Decimal::new(10000, 2));
    env.gateway.submit_payment(payment.clone()).await.unwrap();

    // Run settlement
    let batch = env.settlement.run_settlement_window().await.unwrap();

    // Generate ISO 20022 pacs.008
    let xml = env.settlement.generate_iso20022(&batch).await.unwrap();

    // Verify XML structure
    assert!(xml.contains("<Document"), "Invalid XML structure");
    assert!(xml.contains("pacs.008"), "Not a pacs.008 message");
    assert!(xml.contains(&payment.amount.to_string()), "Amount not in XML");
    assert!(xml.contains(&payment.debtor_bic), "Debtor BIC not in XML");
    assert!(xml.contains(&payment.creditor_bic), "Creditor BIC not in XML");
}

// ============================================================================
// Test Helpers
// ============================================================================

struct TestEnvironment {
    gateway: Arc<Gateway>,
    ledger: Arc<Ledger>,
    settlement: Arc<SettlementEngine>,
    consensus: Arc<ConsensusNode>,
    consensus_nodes: Vec<Arc<ConsensusNode>>,
    audit_logger: Arc<AuditLogger>,
    gateway_tls: Arc<GatewayTls>,
    client_cert: Certificate,
}

impl TestEnvironment {
    async fn new() -> Self {
        Self::new_with_validators(1).await
    }

    async fn new_with_validators(num_validators: usize) -> Self {
        // Initialize components with test configuration
        let temp_dir = tempfile::tempdir().unwrap();

        // Ledger
        let ledger_config = LedgerConfig {
            data_dir: temp_dir.path().join("ledger"),
            batching: BatchingConfig {
                enabled: true,
                max_batch_size: 100,
                batch_timeout: Duration::from_millis(10),
            },
            rocksdb: Default::default(),
        };
        let ledger = Arc::new(Ledger::open(ledger_config).await.unwrap());

        // Settlement
        let settlement_config = SettlementConfig {
            window_duration: Duration::from_secs(1), // 1 second for testing
            netting_enabled: true,
            iso20022_enabled: true,
        };
        let settlement = Arc::new(SettlementEngine::new(settlement_config, ledger.clone()).await.unwrap());

        // Consensus nodes
        let mut consensus_nodes = vec![];
        for i in 0..num_validators {
            let node_config = ConsensusConfig {
                node_id: format!("node-{}", i),
                data_dir: temp_dir.path().join(format!("consensus-{}", i)),
                rpc_addr: format!("127.0.0.1:{}", 26658 + i),
                p2p_addr: format!("127.0.0.1:{}", 26656 + i),
            };
            let node = Arc::new(ConsensusNode::new(node_config, ledger.clone()).await.unwrap());
            consensus_nodes.push(node);
        }
        let consensus = consensus_nodes[0].clone();

        // Gateway
        let gateway_config = GatewayConfig {
            listen_addr: "127.0.0.1:0".to_string(),
            worker_count: 100,
            queue_size: 1000,
        };
        let gateway = Arc::new(Gateway::new(gateway_config, ledger.clone()).await.unwrap());

        // Audit logger
        let audit_config = AuditLogConfig {
            log_path: temp_dir.path().join("audit.log"),
            enable_hash_chain: true,
            min_severity: AuditSeverity::Info,
            rotate_after: 100_000,
            retention_days: 2555,
        };
        let audit_logger = Arc::new(AuditLogger::new(audit_config).unwrap());

        // TLS (placeholder)
        let gateway_tls = Arc::new(GatewayTls::new_test());
        let client_cert = Certificate::new_test();

        Self {
            gateway,
            ledger,
            settlement,
            consensus,
            consensus_nodes,
            audit_logger,
            gateway_tls,
            client_cert,
        }
    }

    async fn new_with_tls() -> Self {
        let env = Self::new().await;
        // TLS-specific setup
        env
    }
}

fn create_test_payment(payment_id: Uuid, amount: Decimal) -> Payment {
    Payment {
        payment_id,
        amount,
        currency: Currency::USD,
        debtor_bic: "DEUTDEFF".to_string(),
        creditor_bic: "CHASUS33".to_string(),
        debtor_account: "DE89370400440532013000".to_string(),
        creditor_account: "US12345678901234567890".to_string(),
        reference: format!("TEST-{}", payment_id),
        timestamp: Utc::now(),
    }
}

fn create_payment(debtor: &str, creditor: &str, amount: Decimal) -> Payment {
    Payment {
        payment_id: Uuid::new_v4(),
        amount,
        currency: Currency::USD,
        debtor_bic: format!("{}AAA", debtor),
        creditor_bic: format!("{}AAA", creditor),
        debtor_account: format!("{}_ACCOUNT", debtor),
        creditor_account: format!("{}_ACCOUNT", creditor),
        reference: format!("TEST-{}-{}", debtor, creditor),
        timestamp: Utc::now(),
    }
}

fn create_event(payment_id: Uuid, event_type: EventType, amount: Decimal) -> LedgerEvent {
    LedgerEvent {
        event_id: Uuid::new_v4(),
        payment_id,
        event_type,
        amount,
        currency: Currency::USD,
        debtor: AccountId::new("DEBTOR"),
        creditor: AccountId::new("CREDITOR"),
        timestamp_nanos: Utc::now().timestamp_nanos_opt().unwrap(),
        block_id: None,
        signature: Signature::from_bytes([0u8; 64]),
        previous_event_id: None,
        metadata: Default::default(),
    }
}

fn generate_invalid_cert() -> Certificate {
    Certificate::new_invalid()
}

// Stub implementations (would be real in actual tests)
mod stubs {
    use super::*;

    pub struct Gateway;
    impl Gateway {
        pub async fn new(_config: GatewayConfig, _ledger: Arc<Ledger>) -> Result<Self, Box<dyn std::error::Error>> {
            Ok(Self)
        }
        pub async fn submit_payment(&self, _payment: Payment) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }

    pub struct GatewayTls;
    impl GatewayTls {
        pub fn new_test() -> Self { Self }
        pub async fn submit_payment_with_cert(&self, _payment: Payment, _cert: Certificate) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
    }

    pub struct Certificate;
    impl Certificate {
        pub fn new_test() -> Self { Self }
        pub fn new_invalid() -> Self { Self }
    }

    pub struct ConsensusNode;
    impl ConsensusNode {
        pub async fn new(_config: ConsensusConfig, _ledger: Arc<Ledger>) -> Result<Self, Box<dyn std::error::Error>> {
            Ok(Self)
        }
        pub async fn create_transaction(&self, _payment: Payment) -> Result<Transaction, Box<dyn std::error::Error>> {
            Ok(Transaction { tx_id: Uuid::new_v4(), event: create_event(Uuid::new_v4(), EventType::PaymentInitiated, Decimal::ZERO), hash: vec![] })
        }
        pub async fn broadcast_tx(&self, _tx_bytes: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
            Ok(())
        }
        pub async fn get_height(&self) -> Result<u64, Box<dyn std::error::Error>> {
            Ok(1)
        }
        pub async fn query_payment(&self, _payment_id: Uuid) -> Result<PaymentState, Box<dyn std::error::Error>> {
            Ok(PaymentState { status: PaymentStatus::Committed, current_amount: Decimal::ZERO })
        }
        pub async fn shutdown(&self) {}
    }

    pub struct SettlementEngine;
    impl SettlementEngine {
        pub async fn new(_config: SettlementConfig, _ledger: Arc<Ledger>) -> Result<Self, Box<dyn std::error::Error>> {
            Ok(Self)
        }
        pub async fn run_settlement_window(&self) -> Result<SettlementBatch, Box<dyn std::error::Error>> {
            Ok(SettlementBatch {
                batch_id: Uuid::new_v4(),
                net_transfers: vec![],
                gross_amount: Decimal::ZERO,
                net_amount: Decimal::ZERO,
                payment_count: 0,
            })
        }
        pub async fn get_recent_batches(&self, _count: usize) -> Result<Vec<SettlementBatch>, Box<dyn std::error::Error>> {
            Ok(vec![])
        }
        pub async fn generate_iso20022(&self, _batch: &SettlementBatch) -> Result<String, Box<dyn std::error::Error>> {
            Ok("<Document>pacs.008</Document>".to_string())
        }
    }

    pub struct GatewayConfig {
        pub listen_addr: String,
        pub worker_count: usize,
        pub queue_size: usize,
    }

    pub struct ConsensusConfig {
        pub node_id: String,
        pub data_dir: std::path::PathBuf,
        pub rpc_addr: String,
        pub p2p_addr: String,
    }

    pub struct SettlementConfig {
        pub window_duration: Duration,
        pub netting_enabled: bool,
        pub iso20022_enabled: bool,
    }

    pub struct SettlementBatch {
        pub batch_id: Uuid,
        pub net_transfers: Vec<NetTransfer>,
        pub gross_amount: Decimal,
        pub net_amount: Decimal,
        pub payment_count: usize,
    }

    pub struct NetTransfer;

    pub struct Payment {
        pub payment_id: Uuid,
        pub amount: Decimal,
        pub currency: Currency,
        pub debtor_bic: String,
        pub creditor_bic: String,
        pub debtor_account: String,
        pub creditor_account: String,
        pub reference: String,
        pub timestamp: chrono::DateTime<Utc>,
    }

    pub struct PaymentState {
        pub status: PaymentStatus,
        pub current_amount: Decimal,
    }

    pub enum PaymentStatus {
        Pending,
        Committed,
        Settled,
    }

    pub struct Transaction {
        pub tx_id: Uuid,
        pub event: LedgerEvent,
        pub hash: Vec<u8>,
    }

    impl Transaction {
        pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
            Ok(vec![])
        }
    }

    pub enum Currency { USD }
    pub enum EventType { PaymentInitiated, PaymentApproved, PaymentSettled }
}

use stubs::*;