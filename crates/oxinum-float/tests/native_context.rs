//! Integration tests for [`FloatContext`] precision builder.

use oxinum_float::native::{BigFloat, FloatContext, RoundingMode};

const MODE: RoundingMode = RoundingMode::HalfEven;

fn mk(n: i64, prec: u32) -> BigFloat {
    BigFloat::from_i64(n, prec, MODE)
}

// -----------------------------------------------------------------------
// Builder / accessor tests
// -----------------------------------------------------------------------

#[test]
fn context_new_default_half_even() {
    let ctx = FloatContext::new(64);
    assert_eq!(ctx.precision(), 64);
    assert_eq!(ctx.rounding(), RoundingMode::HalfEven);
}

#[test]
fn context_with_rounding_builder() {
    let ctx = FloatContext::new(100).with_rounding(RoundingMode::ToInf);
    assert_eq!(ctx.precision(), 100);
    assert_eq!(ctx.rounding(), RoundingMode::ToInf);
}

#[test]
fn context_with_rounding_does_not_change_precision() {
    let ctx = FloatContext::new(200).with_rounding(RoundingMode::ToZero);
    assert_eq!(ctx.precision(), 200);
}

#[test]
fn context_is_copy() {
    let ctx = FloatContext::new(64);
    // If FloatContext is Copy, this should compile and both values work.
    let ctx2 = ctx;
    assert_eq!(ctx.precision(), ctx2.precision());
}

// -----------------------------------------------------------------------
// Constant accessors
// -----------------------------------------------------------------------

#[test]
fn context_pi_precision() {
    let ctx = FloatContext::new(128);
    let pi = ctx.pi().expect("pi(128)");
    assert_eq!(pi.precision(), 128);
}

#[test]
fn context_pi_f64_approx() {
    let ctx = FloatContext::new(64);
    let pi = ctx.pi().expect("pi(64)");
    assert!(
        (pi.to_f64() - std::f64::consts::PI).abs() < 1e-14,
        "pi(64) f64 mismatch: {}",
        pi.to_f64()
    );
}

#[test]
fn context_e_const_f64_approx() {
    let ctx = FloatContext::new(64);
    let e = ctx.e_const().expect("e_const(64)");
    assert!(
        (e.to_f64() - std::f64::consts::E).abs() < 1e-14,
        "e_const(64) f64 mismatch: {}",
        e.to_f64()
    );
}

#[test]
fn context_ln2_f64_approx() {
    let ctx = FloatContext::new(64);
    let l = ctx.ln2().expect("ln2(64)");
    assert!(
        (l.to_f64() - std::f64::consts::LN_2).abs() < 1e-14,
        "ln2(64) f64 mismatch: {}",
        l.to_f64()
    );
}

// -----------------------------------------------------------------------
// Transcendental forwarder correctness
// -----------------------------------------------------------------------

#[test]
fn context_sqrt_four_is_two() {
    let ctx = FloatContext::new(64);
    let four = mk(4, 64);
    let result = ctx.sqrt(&four).expect("sqrt(4)");
    assert!(
        (result.to_f64() - 2.0).abs() < 1e-14,
        "sqrt(4) = {}",
        result.to_f64()
    );
}

#[test]
fn context_exp_zero_is_one() {
    let ctx = FloatContext::new(64);
    let zero = BigFloat::zero(64);
    let result = ctx.exp(&zero).expect("exp(0)");
    assert_eq!(result.to_f64(), 1.0);
}

#[test]
fn context_ln_one_is_zero() {
    let ctx = FloatContext::new(64);
    let one = mk(1, 64);
    let result = ctx.ln(&one).expect("ln(1)");
    assert!(result.is_zero(), "ln(1) should be zero, got: {result:?}");
}

#[test]
fn context_exp_ln_roundtrip() {
    // exp(ln(2)) ≈ 2
    let ctx = FloatContext::new(200);
    let two = mk(2, 200);
    let ln2 = ctx.ln(&two).expect("ln(2)");
    let back = ctx.exp(&ln2).expect("exp(ln(2))");
    let diff = (back.to_f64() - 2.0).abs();
    assert!(diff < 1e-14, "exp(ln(2)) should be ≈ 2, diff = {diff}");
}

#[test]
fn context_sin_zero_is_zero() {
    let ctx = FloatContext::new(64);
    let zero = BigFloat::zero(64);
    let result = ctx.sin(&zero).expect("sin(0)");
    assert!(result.is_zero(), "sin(0) should be zero");
}

#[test]
fn context_cos_zero_is_one() {
    let ctx = FloatContext::new(64);
    let zero = BigFloat::zero(64);
    let result = ctx.cos(&zero).expect("cos(0)");
    assert!(
        (result.to_f64() - 1.0).abs() < 1e-14,
        "cos(0) = {}",
        result.to_f64()
    );
}

#[test]
fn context_tan_pi_over_4_is_one() {
    let ctx = FloatContext::new(64);
    let pi = ctx.pi().expect("pi");
    let four = mk(4, 64);
    let pi_over_4 = pi.div_ref(&four).expect("pi/4");
    let result = ctx.tan(&pi_over_4).expect("tan(pi/4)");
    assert!(
        (result.to_f64() - 1.0).abs() < 1e-13,
        "tan(pi/4) = {}",
        result.to_f64()
    );
}

#[test]
fn context_atan_one_is_pi_over_4() {
    let ctx = FloatContext::new(64);
    let one = mk(1, 64);
    let result = ctx.atan(&one).expect("atan(1)");
    assert!(
        (result.to_f64() - std::f64::consts::FRAC_PI_4).abs() < 1e-14,
        "atan(1) = {}",
        result.to_f64()
    );
}

#[test]
fn context_atan2_one_one_is_pi_over_4() {
    let ctx = FloatContext::new(64);
    let one = mk(1, 64);
    let result = ctx.atan2(&one, &one).expect("atan2(1,1)");
    assert!(
        (result.to_f64() - std::f64::consts::FRAC_PI_4).abs() < 1e-14,
        "atan2(1,1) = {}",
        result.to_f64()
    );
}

#[test]
fn context_pow_two_to_ten() {
    let ctx = FloatContext::new(100);
    let two = mk(2, 100);
    let ten = mk(10, 100);
    let result = ctx.pow(&two, &ten).expect("2^10");
    assert!(
        (result.to_f64() - 1024.0).abs() < 1e-10,
        "2^10 = {}",
        result.to_f64()
    );
}

#[test]
fn context_log_100_base_10() {
    let ctx = FloatContext::new(100);
    let hundred = mk(100, 100);
    let ten = mk(10, 100);
    let result = ctx.log(&hundred, &ten).expect("log_10(100)");
    assert!(
        (result.to_f64() - 2.0).abs() < 1e-14,
        "log_10(100) = {}",
        result.to_f64()
    );
}

#[test]
fn context_sin_pi_near_zero() {
    // sin(π) should be very close to zero.
    let ctx = FloatContext::new(200);
    let pi = ctx.pi().expect("pi");
    let sin_pi = ctx.sin(&pi).expect("sin(pi)");
    // sin(π) in f64 should be within floating-point rounding of zero.
    let abs_f64 = sin_pi.abs().to_f64();
    assert!(
        abs_f64 < 1e-30,
        "sin(pi) should be near zero, got {abs_f64}"
    );
}
