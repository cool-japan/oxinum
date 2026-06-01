//! Integration tests for native `BigFloat` transcendental functions: `exp`, `ln`,
//! and the trig family `sin`, `cos`, `tan`, `atan`, `atan2` (T3).

use oxinum_core::OxiNumError;
use oxinum_float::native::{e_const, ln2, pi, BigFloat, RoundingMode};

fn mk(n: i64, prec: u32) -> BigFloat {
    BigFloat::from_i64(n, prec, RoundingMode::HalfEven)
}

/// Check |a - b| < 2^{-tol_bits} using exponent-based comparison.
///
/// The threshold is constructed as a BigFloat via `from_parts` so it works
/// correctly even at high precision (unlike `from_f64(2.0f64.powi(-tol_bits))`
/// which underflows for tol_bits >= 1022).
fn approx_eq(a: &BigFloat, b: &BigFloat, tol_bits: u32) -> bool {
    let diff = a.clone().sub_ref(&b.clone()).abs();
    if diff.is_zero() {
        return true;
    }
    // diff < 2^(-tol_bits) iff the top bit of diff is at position < -tol_bits.
    // top_bit_pos = diff.exponent + diff.mantissa.bit_length() - 1
    // We need top_bit_pos < -(tol_bits as i64)
    let top_bit_pos = diff
        .exponent()
        .saturating_add(diff.mantissa().bit_length() as i64 - 1);
    top_bit_pos < -(tol_bits as i64)
}

// ============================================================
// exp() tests
// ============================================================

#[test]
fn exp_zero_is_one() {
    let x = mk(0, 100);
    let result = x.exp(100, RoundingMode::HalfEven).expect("exp(0)");
    assert_eq!(result, mk(1, 100), "exp(0) should == 1");
}

#[test]
fn exp_one_is_e() {
    let x = mk(1, 100);
    let result = x.exp(100, RoundingMode::HalfEven).expect("exp(1)");
    let e = e_const(100).expect("e");
    assert!(
        approx_eq(&result, &e, 85),
        "exp(1) should approx e; diff = {}",
        (result.to_f64() - e.to_f64()).abs()
    );
}

#[test]
fn exp_times_exp_neg_is_one() {
    let prec = 100u32;
    let x = BigFloat::from_f64(3.7, prec).expect("3.7");
    let ex = x.exp(prec, RoundingMode::HalfEven).expect("exp(x)");
    let neg_x = x.neg();
    let enx = neg_x.exp(prec, RoundingMode::HalfEven).expect("exp(-x)");
    let product = ex.mul_ref_with_mode(&enx, RoundingMode::HalfEven);
    let one = mk(1, prec);
    assert!(
        approx_eq(&product, &one, 85),
        "exp(x)*exp(-x) should approx 1; diff = {}",
        (product.to_f64() - 1.0).abs()
    );
}

#[test]
fn exp_overflow_error() {
    let x = BigFloat::from_f64(800.0, 64).expect("800.0");
    let result = x.exp(64, RoundingMode::HalfEven);
    assert!(
        matches!(result, Err(OxiNumError::Overflow(_))),
        "Expected Overflow, got: {result:?}"
    );
}

#[test]
fn exp_large_negative_is_zero() {
    let x = BigFloat::from_f64(-800.0, 64).expect("-800.0");
    let result = x.exp(64, RoundingMode::HalfEven).expect("exp(-800)");
    assert!(result.is_zero(), "exp(-800) should be zero");
}

/// exp(2) = e^2 approximately 7.389056099.
#[test]
fn exp_two_cross_val_f64() {
    let prec = 64u32;
    let x = mk(2, prec);
    let result = x.exp(prec, RoundingMode::HalfEven).expect("exp(2)");
    let expected = 2.0_f64.exp();
    let rel_err = (result.to_f64() - expected).abs() / expected.abs();
    assert!(
        rel_err < 1e-14,
        "exp(2) cross-val: got {}, expected {expected}; rel_err = {rel_err}",
        result.to_f64()
    );
}

#[test]
fn exp_cross_val_f64_various() {
    let prec = 64u32;
    for x_f64 in [-5.0_f64, -1.0, -0.5, 0.5, 1.5, 3.0, 7.3] {
        let x = BigFloat::from_f64(x_f64, prec).expect("from_f64");
        let result = x.exp(prec, RoundingMode::HalfEven).expect("exp");
        let expected = x_f64.exp();
        let rel_err = (result.to_f64() - expected).abs() / expected.abs().max(1e-300);
        assert!(
            rel_err < 1e-14,
            "exp({x_f64}): got {}, expected {expected}; rel_err = {rel_err}",
            result.to_f64()
        );
    }
}

// ============================================================
// ln() tests
// ============================================================

#[test]
fn ln_one_is_zero() {
    let x = mk(1, 100);
    let result = x.ln(100, RoundingMode::HalfEven).expect("ln(1)");
    assert!(result.is_zero(), "ln(1) should be 0, got: {:?}", result);
}

#[test]
fn ln_e_is_one() {
    let prec = 100u32;
    let e = e_const(prec).expect("e");
    let result = e.ln(prec, RoundingMode::HalfEven).expect("ln(e)");
    let one = mk(1, prec);
    assert!(
        approx_eq(&result, &one, 85),
        "ln(e) should approx 1; got {}, diff = {}",
        result.to_f64(),
        (result.to_f64() - 1.0).abs()
    );
}

#[test]
fn ln_zero_is_domain_error() {
    let x = mk(0, 64);
    let result = x.ln(64, RoundingMode::HalfEven);
    assert!(
        matches!(result, Err(OxiNumError::Domain(_))),
        "Expected Domain, got: {result:?}"
    );
}

#[test]
fn ln_negative_is_domain_error() {
    let x = mk(-1, 64);
    let result = x.ln(64, RoundingMode::HalfEven);
    assert!(
        matches!(result, Err(OxiNumError::Domain(_))),
        "Expected Domain, got: {result:?}"
    );
}

#[test]
fn ln_two_matches_ln2_constant() {
    let prec = 100u32;
    let two = mk(2, prec);
    let computed = two.ln(prec, RoundingMode::HalfEven).expect("ln(2)");
    let expected = ln2(prec).expect("ln2 constant");
    assert!(
        approx_eq(&computed, &expected, 85),
        "ln(2) should match ln2 constant; diff = {}",
        (computed.to_f64() - expected.to_f64()).abs()
    );
}

/// Round-trip: exp(ln(x)) approximately x for various positive x values.
#[test]
fn ln_exp_roundtrip() {
    let prec = 100u32;
    for x_f64 in [0.5_f64, 1.0, 2.0, 10.0, 0.01, 100.0] {
        let x = BigFloat::from_f64(x_f64, prec).expect("from_f64");
        let ln_x = x.ln(prec, RoundingMode::HalfEven).expect("ln");
        let back = ln_x.exp(prec, RoundingMode::HalfEven).expect("exp");
        assert!(
            approx_eq(&back, &x, 80),
            "ln then exp round-trip failed for x={x_f64}; got {}, expected {x_f64}; diff = {}",
            back.to_f64(),
            (back.to_f64() - x_f64).abs()
        );
    }
}

/// Round-trip: ln(exp(x)) approximately x for various x values.
#[test]
fn exp_ln_roundtrip() {
    let prec = 100u32;
    for x_f64 in [-2.0_f64, -0.5, 0.0, 0.5, 1.0, 2.0, 5.0] {
        let x = BigFloat::from_f64(x_f64, prec).expect("from_f64");
        let ex = x.exp(prec, RoundingMode::HalfEven).expect("exp");
        let back = ex.ln(prec, RoundingMode::HalfEven).expect("ln");
        assert!(
            approx_eq(&back, &x, 80),
            "exp then ln round-trip failed for x={x_f64}; got {}, diff = {}",
            back.to_f64(),
            (back.to_f64() - x_f64).abs()
        );
    }
}

/// Cross-validate ln against f64 for moderate values.
#[test]
fn ln_cross_val_f64() {
    let prec = 64u32;
    for x_f64 in [0.01_f64, 0.5, 1.0, 2.0, 10.0, 100.0, 1000.0] {
        let x = BigFloat::from_f64(x_f64, prec).expect("from_f64");
        let result = x.ln(prec, RoundingMode::HalfEven).expect("ln");
        let expected = x_f64.ln();
        let diff = (result.to_f64() - expected).abs();
        let tol = expected.abs().max(1.0) * 1e-14;
        assert!(
            diff <= tol,
            "ln({x_f64}): got {}, expected {expected}; diff = {diff}",
            result.to_f64()
        );
    }
}

/// Higher precision test: ln at 200 bits.
#[test]
fn ln_high_precision() {
    let prec = 200u32;
    let two = mk(2, prec);
    let computed = two
        .ln(prec, RoundingMode::HalfEven)
        .expect("ln(2) at 200 bits");
    let expected = ln2(prec).expect("ln2 constant at 200 bits");
    assert!(
        approx_eq(&computed, &expected, 150),
        "ln(2) at 200 bits failed; diff (f64) = {}",
        (computed.to_f64() - expected.to_f64()).abs()
    );
}

/// exp at higher precision: 200 bits.
#[test]
fn exp_high_precision() {
    let prec = 200u32;
    let one = mk(1, prec);
    let result = one
        .exp(prec, RoundingMode::HalfEven)
        .expect("exp(1) at 200 bits");
    let e = e_const(prec).expect("e_const at 200 bits");
    assert!(
        approx_eq(&result, &e, 150),
        "exp(1) at 200 bits failed; diff (f64) = {}",
        (result.to_f64() - e.to_f64()).abs()
    );
}

// ============================================================
// sin() / cos() / tan() tests (T3)
// ============================================================

#[test]
fn sin_zero_is_zero() {
    let x = mk(0, 100);
    let result = x.sin(100, RoundingMode::HalfEven).expect("sin(0)");
    assert!(result.is_zero(), "sin(0) must be zero");
}

#[test]
fn cos_zero_is_one() {
    let x = mk(0, 100);
    let result = x.cos(100, RoundingMode::HalfEven).expect("cos(0)");
    let one = mk(1, 100);
    assert!(
        approx_eq(&result, &one, 90),
        "cos(0) must be 1; got {}",
        result.to_f64()
    );
}

#[test]
fn pythagorean_identity() {
    let prec = 100u32;
    for x_f64 in [0.3_f64, 1.0, 2.7, -1.5, 5.0, 0.0] {
        let x = BigFloat::from_f64(x_f64, prec).expect("from_f64");
        let s = x.sin(prec, RoundingMode::HalfEven).expect("sin");
        let c = x.cos(prec, RoundingMode::HalfEven).expect("cos");
        let sum = s
            .mul_ref_with_mode(&s, RoundingMode::HalfEven)
            .add_ref_with_mode(
                &c.mul_ref_with_mode(&c, RoundingMode::HalfEven),
                RoundingMode::HalfEven,
            );
        let one = mk(1, prec);
        assert!(
            approx_eq(&sum, &one, 90),
            "sin^2({x_f64})+cos^2({x_f64}) != 1; got {}; diff = {:.2e}",
            sum.to_f64(),
            (sum.to_f64() - 1.0).abs()
        );
    }
}

#[test]
fn sin_pi_is_zero() {
    let prec = 100u32;
    let pi_val = pi(prec).expect("pi");
    let s = pi_val.sin(prec, RoundingMode::HalfEven).expect("sin(pi)");
    assert!(
        s.abs().to_f64() < 1e-25,
        "sin(pi) should be ~0; got {}",
        s.to_f64()
    );
}

#[test]
fn cos_pi_is_minus_one() {
    let prec = 100u32;
    let pi_val = pi(prec).expect("pi");
    let c = pi_val.cos(prec, RoundingMode::HalfEven).expect("cos(pi)");
    let minus_one = mk(-1, prec);
    assert!(
        approx_eq(&c, &minus_one, 90),
        "cos(pi) must be -1; got {}",
        c.to_f64()
    );
}

#[test]
fn sin_cross_val_f64() {
    let prec = 64u32;
    for x_f64 in [-2.5_f64, -1.0, 0.5, 1.0, 2.0, 4.0] {
        let x = BigFloat::from_f64(x_f64, prec).expect("from_f64");
        let result = x.sin(prec, RoundingMode::HalfEven).expect("sin");
        let expected = x_f64.sin();
        let diff = (result.to_f64() - expected).abs();
        assert!(
            diff < 1e-14,
            "sin({x_f64}): got {}, expected {expected}; diff={diff:.2e}",
            result.to_f64()
        );
    }
}

#[test]
fn cos_cross_val_f64() {
    let prec = 64u32;
    for x_f64 in [-2.5_f64, -1.0, 0.5, 1.0, 2.0, 4.0] {
        let x = BigFloat::from_f64(x_f64, prec).expect("from_f64");
        let result = x.cos(prec, RoundingMode::HalfEven).expect("cos");
        let expected = x_f64.cos();
        let diff = (result.to_f64() - expected).abs();
        assert!(
            diff < 1e-14,
            "cos({x_f64}): got {}, expected {expected}; diff={diff:.2e}",
            result.to_f64()
        );
    }
}

#[test]
fn tan_cross_val_f64() {
    let prec = 64u32;
    for x_f64 in [-1.2_f64, -0.5, 0.5, 1.0, 1.3] {
        let x = BigFloat::from_f64(x_f64, prec).expect("from_f64");
        let result = x.tan(prec, RoundingMode::HalfEven).expect("tan");
        let expected = x_f64.tan();
        let diff = (result.to_f64() - expected).abs();
        assert!(
            diff < 1e-14,
            "tan({x_f64}): got {}, expected {expected}; diff={diff:.2e}",
            result.to_f64()
        );
    }
}

// ============================================================
// atan() / atan2() tests (T3)
// ============================================================

#[test]
fn atan_one_times_four_is_pi() {
    let prec = 100u32;
    let one = mk(1, prec);
    let atan_1 = one.atan(prec, RoundingMode::HalfEven).expect("atan(1)");
    let four = mk(4, prec);
    let computed_pi = atan_1
        .mul_ref_with_mode(&four, RoundingMode::HalfEven)
        .with_precision(prec, RoundingMode::HalfEven);
    let expected_pi = pi(prec).expect("pi");
    assert!(
        approx_eq(&computed_pi, &expected_pi, 90),
        "4*atan(1) != pi; got {}; diff={:.2e}",
        computed_pi.to_f64(),
        (computed_pi.to_f64() - expected_pi.to_f64()).abs()
    );
}

#[test]
fn machin_formula() {
    // pi = 16*atan(1/5) - 4*atan(1/239)
    // Use extra precision to compensate for accumulation through mul/sub.
    let prec = 256u32;
    let mode = RoundingMode::HalfEven;
    // Use exact rational arithmetic to get 1/5 and 1/239 at full precision.
    let one_fifth = mk(1, prec)
        .div_ref_with_mode(&mk(5, prec), mode)
        .expect("1/5");
    let one_239 = mk(1, prec)
        .div_ref_with_mode(&mk(239, prec), mode)
        .expect("1/239");
    let atan_5 = one_fifth.atan(prec, mode).expect("atan(1/5)");
    let atan_239 = one_239.atan(prec, mode).expect("atan(1/239)");
    let sixteen = mk(16, prec);
    let four = mk(4, prec);
    let machin_pi = sixteen
        .mul_ref_with_mode(&atan_5, mode)
        .sub_ref_with_mode(&four.mul_ref_with_mode(&atan_239, mode), mode)
        .with_precision(prec, mode);
    let expected_pi = pi(prec).expect("pi");
    // Verify 170 bits of agreement (leaving a ~86-bit margin for accumulated rounding).
    assert!(
        approx_eq(&machin_pi, &expected_pi, 170),
        "Machin formula failed; diff={:.2e}",
        (machin_pi.to_f64() - expected_pi.to_f64()).abs()
    );
}

#[test]
fn atan_cross_val_f64() {
    let prec = 64u32;
    for x_f64 in [-10.0_f64, -2.0, -0.5, 0.0, 0.5, 2.0, 10.0] {
        let x = BigFloat::from_f64(x_f64, prec).expect("from_f64");
        let result = x.atan(prec, RoundingMode::HalfEven).expect("atan");
        let expected = x_f64.atan();
        let diff = (result.to_f64() - expected).abs();
        assert!(
            diff < 1e-14,
            "atan({x_f64}): got {}, expected {expected}; diff={diff:.2e}",
            result.to_f64()
        );
    }
}

#[test]
fn atan2_quadrants() {
    let prec = 64u32;
    let mode = RoundingMode::HalfEven;
    let cases: &[(f64, f64, f64)] = &[
        (1.0, 1.0, std::f64::consts::FRAC_PI_4),
        (1.0, -1.0, 3.0 * std::f64::consts::FRAC_PI_4),
        (-1.0, 1.0, -std::f64::consts::FRAC_PI_4),
        (-1.0, -1.0, -3.0 * std::f64::consts::FRAC_PI_4),
        (1.0, 0.0, std::f64::consts::FRAC_PI_2),
        (-1.0, 0.0, -std::f64::consts::FRAC_PI_2),
    ];
    for &(y_f64, x_f64, expected) in cases {
        let y = BigFloat::from_f64(y_f64, prec).expect("y");
        let x = BigFloat::from_f64(x_f64, prec).expect("x");
        let result = y.atan2(&x, prec, mode).expect("atan2");
        let diff = (result.to_f64() - expected).abs();
        assert!(
            diff < 1e-13,
            "atan2({y_f64},{x_f64}): got {}, expected {expected}; diff={diff:.2e}",
            result.to_f64()
        );
    }
}

#[test]
fn atan2_both_zero_returns_zero() {
    let prec = 64u32;
    let mode = RoundingMode::HalfEven;
    let z = mk(0, prec);
    let result = z.atan2(&z, prec, mode).expect("atan2(0,0)");
    assert!(result.is_zero(), "atan2(0,0) should be 0");
}

// ============================================================
// ln_agm() tests
// ============================================================

#[test]
fn ln_agm_one_is_zero() {
    let x = mk(1, 100);
    let result = x.ln_agm(100, RoundingMode::HalfEven).expect("ln_agm(1)");
    assert!(result.is_zero(), "ln_agm(1) should be 0, got: {result:?}");
}

#[test]
fn ln_agm_zero_is_domain_error() {
    let x = mk(0, 64);
    let result = x.ln_agm(64, RoundingMode::HalfEven);
    assert!(
        matches!(result, Err(OxiNumError::Domain(_))),
        "Expected Domain error for ln_agm(0), got: {result:?}"
    );
}

#[test]
fn ln_agm_negative_is_domain_error() {
    let x = mk(-1, 64);
    let result = x.ln_agm(64, RoundingMode::HalfEven);
    assert!(
        matches!(result, Err(OxiNumError::Domain(_))),
        "Expected Domain error for ln_agm(-1), got: {result:?}"
    );
}

#[test]
fn ln_agm_e_is_approximately_one() {
    let prec = 100u32;
    let e = e_const(prec).expect("e_const");
    let result = e.ln_agm(60, RoundingMode::HalfEven).expect("ln_agm(e)");
    let expected = mk(1, 60);
    assert!(
        approx_eq(&result, &expected, 45),
        "ln_agm(e) should be ≈ 1, got: {} (diff from 1: {})",
        result.to_f64(),
        (result.to_f64() - 1.0).abs()
    );
}

#[test]
fn ln_agm_matches_newton_ln_basic() {
    let prec = 80u32;
    let tol = 60u32;
    let mode = RoundingMode::HalfEven;
    for n in [2i64, 7, 100, 1000] {
        let x = BigFloat::from_i64(n, prec + 64, mode);
        let ln_newton = x.ln(prec, mode).expect("newton ln");
        let ln_agm = x.ln_agm(prec, mode).expect("agm ln");
        assert!(
            approx_eq(&ln_newton, &ln_agm, tol),
            "ln_agm({n}) vs Newton mismatch: newton={}, agm={}",
            ln_newton.to_f64(),
            ln_agm.to_f64()
        );
    }
}

#[test]
fn ln_agm_ln2_matches_constant() {
    let prec = 100u32;
    let two = mk(2, prec);
    let computed = two.ln_agm(prec, RoundingMode::HalfEven).expect("ln_agm(2)");
    let expected = ln2(prec).expect("ln2 constant");
    assert!(
        approx_eq(&computed, &expected, 80),
        "ln_agm(2) should match ln2 constant; diff = {}",
        (computed.to_f64() - expected.to_f64()).abs()
    );
}

#[test]
fn ln_agm_high_precision() {
    // Validate ln_agm at 200-bit precision against Newton ln.
    let prec = 200u32;
    let mode = RoundingMode::HalfEven;
    let x = mk(17, prec + 64);
    let ln_newton = x.ln(prec, mode).expect("newton ln");
    let ln_agm = x.ln_agm(prec, mode).expect("agm ln");
    assert!(
        approx_eq(&ln_newton, &ln_agm, 160),
        "ln_agm(17) at 200-bit precision mismatches Newton; diff={:.2e}",
        (ln_newton.to_f64() - ln_agm.to_f64()).abs()
    );
}

#[test]
fn trig_high_precision_sin_pi_over_4() {
    // sin(pi/4) = cos(pi/4) = 1/sqrt(2) approximately 0.7071067811865476
    let prec = 150u32;
    let pi_val = pi(prec).expect("pi");
    let four = mk(4, prec);
    let pi_over_4 = pi_val
        .div_ref_with_mode(&four, RoundingMode::HalfEven)
        .expect("pi/4");
    let s = pi_over_4
        .sin(prec, RoundingMode::HalfEven)
        .expect("sin(pi/4)");
    let c = pi_over_4
        .cos(prec, RoundingMode::HalfEven)
        .expect("cos(pi/4)");
    assert!(
        approx_eq(&s, &c, 130),
        "sin(pi/4) != cos(pi/4); diff={:.2e}",
        (s.to_f64() - c.to_f64()).abs()
    );
    let expected = std::f64::consts::FRAC_1_SQRT_2;
    assert!(
        (s.to_f64() - expected).abs() < 1e-14,
        "sin(pi/4) != 1/sqrt(2); got {}",
        s.to_f64()
    );
}
