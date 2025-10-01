//! Protocol types (Rust representations of protobuf messages)

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =========================================================================
// PAYMENT TYPES
// =========================================================================

/// Payment instruction (INSTRUCT_PAYMENT message)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructPayment {
    /// Payment ID (UUIDv7)
    pub payment_id: Uuid,
    /// UETR (ISO 20022)
    pub uetr: String,
    /// Idempotency key
    pub idempotency_key: String,
    /// Anti-replay nonce (monotonic per sender)
    pub nonce: u64,
    /// Timestamp (for TTL enforcement)
    pub timestamp: DateTime<Utc>,
    /// TTL in seconds (default 300)
    pub ttl_seconds: u32,
    /// Network ID
    pub network_id: String,
    /// Corridor ID (e.g., "UAE-IND")
    pub corridor_id: String,
    /// Payment details
    pub payment: PaymentDetails,
    /// Debit eligibility token
    pub debit_token: EligibilityToken,
    /// Credit eligibility token
    pub credit_token: EligibilityToken,
    /// Canonical hash (SHA3-256)
    pub canonical_hash: [u8; 32],
    /// Sender signature (Ed25519)
    pub sender_signature: Vec<u8>,
    /// Sender public key
    pub sender_public_key: Vec<u8>,
    /// Sender bank ID (BIC/LEI)
    pub sender_bank_id: String,
}

/// Payment details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentDetails {
    /// Amount (fixed-point string)
    pub amount: Decimal,
    /// ISO 4217 currency code
    pub currency: String,
    /// Decimal scale (2 for USD)
    pub scale: u32,
    /// Debtor account
    pub debtor: Account,
    /// Creditor account
    pub creditor: Account,
    /// Purpose code (ISO 20022)
    pub purpose_code: String,
    /// Remittance information
    pub remittance_info: Option<String>,
    /// Settlement method
    pub settlement_method: SettlementMethod,
    /// Value date
    pub value_date: DateTime<Utc>,
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Account ID (IBAN or local)
    pub account_id: String,
    /// Account name
    pub account_name: String,
    /// BIC code
    pub bic: String,
    /// LEI (Legal Entity Identifier)
    pub lei: Option<String>,
    /// Bank name
    pub bank_name: String,
    /// Country code (ISO 3166-1 alpha-2)
    pub country_code: String,
    /// Address (for Travel Rule compliance)
    pub address: Option<Address>,
}

/// Address
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub address_line_1: String,
    pub address_line_2: Option<String>,
    pub city: String,
    pub postal_code: String,
    pub country_code: String,
}

/// Settlement method
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SettlementMethod {
    /// Real-time (no netting)
    Instant,
    /// Payment vs Payment (atomic)
    Pvp,
    /// Batch netting (default)
    Netting,
    /// Legacy correspondent banking
    Correspondent,
}

// =========================================================================
// ELIGIBILITY TOKEN
// =========================================================================

/// Eligibility token (pre-auth proof)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EligibilityToken {
    /// Token ID
    pub token_id: Uuid,
    /// Issuing bank BIC/LEI
    pub bank_id: String,
    /// Payment reference
    pub payment_id: Uuid,
    /// Reserved amount
    pub amount: Decimal,
    /// Currency
    pub currency: String,
    /// Issue timestamp
    pub issued_at: DateTime<Utc>,
    /// Expiry timestamp (short TTL, 15 min)
    pub expires_at: DateTime<Utc>,
    /// Token type (debit or credit)
    pub token_type: TokenType,
    /// Account ID
    pub account_id: String,
    /// Bank signature (Ed25519)
    pub signature: Vec<u8>,
    /// Public key for verification
    pub public_key: Vec<u8>,
}

/// Token type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenType {
    /// Debit token (debtor bank reserves outgoing funds)
    Debit,
    /// Credit token (creditor bank confirms receiving capacity)
    Credit,
}

// =========================================================================
// NETTING TYPES
// =========================================================================

/// Netting proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NettingProposal {
    /// Batch ID
    pub batch_id: Uuid,
    /// Window ID (timestamp)
    pub window_id: String,
    /// Corridor ID
    pub corridor_id: String,
    /// Currency
    pub currency: String,
    /// Window start
    pub window_start: DateTime<Utc>,
    /// Window end
    pub window_end: DateTime<Utc>,
    /// Proposal timestamp
    pub proposed_at: DateTime<Utc>,
    /// Bilateral obligations
    pub obligations: Vec<BilateralObligation>,
    /// Net transfers (after netting)
    pub net_transfers: Vec<NetTransfer>,
    /// Merkle root of obligations
    pub merkle_root: [u8; 32],
    /// Gross amount (before netting)
    pub gross_amount: Decimal,
    /// Net amount (after netting)
    pub net_amount: Decimal,
    /// Netting efficiency (0.0-1.0)
    pub netting_efficiency: f64,
    /// Banks that must confirm
    pub requires_confirmations: Vec<String>,
    /// Threshold checks
    pub meets_min_volume: bool,
    pub meets_min_efficiency: bool,
    pub participant_count: usize,
}

/// Bilateral obligation (A owes B)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BilateralObligation {
    pub obligation_id: Uuid,
    pub debtor_bank: String,
    pub creditor_bank: String,
    pub gross_amount: Decimal,
    pub currency: String,
    pub payment_count: usize,
    pub payment_ids: Vec<Uuid>,
}

/// Net transfer (after netting optimization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetTransfer {
    pub transfer_id: Uuid,
    pub from_bank: String,
    pub to_bank: String,
    pub net_amount: Decimal,
    pub currency: String,
    /// ISO 20022 pacs.008 instruction (optional in MVP)
    pub iso20022_instruction: Option<Vec<u8>>,
}

// =========================================================================
// BANK CONFIRMATION
// =========================================================================

/// Bank confirmation (ACK/NACK)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankConfirmation {
    pub confirmation_id: Uuid,
    pub batch_id: Uuid,
    pub bank_id: String,
    pub status: ConfirmationStatus,
    pub reason: Option<String>,
    pub confirmed_at: DateTime<Utc>,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
    pub liquidity: Option<LiquidityAttestation>,
}

/// Confirmation status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConfirmationStatus {
    /// Approved
    Ack,
    /// Rejected
    Nack,
    /// Timeout (no response)
    Timeout,
}

/// Liquidity attestation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityAttestation {
    pub bank_id: String,
    pub currency: String,
    pub reserved_amount: Decimal,
    pub valid_until: DateTime<Utc>,
    pub signature: Vec<u8>,
}

// =========================================================================
// FINALIZATION (2PC)
// =========================================================================

/// Finalize settlement (2PC COMMIT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalizeSettlement {
    pub batch_id: Uuid,
    pub confirmations: Vec<BankConfirmation>,
    pub all_banks_ready: bool,
    pub instructions: Vec<SettlementInstruction>,
    pub atomic_components: Vec<AtomicComponent>,
    pub requeued_payments: Vec<Uuid>,
    pub coordinator_id: String,
    pub finalized_at: DateTime<Utc>,
}

/// Settlement instruction (actual wire transfer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementInstruction {
    pub instruction_id: Uuid,
    pub from_bank: String,
    pub to_bank: String,
    pub amount: Decimal,
    pub currency: String,
    pub iso20022_pacs008: Option<Vec<u8>>,
    pub status: InstructionStatus,
    pub executed_at: Option<DateTime<Utc>>,
}

/// Instruction status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstructionStatus {
    Pending,
    Executed,
    Failed,
    Timeout,
}

/// Atomic component (strongly connected subgraph)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicComponent {
    pub component_id: Uuid,
    pub bank_ids: Vec<String>,
    pub net_transfers: Vec<NetTransfer>,
    pub total_amount: Decimal,
    pub finalized: bool,
}

// =========================================================================
// SETTLEMENT PROOF
// =========================================================================

/// Settlement proof (cryptographic evidence)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementProof {
    pub proof_id: Uuid,
    pub batch_id: Uuid,
    pub checkpoint_height: u64,
    pub merkle_root: [u8; 32],
    pub merkle_paths: Vec<MerkleProofPath>,
    pub app_hash: [u8; 32],
    pub prev_checkpoint_id: [u8; 32],
    pub network_id: String,
    pub proto_version: u16,
    pub batch_finalized_at: DateTime<Utc>,
    pub proof_generated_at: DateTime<Utc>,
    pub validator_signatures: Vec<ValidatorSignature>,
    pub hsm_signature: HsmSignature,
    pub summary: BatchSummary,
    pub authorized_parties: Vec<String>,
}

/// Merkle proof path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProofPath {
    pub payment_id: Uuid,
    pub leaf_hash: [u8; 32],
    pub sibling_hashes: Vec<[u8; 32]>,
    pub leaf_index: u32,
}

/// Validator signature (BFT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSignature {
    pub validator_id: String,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
    pub signed_at: DateTime<Utc>,
}

/// HSM signature (final seal)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HsmSignature {
    pub hsm_key_id: String,
    pub key_epoch: String,
    pub algorithm: SignatureAlgorithm,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
    pub signed_at: DateTime<Utc>,
}

/// Signature algorithm
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    Ed25519,
    EcdsaP256,
    RsaPss4096,
}

/// Batch summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    pub corridor_id: String,
    pub currency: String,
    pub payment_count: usize,
    pub bank_count: usize,
    pub gross_amount: Decimal,
    pub net_amount: Decimal,
    pub netting_efficiency: f64,
    pub net_transfer_count: usize,
    pub partial_settlement: bool,
    pub requeued_count: usize,
}

// =========================================================================
// CHECKPOINT
// =========================================================================

/// Checkpoint (periodic state commitment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub checkpoint_id: [u8; 32],
    pub height: u64,
    pub prev_checkpoint_id: [u8; 32],
    pub app_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub network_id: String,
    pub proto_version: u16,
    pub timestamp: DateTime<Utc>,
    pub validator_signatures: Vec<ValidatorSignature>,
    pub hsm_signature: HsmSignature,
    pub stats: CheckpointStats,
}

/// Checkpoint statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointStats {
    pub total_payments: u64,
    pub total_batches: u64,
    pub total_volume: Decimal,
    pub active_corridors: usize,
    pub active_banks: usize,
}