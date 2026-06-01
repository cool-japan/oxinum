//! Integration tests for the half-Lehmer GCD dispatch on
//! `oxinum_int::native::BigUint`.
//!
//! Three test surfaces:
//!
//! 1. Unit edge cases (`gcd(0, 0)`, `gcd(a, 0)`, `gcd(a, a)`, `gcd(1, n)`,
//!    Fibonacci pair coprimality, power-of-two GCDs).
//! 2. Mandatory 300-pair cross-validation: `gcd_lehmer == gcd_binary ==
//!    dashu_int::ubig::gcd` over random `(a, b)` pairs with
//!    `a.limbs, b.limbs ∈ 1..=64`.
//! 3. Proptest invariants (`gcd | a`, `gcd | b`, `gcd >= 1` for non-both-zero).

use dashu_int::ops::Gcd as _;
use dashu_int::UBig;
use oxinum_int::native::{gcd, gcd_binary, BigUint};
use proptest::prelude::*;
use proptest::test_runner::Config as PropConfig;

// ---------------------------------------------------------------------------
// Conversion helpers (native <-> dashu) via byte arrays.
// ---------------------------------------------------------------------------

fn to_dashu(n: &BigUint) -> UBig {
    UBig::from_le_bytes(&n.to_bytes_le())
}

fn from_dashu(u: &UBig) -> BigUint {
    BigUint::from_bytes_le(&u.to_le_bytes())
}

/// Lightweight xorshift PRNG. Deterministic, no extra dev-deps.
#[inline]
fn xorshift64(mut state: u64) -> u64 {
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    state
}

// ---------------------------------------------------------------------------
// Unit edge cases
// ---------------------------------------------------------------------------

#[test]
fn unit_gcd_zero_zero_is_zero() {
    assert_eq!(gcd(BigUint::ZERO, BigUint::ZERO), BigUint::ZERO);
}

#[test]
fn unit_gcd_a_zero() {
    let a = BigUint::from_u64(42);
    assert_eq!(gcd(a.clone(), BigUint::ZERO), a);
    assert_eq!(gcd(BigUint::ZERO, a.clone()), a);
}

#[test]
fn unit_gcd_a_a() {
    let a = BigUint::from_u64(0xDEAD_BEEF_CAFE_BABE);
    assert_eq!(gcd(a.clone(), a.clone()), a);

    // Multi-limb a == a above Lehmer crossover.
    let big = BigUint::from_le_limbs(&[1, 2, 3, 4, 5, 6, 7, 8, 9]);
    assert_eq!(gcd(big.clone(), big.clone()), big);
}

#[test]
fn unit_gcd_one_n() {
    let n = BigUint::from_le_limbs(&[
        0xDEAD_BEEF_CAFE_BABE,
        0x1234_5678_9ABC_DEF0,
        0xAAAA_5555_AAAA_5555,
        0x42,
    ]);
    assert_eq!(gcd(BigUint::one(), n.clone()), BigUint::one());
    assert_eq!(gcd(n, BigUint::one()), BigUint::one());
}

#[test]
fn unit_gcd_power_of_two_pair() {
    // gcd(2^256, 2^512) == 2^256.
    let a = BigUint::one().shl_bits(256);
    let b = BigUint::one().shl_bits(512);
    assert_eq!(gcd(a.clone(), b.clone()), a);
    assert_eq!(gcd(b, a.clone()), a);
}

#[test]
fn unit_gcd_fibonacci_pair_is_one() {
    // gcd(F_n, F_{n+1}) == 1.
    let (f, g) = fib_pair_u64(90);
    let a = BigUint::from_u64(f);
    let b = BigUint::from_u64(g);
    assert_eq!(gcd(a, b), BigUint::one());

    // Above-crossover Fibonacci-like pair: gcd((F<<256), (G<<256)) == 2^256.
    let scale = 256u64;
    let a_big = BigUint::from_u64(f).shl_bits(scale);
    let b_big = BigUint::from_u64(g).shl_bits(scale);
    assert_eq!(gcd(a_big, b_big), BigUint::one().shl_bits(scale));
}

fn fib_pair_u64(n: u32) -> (u64, u64) {
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    for _ in 0..n {
        let next = a.wrapping_add(b);
        a = b;
        b = next;
    }
    (a, b)
}

// ---------------------------------------------------------------------------
// Mandatory 300-pair cross-validation
// ---------------------------------------------------------------------------

#[test]
fn cross_val_300_random_pairs_lehmer_eq_binary_eq_dashu() {
    let mut state: u64 = 0xC001_BEEF_5EED_BABE;
    let target = 300usize;

    for iter in 0..target {
        // Pick limb counts in 1..=64 for each operand. This range straddles
        // the Lehmer crossover (LEHMER_THRESHOLD_LIMBS = 2): we want a mix
        // of below-, at-, and above-threshold cases.
        state = xorshift64(state);
        let a_limbs_count = 1 + (state % 64) as usize;
        state = xorshift64(state);
        let b_limbs_count = 1 + (state % 64) as usize;

        let mut a_limbs = Vec::with_capacity(a_limbs_count);
        for _ in 0..a_limbs_count {
            state = xorshift64(state);
            a_limbs.push(state);
        }
        // Force the top limb to be non-zero so the limb count is honest.
        if a_limbs[a_limbs_count - 1] == 0 {
            a_limbs[a_limbs_count - 1] = 1;
        }
        let mut b_limbs = Vec::with_capacity(b_limbs_count);
        for _ in 0..b_limbs_count {
            state = xorshift64(state);
            b_limbs.push(state);
        }
        if b_limbs[b_limbs_count - 1] == 0 {
            b_limbs[b_limbs_count - 1] = 1;
        }

        let a = BigUint::from_le_limbs(&a_limbs);
        let b = BigUint::from_le_limbs(&b_limbs);

        let g_lehmer = gcd(a.clone(), b.clone());
        let g_binary = gcd_binary(a.clone(), b.clone());
        let g_dashu = from_dashu(&to_dashu(&a).gcd(&to_dashu(&b)));

        assert_eq!(
            g_lehmer,
            g_binary,
            "iter {iter}: gcd_lehmer != gcd_binary\n  a={:?}\n  b={:?}",
            a.as_limbs(),
            b.as_limbs()
        );
        assert_eq!(
            g_lehmer,
            g_dashu,
            "iter {iter}: gcd_lehmer != dashu_gcd\n  a={:?}\n  b={:?}",
            a.as_limbs(),
            b.as_limbs()
        );
    }
}

#[test]
fn cross_val_above_crossover_only() {
    // Tighter test: enforce a.limbs >= 3 and b.limbs >= 3 so the Lehmer
    // path is actually exercised (rather than the small-input fallback).
    let mut state: u64 = 0xDEAD_FACE_F00D_CAFE;
    let cases = 200usize;
    for iter in 0..cases {
        state = xorshift64(state);
        let a_limbs_count = 3 + (state % 30) as usize;
        state = xorshift64(state);
        let b_limbs_count = 3 + (state % 30) as usize;

        let mut a_limbs = Vec::with_capacity(a_limbs_count);
        for _ in 0..a_limbs_count {
            state = xorshift64(state);
            a_limbs.push(state);
        }
        if a_limbs[a_limbs_count - 1] == 0 {
            a_limbs[a_limbs_count - 1] = 1;
        }
        let mut b_limbs = Vec::with_capacity(b_limbs_count);
        for _ in 0..b_limbs_count {
            state = xorshift64(state);
            b_limbs.push(state);
        }
        if b_limbs[b_limbs_count - 1] == 0 {
            b_limbs[b_limbs_count - 1] = 1;
        }

        let a = BigUint::from_le_limbs(&a_limbs);
        let b = BigUint::from_le_limbs(&b_limbs);
        let g_lehmer = gcd(a.clone(), b.clone());
        let g_binary = gcd_binary(a.clone(), b.clone());

        assert_eq!(
            g_lehmer, g_binary,
            "above-crossover iter {iter}: lehmer != binary\n  a_limbs={a_limbs_count}\n  b_limbs={b_limbs_count}",
        );
    }
}

// ---------------------------------------------------------------------------
// Proptest invariants
// ---------------------------------------------------------------------------

fn arb_biguint() -> impl Strategy<Value = BigUint> {
    // Generate random byte sequences up to a few-hundred-byte budget so we
    // exercise both small (single-limb) and multi-limb operands.
    proptest::collection::vec(any::<u8>(), 0..256).prop_map(|bytes| BigUint::from_bytes_le(&bytes))
}

fn arb_nonzero_biguint() -> impl Strategy<Value = BigUint> {
    arb_biguint().prop_filter("non-zero", |n| !n.is_zero())
}

proptest! {
    #![proptest_config(PropConfig::with_cases(256))]

    #[test]
    fn prop_gcd_divides_both(a in arb_nonzero_biguint(), b in arb_nonzero_biguint()) {
        let g = gcd(a.clone(), b.clone());
        prop_assert!(!g.is_zero(), "gcd of two non-zero values must be non-zero");
        // g divides a
        let (_qa, ra) = oxinum_int::native::divrem(&a, &g);
        prop_assert!(ra.is_zero(), "gcd must divide a");
        // g divides b
        let (_qb, rb) = oxinum_int::native::divrem(&b, &g);
        prop_assert!(rb.is_zero(), "gcd must divide b");
    }

    #[test]
    fn prop_gcd_lehmer_matches_dashu(a in arb_biguint(), b in arb_biguint()) {
        let g_native = gcd(a.clone(), b.clone());
        let g_dashu = from_dashu(&to_dashu(&a).gcd(&to_dashu(&b)));
        prop_assert_eq!(g_native, g_dashu);
    }

    #[test]
    fn prop_gcd_lehmer_matches_binary(a in arb_biguint(), b in arb_biguint()) {
        let g_lehmer = gcd(a.clone(), b.clone());
        let g_binary = gcd_binary(a.clone(), b.clone());
        prop_assert_eq!(g_lehmer, g_binary);
    }
}
