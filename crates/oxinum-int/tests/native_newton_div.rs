//! Integration tests for the sub-quadratic big/big division path (item ND1).
//!
//! The shipped algorithm is **Burnikel-Ziegler** recursive division (the
//! standard sub-quadratic big/big divide), dispatched from `checked_divrem`
//! once the divisor reaches `NEWTON_DIV_THRESHOLD` limbs, with the existing
//! single-limb fast path and Knuth Algorithm D kept for small/medium divisors.
//!
//! Oracle: every divisor here is sized to exceed the dispatch threshold, so
//! the public `divrem` entry point runs the Burnikel-Ziegler path. Results are
//! cross-validated against `dashu_int::UBig` (the same oracle that pins
//! Knuth-D), plus the Euclidean invariant `u == q*v + r` with `0 <= r < v`.
//!
//! The exact agreement against the trusted Knuth-D path is asserted by the
//! in-module unit tests in `src/native/div.rs` (which can call the internal
//! recursion directly); these integration tests guarantee the *public*
//! dispatch reaches the new path and matches an independent oracle.

use dashu_int::UBig;
use oxinum_int::native::{BigUint, NEWTON_DIV_THRESHOLD};
use proptest::prelude::*;
use proptest::test_runner::Config as PropConfig;

// The dispatch threshold (in divisor limbs) for the Burnikel-Ziegler path.
// Bound directly to the crate constant so every divisor sized to `>= THRESHOLD`
// is guaranteed to drive `divrem` through the new sub-quadratic path.
const THRESHOLD: usize = NEWTON_DIV_THRESHOLD;

// ---------------------------------------------------------------------------
// Conversion helpers (native <-> dashu) via little-endian byte arrays.
// ---------------------------------------------------------------------------

fn to_dashu(n: &BigUint) -> UBig {
    UBig::from_le_bytes(&n.to_bytes_le())
}

fn from_dashu(u: &UBig) -> BigUint {
    BigUint::from_bytes_le(&u.to_le_bytes())
}

fn limbs_to_native(limbs: &[u64]) -> BigUint {
    BigUint::from_le_limbs(limbs)
}

/// Deterministic xorshift64 PRNG (no extra deps, matches the other test files).
fn xorshift64(mut s: u64) -> u64 {
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    s
}

/// Build a `BigUint` with exactly `n` limbs (top limb forced non-zero).
fn rand_biguint_exact(state: &mut u64, n: usize) -> BigUint {
    let mut limbs = Vec::with_capacity(n);
    for _ in 0..n {
        *state = xorshift64(*state);
        limbs.push(*state);
    }
    if limbs[n - 1] == 0 {
        limbs[n - 1] = 1;
    }
    limbs_to_native(&limbs)
}

/// Assert the public `divrem` (Burnikel-Ziegler path) matches the dashu oracle
/// exactly and satisfies the Euclidean invariant.
fn assert_divrem_matches_dashu(u: &BigUint, v: &BigUint) {
    assert!(!v.is_zero(), "divisor must be non-zero");
    assert!(
        v.as_limbs().len() >= THRESHOLD,
        "test divisor must exceed the dispatch threshold so the BZ path is hit"
    );
    let (q, r) = oxinum_int::native::divrem(u, v);

    // Oracle: dashu UBig div_rem via the standard `/` and `%`.
    let du = to_dashu(u);
    let dv = to_dashu(v);
    let dq = &du / &dv;
    let dr = &du % &dv;
    assert_eq!(q, from_dashu(&dq), "quotient mismatch vs dashu");
    assert_eq!(r, from_dashu(&dr), "remainder mismatch vs dashu");

    // Euclidean invariant against the ORIGINAL operands.
    let back = &(&q * v) + &r;
    assert_eq!(&back, u, "u == q*v + r failed");
    assert!(r < *v, "remainder not < divisor");
}

// ---------------------------------------------------------------------------
// Boundary: sizes just above the dispatch threshold.
// ---------------------------------------------------------------------------

#[test]
fn boundary_just_above_threshold() {
    let mut state = 0xBADC_0FFE_E0DD_F00Du64;
    for extra_u in 0..6usize {
        for extra_v in 0..3usize {
            let vlen = THRESHOLD + extra_v;
            let ulen = vlen + extra_u;
            let v = rand_biguint_exact(&mut state, vlen);
            let u = rand_biguint_exact(&mut state, ulen);
            if u >= v {
                assert_divrem_matches_dashu(&u, &v);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Adversarial cases (all padded above the threshold).
// ---------------------------------------------------------------------------

#[test]
fn adversarial_power_of_two_divisor() {
    // v = 2^(64*K), K above threshold; quotient is a pure limb shift.
    let mut state = 0x0102_0304_0506_0708u64;
    for k in [THRESHOLD, THRESHOLD + 1, THRESHOLD + 13] {
        let mut vlimbs = vec![0u64; k];
        vlimbs.push(1); // 2^(64*k)
        let v = limbs_to_native(&vlimbs);
        let u = rand_biguint_exact(&mut state, k + 19);
        assert_divrem_matches_dashu(&u, &v);
    }
}

#[test]
fn adversarial_arbitrary_power_of_two_bitshift() {
    // v = 2^b for a non-limb-aligned bit count b, padded above threshold.
    let mut state = 0xF0E1_D2C3_B4A5_9687u64;
    let v = BigUint::one().shl_bits((THRESHOLD as u64) * 64 + 37);
    assert!(v.as_limbs().len() >= THRESHOLD);
    let u = rand_biguint_exact(&mut state, THRESHOLD + 22);
    assert_divrem_matches_dashu(&u, &v);
}

#[test]
fn adversarial_divisor_one_bit_below_dividend() {
    // v <= u < 2v  => quotient is exactly 1, remainder = u - v.
    let mut state = 0x1212_3434_5656_7878u64;
    let v = rand_biguint_exact(&mut state, THRESHOLD + 4);
    let u = &v + &v.shr_bits(1);
    assert_divrem_matches_dashu(&u, &v);
}

#[test]
fn adversarial_exact_multiple_remainder_zero() {
    // u = q*v exactly: remainder must be zero (correction-loop edge).
    let mut state = 0x9999_8888_7777_6666u64;
    for qlen in [1usize, 7, 31, 64] {
        let v = rand_biguint_exact(&mut state, THRESHOLD + 6);
        let q = rand_biguint_exact(&mut state, qlen);
        let u = &q * &v;
        let (qq, rr) = oxinum_int::native::divrem(&u, &v);
        assert_eq!(qq, q, "exact-multiple quotient mismatch");
        assert!(rr.is_zero(), "exact-multiple remainder must be zero");
        assert_divrem_matches_dashu(&u, &v);
    }
}

#[test]
fn adversarial_top_limb_exactly_2_pow_63() {
    let mut state = 0xCAFE_F00D_DEAD_BEEFu64;
    let mut vlimbs = Vec::with_capacity(THRESHOLD + 2);
    for _ in 0..(THRESHOLD + 2) {
        state = xorshift64(state);
        vlimbs.push(state);
    }
    vlimbs[THRESHOLD + 1] = 1u64 << 63; // top limb exactly 2^63
    let v = limbs_to_native(&vlimbs);
    let u = rand_biguint_exact(&mut state, THRESHOLD + 30);
    if u >= v {
        assert_divrem_matches_dashu(&u, &v);
    }
}

#[test]
fn adversarial_top_limb_2_pow_63_plus_1() {
    let mut state = 0xBEEF_DEAD_F00D_CAFEu64;
    let mut vlimbs = Vec::with_capacity(THRESHOLD + 2);
    for _ in 0..(THRESHOLD + 2) {
        state = xorshift64(state);
        vlimbs.push(state);
    }
    vlimbs[THRESHOLD + 1] = (1u64 << 63) + 1; // top limb 2^63 + 1
    let v = limbs_to_native(&vlimbs);
    let u = rand_biguint_exact(&mut state, THRESHOLD + 30);
    if u >= v {
        assert_divrem_matches_dashu(&u, &v);
    }
}

#[test]
fn adversarial_highly_asymmetric_1000_by_60() {
    // 1000-limb dividend by a 60-limb divisor (>= threshold): exercises the
    // outer blocking loop heavily.
    let mut state = 0x5DEE_CE15_C0FF_EE00u64;
    let v = rand_biguint_exact(&mut state, 60);
    let u = rand_biguint_exact(&mut state, 1000);
    assert_divrem_matches_dashu(&u, &v);
}

#[test]
fn adversarial_all_ones_divisor() {
    // v = every limb = u64::MAX (top bit set, so shift = 0 at normalization).
    let mut state = 0x0BAD_F00D_C0DE_1234u64;
    let v = limbs_to_native(&vec![u64::MAX; THRESHOLD + 5]);
    let u = rand_biguint_exact(&mut state, THRESHOLD + 40);
    assert_divrem_matches_dashu(&u, &v);
}

// ---------------------------------------------------------------------------
// >= 200 random pairs with u.len in threshold..=500, v.len in threshold..=u.len.
// ---------------------------------------------------------------------------

#[test]
fn random_pairs_cross_val_dashu() {
    let mut state = 0x2C0F_FEE2_C0FF_EE2Cu64;
    let mut tested = 0usize;
    let mut attempts = 0usize;
    while tested < 220 && attempts < 4000 {
        attempts += 1;
        state = xorshift64(state);
        // u.len in [threshold ..= 500].
        let span_u = 500 - THRESHOLD + 1;
        let ulen = THRESHOLD + (state as usize % span_u);
        state = xorshift64(state);
        // v.len in [threshold ..= ulen].
        let span_v = ulen - THRESHOLD + 1;
        let vlen = THRESHOLD + (state as usize % span_v);
        let v = rand_biguint_exact(&mut state, vlen);
        let u = rand_biguint_exact(&mut state, ulen);
        if u < v {
            continue;
        }
        assert_divrem_matches_dashu(&u, &v);
        tested += 1;
    }
    assert!(tested >= 200, "expected >= 200 random pairs, got {tested}");
}

// ---------------------------------------------------------------------------
// Proptest: Euclidean invariant + agreement with dashu oracle.
// ---------------------------------------------------------------------------

/// Strategy producing a divisor with length in `threshold..=threshold+10`.
fn arb_divisor() -> impl Strategy<Value = BigUint> {
    (prop::collection::vec(any::<u64>(), THRESHOLD..=(THRESHOLD + 10))).prop_map(|mut limbs| {
        // Force exactly-len, non-zero top limb so length >= threshold holds.
        let last = limbs.len() - 1;
        if limbs[last] == 0 {
            limbs[last] = 1;
        }
        limbs_to_native(&limbs)
    })
}

/// Strategy producing a dividend with length in `threshold..=threshold+60`.
fn arb_dividend() -> impl Strategy<Value = BigUint> {
    (prop::collection::vec(any::<u64>(), THRESHOLD..=(THRESHOLD + 60)))
        .prop_map(|limbs| limbs_to_native(&limbs))
}

proptest! {
    #![proptest_config(PropConfig {
        cases: 200,
        ..PropConfig::default()
    })]

    #[test]
    fn proptest_euclidean_and_dashu_agreement(u in arb_dividend(), v in arb_divisor()) {
        prop_assume!(!v.is_zero());
        prop_assume!(u >= v);
        prop_assert!(v.as_limbs().len() >= THRESHOLD);

        let (q, r) = oxinum_int::native::divrem(&u, &v);

        // Euclidean invariant.
        let back = &(&q * &v) + &r;
        prop_assert_eq!(&back, &u);
        prop_assert!(r < v);

        // Agreement with the dashu oracle.
        let du = to_dashu(&u);
        let dv = to_dashu(&v);
        prop_assert_eq!(q, from_dashu(&(&du / &dv)));
        prop_assert_eq!(r, from_dashu(&(&du % &dv)));
    }
}
