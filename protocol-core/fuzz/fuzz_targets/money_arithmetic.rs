#![no_main]

use libfuzzer_sys::fuzz_target;
use rust_decimal::Decimal;

fuzz_target!(|data: &[u8]| {
    // Test money arithmetic invariants
    if data.len() < 16 {
        return;
    }

    // Parse two decimal amounts from fuzzer data
    let amount1 = i64::from_le_bytes([
        data[0], data[1], data[2], data[3],
        data[4], data[5], data[6], data[7],
    ]);
    let amount2 = i64::from_le_bytes([
        data[8], data[9], data[10], data[11],
        data[12], data[13], data[14], data[15],
    ]);

    // Convert to Decimal (max 2 decimal places for money)
    let a = Decimal::from(amount1) / Decimal::from(100);
    let b = Decimal::from(amount2) / Decimal::from(100);

    // Test commutative property: a + b == b + a
    if let (Ok(sum1), Ok(sum2)) = (a.checked_add(b), b.checked_add(a)) {
        assert_eq!(sum1, sum2, "Addition is not commutative");
    }

    // Test associative property: (a + b) + c == a + (b + c)
    if data.len() >= 24 {
        let amount3 = i64::from_le_bytes([
            data[16], data[17], data[18], data[19],
            data[20], data[21], data[22], data[23],
        ]);
        let c = Decimal::from(amount3) / Decimal::from(100);

        if let (Some(ab), Some(bc)) = (a.checked_add(b), b.checked_add(c)) {
            if let (Some(ab_c), Some(a_bc)) = (ab.checked_add(c), a.checked_add(bc)) {
                assert_eq!(ab_c, a_bc, "Addition is not associative");
            }
        }
    }

    // Test subtraction inverse: a - b + b == a
    if let (Some(diff), Some(restored)) = (a.checked_sub(b), a.checked_sub(b).and_then(|d| d.checked_add(b))) {
        assert_eq!(restored, a, "Subtraction inverse failed");
    }

    // Test zero identity: a + 0 == a
    let zero = Decimal::ZERO;
    if let Some(sum) = a.checked_add(zero) {
        assert_eq!(sum, a, "Zero identity failed");
    }

    // Test multiplication by 1: a * 1 == a
    let one = Decimal::ONE;
    if let Some(product) = a.checked_mul(one) {
        assert_eq!(product, a, "Multiplication by one failed");
    }

    // Test money conservation in netting scenario
    // If Alice pays Bob $100 and Bob pays Alice $60, net should be $40 to Bob
    let alice_to_bob = a.abs();
    let bob_to_alice = b.abs();

    if let (Some(net1), Some(net2)) = (
        alice_to_bob.checked_sub(bob_to_alice),
        bob_to_alice.checked_sub(alice_to_bob),
    ) {
        // Net amounts should be opposite
        assert_eq!(net1, -net2, "Netting conservation violated");
    }

    // Test rounding to 2 decimal places (money precision)
    let rounded = a.round_dp(2);
    assert!(rounded.scale() <= 2, "Money should have at most 2 decimal places");

    // Test no precision loss in addition
    if let Some(sum) = a.checked_add(b) {
        let rounded_sum = sum.round_dp(2);
        let sum_of_rounded = a.round_dp(2).checked_add(b.round_dp(2)).unwrap().round_dp(2);

        // Difference should be negligible (< 0.01)
        let diff = (rounded_sum - sum_of_rounded).abs();
        assert!(diff < Decimal::from_str_exact("0.01").unwrap(), "Precision loss in addition");
    }
});
