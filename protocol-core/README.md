# Protocol Core

**Formal Protocol Layer implementation for DelTran Settlement Rail.**

Implements the four core protocol operations:
- **INSTRUCT_PAYMENT**: Payment initiation with pre-auth eligibility tokens
- **NETTING**: Multilateral netting with bank confirmations
- **FINALIZE**: 2-phase commit settlement with partial settlement support
- **SETTLEMENT_PROOF**: Cryptographic proofs with BFT + HSM signatures

---

## Features

### ✅ State Machine
- Enforces valid protocol transitions
- Prevents invalid state jumps
- Supports retry paths (netting timeout → requeue)

### ✅ Canonical Serialization
- Deterministic SHA3-256 hashing
- Fixed field order (protobuf-compliant)
- Fixed-scale decimals for money

### ✅ Validation
- **Eligibility tokens**: Pre-auth validation, expiry checks, signature verification
- **Replay protection**: Nonce (monotonic) + TTL (5 min window)
- **BIC/IBAN**: Pattern validation
- **Netting thresholds**: $100k min volume, 15% min efficiency, 2+ participants

### ✅ Merkle Trees
- Binary Merkle tree with SHA3-256
- Individual payment inclusion proofs
- Root verification

### ✅ HSM Integration
- **Mock** (Ed25519 software signing) for development
- **PKCS#11 interface** (skeleton for AWS CloudHSM)
- Key rotation support (key epochs)

### ✅ Checkpoints
- Generated every 100 blocks (~10 minutes)
- BFT multi-sig (5 of 7 validators)
- HSM coordinator signature (final seal)
- Checkpoint chaining (prev_checkpoint_id)

### ✅ Settlement Proofs
- Merkle inclusion proofs for each payment
- BFT quorum (5 of 7 validators)
- HSM final seal
- ACL (authorized parties only)

### ✅ Partial Settlement
- Graph decomposition (strongly connected components)
- Atomic components (all banks confirmed)
- Requeues payments involving failed banks

---

## Architecture

```
┌─────────────────────────────────────┐
│         State Machine               │  ← Protocol state transitions
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│         Validation                  │  ← Eligibility, replay, BIC/IBAN
├─────────────────────────────────────┤
│  • Eligibility tokens               │
│  • Nonce + TTL replay protection    │
│  • BIC/IBAN/signatures              │
│  • Netting thresholds               │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│         Canonical                   │  ← Deterministic serialization
├─────────────────────────────────────┤
│  • Fixed field order                │
│  • SHA3-256 hashing                 │
│  • Fixed-scale decimals             │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│         Merkle Trees                │  ← Cryptographic commitments
├─────────────────────────────────────┤
│  • Binary tree                      │
│  • Inclusion proofs                 │
│  • Root verification                │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│         HSM                         │  ← Hardware security
├─────────────────────────────────────┤
│  • Mock (Ed25519)                   │
│  • PKCS#11 (CloudHSM)               │
│  • Key epochs                       │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│         Checkpoints                 │  ← Periodic state commits
├─────────────────────────────────────┤
│  • Every 100 blocks                 │
│  • BFT + HSM signatures             │
│  • Chained (prev_checkpoint_id)     │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│         Proofs                      │  ← Settlement evidence
├─────────────────────────────────────┤
│  • Merkle paths                     │
│  • BFT multi-sig (5/7)              │
│  • HSM final seal                   │
└─────────────┬───────────────────────┘
              │
┌─────────────▼───────────────────────┐
│         Partial Settlement          │  ← Fault tolerance
├─────────────────────────────────────┤
│  • Graph decomposition (SCC)        │
│  • Atomic components                │
│  • Requeue on failure               │
└─────────────────────────────────────┘
```

---

## Usage

### Basic Example

```rust
use protocol_core::{
    InstructPayment, ProtocolValidator, StateMachine, ProtocolState,
};

// 1. Create validator
let mut validator = ProtocolValidator::new();

// 2. Validate payment instruction
let instruction: InstructPayment = /* ... */;
validator.validate_instruct_payment(&instruction)?;

// 3. State machine
let mut sm = StateMachine::new();
sm.transition(ProtocolState::PaymentValidated)?;
sm.transition(ProtocolState::PaymentEligibilityConfirmed)?;
```

### Checkpoint Generation

```rust
use protocol_core::{CheckpointGenerator, HsmMock};
use std::sync::Arc;

// Create HSM (mock for dev, PKCS#11 for production)
let hsm = Arc::new(HsmMock::new("coordinator-key".into(), "2025-Q1".into()));

// Create generator
let generator = CheckpointGenerator::new(
    hsm,
    "deltran-mainnet".into(),
    1 /* proto_version */
);

// Generate checkpoint
let checkpoint = generator.generate(
    100, /* height */
    [0u8; 32], /* prev_checkpoint_id */
    app_hash,
    merkle_root,
    stats,
)?;
```

### Settlement Proof

```rust
use protocol_core::{ProofGenerator, MerkleTree};

let generator = ProofGenerator::new(hsm, "deltran-mainnet".into(), 1);

// Generate proof
let proof = generator.generate_settlement_proof(
    batch_id,
    checkpoint_height,
    prev_checkpoint_id,
    app_hash,
    payment_hashes,
    summary,
    authorized_parties,
)?;

// Verify proof
generator.verify(&proof)?;
```

### Partial Settlement

```rust
use protocol_core::PartialSettlementEngine;

let (components, requeued) = PartialSettlementEngine::decompose(
    &net_transfers,
    &failed_banks,
)?;

for component in components {
    // Finalize atomic component
    if !component.bank_ids.iter().any(|b| failed_banks.contains(b)) {
        // All banks in this component confirmed
        finalize_component(&component)?;
    }
}
```

---

## Testing

```bash
# Unit tests
cargo test --package protocol-core

# Specific module
cargo test --package protocol-core -- validation

# With output
cargo test --package protocol-core -- --nocapture
```

---

## Configuration

### HSM Mock (Development)

```rust
let config = HsmConfig {
    hsm_type: HsmType::Mock,
    key_id: "coordinator-key".into(),
    key_epoch: "2025-Q1".into(),
    algorithm: HsmAlgorithm::Ed25519,
};
```

### HSM PKCS#11 (Production)

```rust
let config = HsmConfig {
    hsm_type: HsmType::Pkcs11 {
        library_path: "/opt/cloudhsm/lib/libcloudhsm_pkcs11.so".into(),
        slot_id: 0,
        pin: env::var("HSM_PIN").unwrap(),
    },
    key_id: "prod-coordinator-key".into(),
    key_epoch: "2025-Q1".into(),
    algorithm: HsmAlgorithm::EcdsaP256,
};
```

---

## Constants

| Constant | Value | Description |
|----------|-------|-------------|
| `PROTOCOL_VERSION` | 1 | Semantic protocol version |
| `DEFAULT_TTL_SECONDS` | 300 | Payment instruction TTL (5 min) |
| `DEFAULT_CHECKPOINT_INTERVAL` | 100 | Blocks between checkpoints (~10 min) |
| `BFT_QUORUM` | 5/7 | Validator quorum (71.4%) |
| `MIN_NETTING_VOLUME` | $100k | Minimum gross volume for netting |
| `MIN_NETTING_EFFICIENCY` | 15% | Minimum netting efficiency |
| `MIN_NETTING_PARTICIPANTS` | 2 | Minimum banks for netting |
| `DEFAULT_2PC_TIMEOUT_SECONDS` | 900 | 2PC timeout (15 min) |

---

## Security

### ✅ No Unsafe Code
```rust
#![forbid(unsafe_code)]
```

### ✅ Replay Protection
- **Nonce**: Monotonically increasing per sender (anti-replay)
- **TTL**: 5-minute window (prevents stale messages)
- **Timestamp**: Future rejection (±5s clock skew tolerance)

### ✅ Signature Verification
- Ed25519 (default)
- ECDSA P-256 (HSM)
- RSA-PSS 4096 (legacy)

### ✅ Canonical Hashing
- SHA3-256 (not SHA2, for quantum resistance)
- Fixed field order
- Deterministic serialization

---

## Roadmap

### MVP (Current)
- ✅ Protocol schemas (protobuf)
- ✅ State machine
- ✅ Canonical serialization
- ✅ Validation (eligibility, replay, BIC/IBAN)
- ✅ Merkle trees
- ✅ HSM mock
- ✅ Checkpoints
- ✅ Settlement proofs
- ✅ Partial settlement

### Production (Next)
- [ ] Real PKCS#11 HSM integration
- [ ] Sanctions screening integration (Dow Jones/Refinitiv)
- [ ] Full ISO 20022 pacs.008 generation
- [ ] Regulatory read-only API
- [ ] Property tests for money invariants
- [ ] Fuzzing (cargo-fuzz)
- [ ] SBOM generation (CycloneDX)

---

## License

Apache 2.0 - See [LICENSE](../LICENSE)

---

**Made with ❤️ by DelTran Team**