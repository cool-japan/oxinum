//! Integration tests for native BigFloat constants: π, e, ln 2.
//!
//! Validates the Chudnovsky + binary-splitting π, the 1/k! e, and the
//! Hwang atanh ln 2 implementations against known values and against
//! each other.

use oxinum_float::native::{e_const, ln2, pi, BigFloat, RoundingMode};

// ---------------------------------------------------------------------------
// π
// ---------------------------------------------------------------------------

#[test]
fn pi_is_non_zero() {
    let p = pi(64).expect("pi(64)");
    assert!(!p.is_zero());
}

#[test]
fn pi_between_3_and_4() {
    let p = pi(170).expect("pi(170)");
    let three = BigFloat::from_i64(3, 170, RoundingMode::HalfEven);
    let four = BigFloat::from_i64(4, 170, RoundingMode::HalfEven);
    assert!(p > three, "π must be > 3");
    assert!(p < four, "π must be < 4");
}

#[test]
fn pi_precision_at_least_requested() {
    let p = pi(170).expect("pi(170)");
    assert!(p.precision() >= 170);
}

#[test]
fn pi_f64_matches_std() {
    let p = pi(64).expect("pi(64)");
    let diff = (p.to_f64() - std::f64::consts::PI).abs();
    assert!(
        diff < 1e-14,
        "π f64 error too large: {diff:.2e}; got {}",
        p.to_f64()
    );
}

#[test]
fn pi_200_bits_f64_check() {
    // A 200-bit π still rounds to the same f64 as std::f64::consts::PI.
    let p = pi(200).expect("pi(200)");
    let diff = (p.to_f64() - std::f64::consts::PI).abs();
    assert!(diff < 1e-14, "200-bit π f64 error: {diff:.2e}");
}

// ---------------------------------------------------------------------------
// e
// ---------------------------------------------------------------------------

#[test]
fn e_is_non_zero() {
    let e = e_const(64).expect("e_const(64)");
    assert!(!e.is_zero());
}

#[test]
fn e_between_2_and_3() {
    let e = e_const(170).expect("e_const(170)");
    let two = BigFloat::from_i64(2, 170, RoundingMode::HalfEven);
    let three = BigFloat::from_i64(3, 170, RoundingMode::HalfEven);
    assert!(e > two, "e must be > 2");
    assert!(e < three, "e must be < 3");
}

#[test]
fn e_precision_at_least_requested() {
    let e = e_const(170).expect("e_const(170)");
    assert!(e.precision() >= 170);
}

#[test]
fn e_f64_matches_std() {
    let e = e_const(64).expect("e_const(64)");
    let diff = (e.to_f64() - std::f64::consts::E).abs();
    assert!(
        diff < 1e-14,
        "e f64 error too large: {diff:.2e}; got {}",
        e.to_f64()
    );
}

#[test]
fn e_200_bits_f64_check() {
    let e = e_const(200).expect("e_const(200)");
    let diff = (e.to_f64() - std::f64::consts::E).abs();
    assert!(diff < 1e-14, "200-bit e f64 error: {diff:.2e}");
}

// ---------------------------------------------------------------------------
// ln 2
// ---------------------------------------------------------------------------

#[test]
fn ln2_is_non_zero() {
    let l = ln2(64).expect("ln2(64)");
    assert!(!l.is_zero());
}

#[test]
fn ln2_between_0_and_1() {
    let l = ln2(170).expect("ln2(170)");
    let zero = BigFloat::from_i64(0, 170, RoundingMode::HalfEven);
    let one = BigFloat::from_i64(1, 170, RoundingMode::HalfEven);
    assert!(l > zero, "ln 2 must be > 0");
    assert!(l < one, "ln 2 must be < 1");
}

#[test]
fn ln2_precision_at_least_requested() {
    let l = ln2(170).expect("ln2(170)");
    assert!(l.precision() >= 170);
}

#[test]
fn ln2_f64_matches_std() {
    let l = ln2(64).expect("ln2(64)");
    let diff = (l.to_f64() - std::f64::consts::LN_2).abs();
    assert!(
        diff < 1e-14,
        "ln2 f64 error too large: {diff:.2e}; got {}",
        l.to_f64()
    );
}

#[test]
fn ln2_200_bits_f64_check() {
    let l = ln2(200).expect("ln2(200)");
    let diff = (l.to_f64() - std::f64::consts::LN_2).abs();
    assert!(diff < 1e-14, "200-bit ln2 f64 error: {diff:.2e}");
}

// ---------------------------------------------------------------------------
// Cache reuse tests
// ---------------------------------------------------------------------------

#[test]
fn pi_cache_reuse_lower_prec() {
    // Request higher precision first (populates cache), then lower.
    let p100 = pi(100).expect("pi(100)");
    let p50 = pi(50).expect("pi(50) from cache");
    let p80 = pi(80).expect("pi(80) from cache");

    assert!(p100.precision() >= 100);
    assert!(p50.precision() >= 50);
    assert!(p80.precision() >= 80);

    // All should agree to f64 precision.
    let pi_f64 = std::f64::consts::PI;
    assert!((p100.to_f64() - pi_f64).abs() < 1e-14, "p100 f64 mismatch");
    assert!((p50.to_f64() - pi_f64).abs() < 1e-14, "p50 f64 mismatch");
    assert!((p80.to_f64() - pi_f64).abs() < 1e-14, "p80 f64 mismatch");
}

#[test]
fn e_cache_multiple_precisions() {
    let e64 = e_const(64).expect("e(64)");
    let e128 = e_const(128).expect("e(128)");
    let e96 = e_const(96).expect("e(96) from cache");

    let e_f64 = std::f64::consts::E;
    assert!((e64.to_f64() - e_f64).abs() < 1e-14);
    assert!((e128.to_f64() - e_f64).abs() < 1e-14);
    assert!((e96.to_f64() - e_f64).abs() < 1e-14);
}

// ---------------------------------------------------------------------------
// Cross-constant consistency
// ---------------------------------------------------------------------------

#[test]
fn constants_all_positive() {
    let zero = BigFloat::from_i64(0, 64, RoundingMode::HalfEven);
    let p = pi(64).expect("pi");
    let e = e_const(64).expect("e");
    let l = ln2(64).expect("ln2");

    assert!(p > zero, "π should be positive");
    assert!(e > zero, "e should be positive");
    assert!(l > zero, "ln 2 should be positive");
}

#[test]
fn pi_gt_e_gt_ln2() {
    // Known ordering: π ≈ 3.14 > e ≈ 2.718 > ln2 ≈ 0.693
    let p = pi(128).expect("pi(128)");
    let e = e_const(128).expect("e(128)");
    let l = ln2(128).expect("ln2(128)");

    assert!(p > e, "π should be > e");
    assert!(e > l, "e should be > ln2");
}

// ---------------------------------------------------------------------------
// Bit-level agreement: low-prec vs. high-prec independently computed
//
// These tests exercise the RECOMPUTE-AND-UPGRADE cache path:
// call low-precision first (populates cache at low+32 bits), then request
// high-precision (cache miss → recompute at high+32 bits).  Both independent
// computations must agree to the lower precision at the bit level.
// ---------------------------------------------------------------------------

#[test]
fn pi_low_then_high_agrees_at_bit_level() {
    // Note: because caches are global and tests run in parallel or after other
    // tests that may have warmed the cache at a higher precision, we cannot
    // guarantee which code path is taken.  Instead we just verify that two
    // independently returned values agree when both are rounded to the same
    // lower precision.
    let p_low = pi(60).expect("pi(60)");
    let p_high = pi(120).expect("pi(120)");

    // Round p_high down to 60 bits and compare mantissa/exponent directly.
    let p_high_at60 = p_high.with_precision(60, RoundingMode::HalfEven);
    assert_eq!(
        p_low.mantissa(),
        p_high_at60.mantissa(),
        "π(60) and π(120)-truncated-to-60 disagree in mantissa"
    );
    assert_eq!(
        p_low.exponent(),
        p_high_at60.exponent(),
        "π(60) and π(120)-truncated-to-60 disagree in exponent"
    );
    assert_eq!(
        p_low.sign(),
        p_high_at60.sign(),
        "π(60) and π(120)-truncated-to-60 disagree in sign"
    );
}

#[test]
fn e_low_then_high_agrees_at_bit_level() {
    let e_low = e_const(60).expect("e(60)");
    let e_high = e_const(120).expect("e(120)");
    let e_high_at60 = e_high.with_precision(60, RoundingMode::HalfEven);
    assert_eq!(
        e_low.mantissa(),
        e_high_at60.mantissa(),
        "e(60) and e(120)-truncated-to-60 disagree in mantissa"
    );
    assert_eq!(
        e_low.exponent(),
        e_high_at60.exponent(),
        "e(60) and e(120)-truncated-to-60 disagree in exponent"
    );
}

#[test]
fn ln2_low_then_high_agrees_at_bit_level() {
    let l_low = ln2(60).expect("ln2(60)");
    let l_high = ln2(120).expect("ln2(120)");
    let l_high_at60 = l_high.with_precision(60, RoundingMode::HalfEven);
    assert_eq!(
        l_low.mantissa(),
        l_high_at60.mantissa(),
        "ln2(60) and ln2(120)-truncated-to-60 disagree in mantissa"
    );
    assert_eq!(
        l_low.exponent(),
        l_high_at60.exponent(),
        "ln2(60) and ln2(120)-truncated-to-60 disagree in exponent"
    );
}
