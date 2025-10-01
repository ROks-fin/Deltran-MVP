#![no_main]

use libfuzzer_sys::fuzz_target;
use protocol_core::{InstructPayment, Payment, Account};
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

fuzz_target!(|data: &[u8]| {
    // Try to parse input as various fields
    if data.len() < 32 {
        return;
    }

    // Create InstructPayment with fuzzed data
    let payment_id = Uuid::from_bytes([
        data[0], data[1], data[2], data[3],
        data[4], data[5], data[6], data[7],
        data[8], data[9], data[10], data[11],
        data[12], data[13], data[14], data[15],
    ]);

    // Use remaining bytes for amount (prevent panic on invalid decimal)
    let amount_bytes = &data[16..std::cmp::min(data.len(), 24)];
    let amount = if let Ok(amount_str) = std::str::from_utf8(amount_bytes) {
        Decimal::from_str(amount_str).unwrap_or(Decimal::from(1000))
    } else {
        Decimal::from(1000)
    };

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
        amount,
        currency: "USD".to_string(),
        purpose: Some("Test payment".to_string()),
    };

    let instruct = InstructPayment {
        payment_id,
        payment,
        sender_bank_id: "BANKGB2L".to_string(),
        receiver_bank_id: "CHASUS33".to_string(),
        sender_public_key: vec![0u8; 32],
        sender_signature: vec![0u8; 64],
        timestamp: chrono::Utc::now(),
        nonce: u64::from_le_bytes([
            data[24], data[25], data[26], data[27],
            data[28], data[29], data[30], data[31],
        ]),
        ttl_seconds: 300,
        debit_token: protocol_core::EligibilityToken {
            token_id: Uuid::new_v4().to_string(),
            bank_id: "BANKGB2L".to_string(),
            token_type: protocol_core::TokenType::Debit,
            amount,
            currency: "USD".to_string(),
            account_id: "GB29NWBK60161331926819".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
            issued_at: chrono::Utc::now(),
            signature: vec![0u8; 64],
        },
        credit_token: protocol_core::EligibilityToken {
            token_id: Uuid::new_v4().to_string(),
            bank_id: "CHASUS33".to_string(),
            token_type: protocol_core::TokenType::Credit,
            amount,
            currency: "USD".to_string(),
            account_id: "CH9300762011623852957".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
            issued_at: chrono::Utc::now(),
            signature: vec![0u8; 64],
        },
        uetr: format!("{}", Uuid::new_v4()),
        canonical_hash: [0u8; 32],
    };

    // Test canonical serialization (should not panic)
    let _ = instruct.canonical_bytes();
    let computed_hash = instruct.canonical_hash();

    // Verify determinism
    let bytes2 = instruct.canonical_bytes();
    let hash2 = instruct.canonical_hash();

    assert_eq!(computed_hash, hash2, "Canonical hash must be deterministic");
});
