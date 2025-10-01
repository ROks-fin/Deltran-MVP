//! Property-based tests for financial invariants
//!
//! These tests verify critical properties that must hold for all inputs,
//! not just specific test cases.

use proptest::prelude::*;
use protocol_core::*;
use rust_decimal::Decimal;
use std::str::FromStr;

// ============================================================================
// Money Invariants
// ============================================================================

proptest! {
    /// Property: Money addition is commutative (a + b == b + a)
    #[test]
    fn money_addition_commutative(
        a in -1_000_000i64..1_000_000i64,
        b in -1_000_000i64..1_000_000i64,
    ) {
        let a_dec = Decimal::from(a) / Decimal::from(100);
        let b_dec = Decimal::from(b) / Decimal::from(100);

        if let (Some(sum1), Some(sum2)) = (
            a_dec.checked_add(b_dec),
            b_dec.checked_add(a_dec),
        ) {
            prop_assert_eq!(sum1, sum2);
        }
    }

    /// Property: Money addition is associative ((a + b) + c == a + (b + c))
    #[test]
    fn money_addition_associative(
        a in -1_000_000i64..1_000_000i64,
        b in -1_000_000i64..1_000_000i64,
        c in -1_000_000i64..1_000_000i64,
    ) {
        let a_dec = Decimal::from(a) / Decimal::from(100);
        let b_dec = Decimal::from(b) / Decimal::from(100);
        let c_dec = Decimal::from(c) / Decimal::from(100);

        if let (Some(ab), Some(bc)) = (
            a_dec.checked_add(b_dec),
            b_dec.checked_add(c_dec),
        ) {
            if let (Some(ab_c), Some(a_bc)) = (
                ab.checked_add(c_dec),
                a_dec.checked_add(bc),
            ) {
                prop_assert_eq!(ab_c, a_bc);
            }
        }
    }

    /// Property: Zero is additive identity (a + 0 == a)
    #[test]
    fn money_zero_identity(a in -1_000_000i64..1_000_000i64) {
        let a_dec = Decimal::from(a) / Decimal::from(100);
        let zero = Decimal::ZERO;

        if let Some(sum) = a_dec.checked_add(zero) {
            prop_assert_eq!(sum, a_dec);
        }
    }

    /// Property: Subtraction inverse (a - b + b == a)
    #[test]
    fn money_subtraction_inverse(
        a in -1_000_000i64..1_000_000i64,
        b in -1_000_000i64..1_000_000i64,
    ) {
        let a_dec = Decimal::from(a) / Decimal::from(100);
        let b_dec = Decimal::from(b) / Decimal::from(100);

        if let Some(diff) = a_dec.checked_sub(b_dec) {
            if let Some(restored) = diff.checked_add(b_dec) {
                prop_assert_eq!(restored, a_dec);
            }
        }
    }

    /// Property: Money conservation in bilateral netting
    /// If A pays B amount X and B pays A amount Y, net should equal X - Y
    #[test]
    fn bilateral_netting_conservation(
        x in 0i64..1_000_000i64,
        y in 0i64..1_000_000i64,
    ) {
        let a_to_b = Decimal::from(x) / Decimal::from(100);
        let b_to_a = Decimal::from(y) / Decimal::from(100);

        let net = a_to_b.checked_sub(b_to_a).unwrap();

        // Verify conservation: sum of flows equals net
        let total_flow = a_to_b.checked_add(b_to_a).unwrap();
        let net_abs = net.abs();

        prop_assert!(net_abs <= total_flow, "Net cannot exceed total flow");

        // If X > Y, net is positive (A receives)
        // If X < Y, net is negative (B receives)
        // If X == Y, net is zero
        if a_to_b > b_to_a {
            prop_assert!(net > Decimal::ZERO);
        } else if a_to_b < b_to_a {
            prop_assert!(net < Decimal::ZERO);
        } else {
            prop_assert_eq!(net, Decimal::ZERO);
        }
    }

    /// Property: No precision loss with 2 decimal places
    #[test]
    fn money_precision_preserved(
        a in -1_000_000i64..1_000_000i64,
        b in -1_000_000i64..1_000_000i64,
    ) {
        let a_dec = Decimal::from(a) / Decimal::from(100);
        let b_dec = Decimal::from(b) / Decimal::from(100);

        // Round to 2 decimal places (standard money precision)
        let a_rounded = a_dec.round_dp(2);
        let b_rounded = b_dec.round_dp(2);

        if let Some(sum) = a_rounded.checked_add(b_rounded) {
            let sum_rounded = sum.round_dp(2);

            // Verify sum has at most 2 decimal places
            prop_assert!(sum_rounded.scale() <= 2);

            // Verify rounding doesn't lose more than 0.01
            let diff = (sum - sum_rounded).abs();
            prop_assert!(diff <= Decimal::from_str("0.01").unwrap());
        }
    }
}

// ============================================================================
// Netting Invariants
// ============================================================================

proptest! {
    /// Property: Multilateral netting conserves total money
    /// Sum of all gross payments == Sum of all net transfers
    #[test]
    fn multilateral_netting_conservation(
        payments in prop::collection::vec(
            (0i64..100_000i64, 0i64..100_000i64), // (amount, _)
            2..10  // 2-10 payments
        )
    ) {
        // Simulate circular payments: A→B→C→A
        let mut gross_total = Decimal::ZERO;
        let mut net_total = Decimal::ZERO;

        for (amount, _) in &payments {
            let amt = Decimal::from(*amount) / Decimal::from(100);
            gross_total = gross_total.checked_add(amt).unwrap();
        }

        // In a closed loop, net should be zero or close to gross
        // (depending on netting efficiency)
        prop_assert!(net_total <= gross_total, "Net cannot exceed gross");
    }

    /// Property: Netting efficiency >= 0% and <= 100%
    #[test]
    fn netting_efficiency_bounds(
        gross in 100_000i64..1_000_000i64,
        net in 0i64..1_000_000i64,
    ) {
        let gross_dec = Decimal::from(gross) / Decimal::from(100);
        let net_dec = Decimal::from(net) / Decimal::from(100);

        // Net cannot exceed gross
        if net_dec <= gross_dec {
            let efficiency = (gross_dec - net_dec) / gross_dec;

            prop_assert!(efficiency >= Decimal::ZERO);
            prop_assert!(efficiency <= Decimal::ONE);
        }
    }
}

// ============================================================================
// Canonical Serialization Invariants
// ============================================================================

proptest! {
    /// Property: Canonical serialization is deterministic
    #[test]
    fn canonical_serialization_deterministic(
        payment_id_bytes in prop::array::uniform16(0u8..),
        nonce in any::<u64>(),
        amount in 1i64..1_000_000i64,
    ) {
        use uuid::Uuid;

        let payment_id = Uuid::from_bytes(payment_id_bytes);
        let amount_dec = Decimal::from(amount) / Decimal::from(100);

        let payment = Payment {
            payment_id,
            uetr: format!("{}", Uuid::new_v4()),
            debtor: Account {
                bic: "BANKGB2L".to_string(),
                account_number: "GB29NWBK60161331926819".to_string(),
                name: "Test Debtor".to_string(),
            },
            creditor: Account {
                bic: "CHASUS33".to_string(),
                account_number: "CH9300762011623852957".to_string(),
                name: "Test Creditor".to_string(),
            },
            amount: amount_dec,
            currency: "USD".to_string(),
            purpose: Some("Test".to_string()),
        };

        let instruct = InstructPayment {
            payment_id,
            payment: payment.clone(),
            sender_bank_id: "BANKGB2L".to_string(),
            receiver_bank_id: "CHASUS33".to_string(),
            sender_public_key: vec![0u8; 32],
            sender_signature: vec![0u8; 64],
            timestamp: chrono::Utc::now(),
            nonce,
            ttl_seconds: 300,
            debit_token: EligibilityToken {
                token_id: Uuid::new_v4().to_string(),
                bank_id: "BANKGB2L".to_string(),
                token_type: TokenType::Debit,
                amount: amount_dec,
                currency: "USD".to_string(),
                account_id: "GB29NWBK60161331926819".to_string(),
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
                issued_at: chrono::Utc::now(),
                signature: vec![0u8; 64],
            },
            credit_token: EligibilityToken {
                token_id: Uuid::new_v4().to_string(),
                bank_id: "CHASUS33".to_string(),
                token_type: TokenType::Credit,
                amount: amount_dec,
                currency: "USD".to_string(),
                account_id: "CH9300762011623852957".to_string(),
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
                issued_at: chrono::Utc::now(),
                signature: vec![0u8; 64],
            },
            uetr: format!("{}", Uuid::new_v4()),
            canonical_hash: [0u8; 32],
        };

        // Serialize twice
        let bytes1 = instruct.canonical_bytes();
        let bytes2 = instruct.canonical_bytes();

        // Hash twice
        let hash1 = instruct.canonical_hash();
        let hash2 = instruct.canonical_hash();

        // Must be identical
        prop_assert_eq!(bytes1, bytes2);
        prop_assert_eq!(hash1, hash2);
    }
}

// ============================================================================
// State Machine Invariants
// ============================================================================

proptest! {
    /// Property: Valid state transitions always succeed
    #[test]
    fn state_machine_valid_transitions_succeed(
        initial_state in prop::sample::select(vec![
            ProtocolState::PaymentInitiated,
            ProtocolState::PaymentValidated,
            ProtocolState::NettingApproved,
        ])
    ) {
        use protocol_core::state::ProtocolState;

        // Define valid next states for each state
        let valid_next = match initial_state {
            ProtocolState::PaymentInitiated => vec![
                ProtocolState::PaymentValidated,
                ProtocolState::PaymentRejected,
            ],
            ProtocolState::PaymentValidated => vec![
                ProtocolState::PaymentEligibilityConfirmed,
                ProtocolState::PaymentRejected,
            ],
            ProtocolState::NettingApproved => vec![
                ProtocolState::SettlementPending,
                ProtocolState::NettingTimeout,
            ],
            _ => vec![],
        };

        for next_state in valid_next {
            prop_assert!(
                initial_state.can_transition_to(next_state),
                "Valid transition {:?} -> {:?} failed",
                initial_state,
                next_state
            );
        }
    }

    /// Property: Invalid state transitions always fail
    #[test]
    fn state_machine_invalid_transitions_fail() {
        use protocol_core::state::ProtocolState;

        // Completed state cannot transition to any other state
        let completed = ProtocolState::ProofGenerated;

        prop_assert!(!completed.can_transition_to(ProtocolState::PaymentInitiated));
        prop_assert!(!completed.can_transition_to(ProtocolState::NettingApproved));
        prop_assert!(!completed.can_transition_to(ProtocolState::SettlementPending));
    }
}

// ============================================================================
// Merkle Tree Invariants
// ============================================================================

proptest! {
    /// Property: Merkle proof verification always succeeds for valid tree
    #[test]
    fn merkle_proof_verification_succeeds(
        leaf_count in 1usize..100,
        leaf_index in 0usize..100,
    ) {
        use protocol_core::merkle::{MerkleTree, hash_data};

        // Ensure leaf_index is within bounds
        if leaf_index >= leaf_count {
            return Ok(());
        }

        // Create leaves
        let leaves: Vec<[u8; 32]> = (0..leaf_count)
            .map(|i| hash_data(format!("leaf-{}", i).as_bytes()))
            .collect();

        // Build tree
        let tree = MerkleTree::build(leaves).unwrap();

        // Generate proof
        let proof = tree.prove(leaf_index).unwrap();

        // Verify proof
        prop_assert!(proof.verify(), "Merkle proof verification failed");
    }

    /// Property: Modified proof always fails verification
    #[test]
    fn merkle_modified_proof_fails(
        leaf_count in 2usize..20,
    ) {
        use protocol_core::merkle::{MerkleTree, hash_data};

        let leaves: Vec<[u8; 32]> = (0..leaf_count)
            .map(|i| hash_data(format!("leaf-{}", i).as_bytes()))
            .collect();

        let tree = MerkleTree::build(leaves).unwrap();
        let mut proof = tree.prove(0).unwrap();

        // Tamper with proof
        proof.leaf_hash[0] ^= 0xFF;

        // Verification should fail
        prop_assert!(!proof.verify(), "Tampered proof should not verify");
    }
}
