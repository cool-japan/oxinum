//! Known-value integration tests for [`oxinum_complex::native::BigComplex`].
//!
//! These mirror a representative subset of the decimal `CBig` known-value cases
//! on the native binary-base `BigComplex`, exercising the `(prec, mode)`-shaped
//! numeric API. All transcendentals run at `prec = 80` bits with
//! [`RoundingMode::HalfEven`]; results are compared on the `f64` projection
//! with an absolute tolerance of `1e-12`.

use oxinum_complex::native::{BigComplex, RoundingMode};

/// Working precision in bits.
const PREC: u32 = 80;

/// Rounding mode for every native operation here.
const MODE: RoundingMode = RoundingMode::HalfEven;

/// Absolute tolerance for `f64`-projected comparisons.
const TOL: f64 = 1e-12;

/// Build a `BigComplex` from two `f64`s at [`PREC`] bits.
fn c(re: f64, im: f64) -> BigComplex {
    BigComplex::from_f64(re, im, PREC).expect("finite parts")
}

/// Assert that `(re, im)` is within [`TOL`] of `(re_ref, im_ref)`.
fn assert_close(parts: (f64, f64), re_ref: f64, im_ref: f64, label: &str) {
    let (re, im) = parts;
    assert!(
        (re - re_ref).abs() < TOL,
        "{label}: re = {re}, expected {re_ref}"
    );
    assert!(
        (im - im_ref).abs() < TOL,
        "{label}: im = {im}, expected {im_ref}"
    );
}

#[test]
fn exp_i_pi_is_minus_one() {
    // exp(iπ) = −1 + 0i.
    let z = c(0.0, std::f64::consts::PI);
    let r = z.exp(PREC, MODE).expect("exp");
    assert_close(r.to_f64_parts(), -1.0, 0.0, "exp(iπ)");
}

#[test]
fn ln_minus_one_is_i_pi() {
    // ln(−1) = 0 + iπ.
    let r = c(-1.0, 0.0).ln(PREC, MODE).expect("ln");
    assert_close(r.to_f64_parts(), 0.0, std::f64::consts::PI, "ln(−1)");
}

#[test]
fn sqrt_minus_one_is_i() {
    // sqrt(−1) = 0 + i.
    let r = c(-1.0, 0.0).sqrt(PREC, MODE).expect("sqrt");
    assert_close(r.to_f64_parts(), 0.0, 1.0, "sqrt(−1)");
}

#[test]
fn sqrt_two_i_is_one_plus_i() {
    // sqrt(2i) = 1 + i.
    let r = c(0.0, 2.0).sqrt(PREC, MODE).expect("sqrt");
    assert_close(r.to_f64_parts(), 1.0, 1.0, "sqrt(2i)");
}

#[test]
fn abs_three_four_is_five() {
    // |3 + 4i| = 5.
    let m = c(3.0, 4.0).abs(PREC, MODE).expect("abs");
    assert!((m.to_f64() - 5.0).abs() < TOL, "|3+4i| = {}", m.to_f64());
}

#[test]
fn arg_of_i_is_half_pi() {
    // arg(i) = π/2.
    let a = BigComplex::i(PREC, MODE).arg(PREC, MODE).expect("arg");
    assert!(
        (a.to_f64() - std::f64::consts::FRAC_PI_2).abs() < TOL,
        "arg(i) = {}",
        a.to_f64()
    );
}

#[test]
fn mul_hand_computed() {
    // (1 + 2i)(3 + 4i) = −5 + 10i.
    let z = &c(1.0, 2.0) * &c(3.0, 4.0);
    assert_close(z.to_f64_parts(), -5.0, 10.0, "(1+2i)(3+4i)");
}

#[test]
fn checked_div_by_zero_is_err() {
    let q = c(1.0, 1.0).checked_div(&BigComplex::zero(PREC), PREC, MODE);
    assert!(
        matches!(q, Err(oxinum_complex::OxiNumError::DivByZero)),
        "expected DivByZero, got {q:?}"
    );
}

#[test]
fn checked_div_general() {
    // (3 + 4i)/(1 + 2i) = 2.2 − 0.4i.
    let q = c(3.0, 4.0)
        .checked_div(&c(1.0, 2.0), PREC, MODE)
        .expect("non-zero divisor");
    assert_close(q.to_f64_parts(), 2.2, -0.4, "(3+4i)/(1+2i)");
}
