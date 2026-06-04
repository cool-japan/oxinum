//! Integration tests for [`oxinum_complex::CBig`] arithmetic operators,
//! conversions, formatting, and the zero-divisor policy.
//!
//! Hand-computed reference values are checked with exact decimal strings where
//! the arithmetic is closed over the integers / terminating decimals involved,
//! and the panic-vs-`Err` split of the division surface is pinned down:
//!
//! * [`CBig::checked_div`] returns [`OxiNumError::DivByZero`] on a zero divisor;
//! * the `/` **operator** panics on a zero divisor (matching the rest of the
//!   workspace) — asserted with `#[should_panic]`.

use oxinum_complex::{CBig, DBig, OxiNumError};

/// Build a `CBig` from two integer-valued `f64`s.
fn c(re: f64, im: f64) -> CBig {
    CBig::from_f64(re, im).expect("finite parts")
}

/// Assert `z == re + im·i` by exact decimal string compare on both components.
fn assert_parts(z: &CBig, re: &str, im: &str) {
    assert_eq!(z.re().to_string(), re, "real part");
    assert_eq!(z.im().to_string(), im, "imag part");
}

// ---------------------------------------------------------------------------
// Add / Sub / Mul / Div on hand-computed values
// ---------------------------------------------------------------------------

#[test]
fn add_hand_computed() {
    // (1 + 2i) + (3 + 4i) = 4 + 6i.
    assert_parts(&(c(1.0, 2.0) + c(3.0, 4.0)), "4", "6");
}

#[test]
fn sub_hand_computed() {
    // (1 + 2i) − (3 + 4i) = −2 − 2i.
    assert_parts(&(c(1.0, 2.0) - c(3.0, 4.0)), "-2", "-2");
}

#[test]
fn mul_hand_computed() {
    // (1 + 2i)(3 + 4i) = (3 − 8) + (4 + 6)i = −5 + 10i.
    assert_parts(&(c(1.0, 2.0) * c(3.0, 4.0)), "-5", "10");
}

#[test]
fn div_hand_computed() {
    // (3 + 4i) / (1 + 2i) = (11 − 2i)/5 = 2.2 − 0.4i.
    assert_parts(&(c(3.0, 4.0) / c(1.0, 2.0)), "2.2", "-0.4");
}

#[test]
fn add_assign_and_sub_assign() {
    let mut a = c(1.0, 2.0);
    a += c(3.0, 4.0);
    assert_parts(&a, "4", "6");
    a -= &c(1.0, 1.0);
    assert_parts(&a, "3", "5");
}

#[test]
fn mul_assign_and_div_assign() {
    // (1 + i) *= (1 + i) → 2i, then /= (1 + i) → 1 + i.
    let mut a = c(1.0, 1.0);
    a *= &c(1.0, 1.0);
    assert_parts(&a, "0", "2");
    a /= c(1.0, 1.0);
    assert_parts(&a, "1", "1");
}

// ---------------------------------------------------------------------------
// Division by zero: checked_div → Err, operator → panic
// ---------------------------------------------------------------------------

#[test]
fn checked_div_by_zero_is_div_by_zero_err() {
    let z = c(1.0, 1.0);
    assert!(matches!(
        z.checked_div(&CBig::zero()),
        Err(OxiNumError::DivByZero)
    ));
}

#[test]
fn checked_div_nonzero_is_ok() {
    // (3 + 4i)/(1 + 2i) via checked_div agrees with the operator.
    let q = c(3.0, 4.0)
        .checked_div(&c(1.0, 2.0))
        .expect("non-zero divisor");
    assert_parts(&q, "2.2", "-0.4");
}

#[test]
#[should_panic]
fn div_operator_by_zero_panics() {
    let z = c(1.0, 1.0);
    let _ = z / CBig::zero();
}

// ---------------------------------------------------------------------------
// norm_sqr / conj / Neg
// ---------------------------------------------------------------------------

#[test]
fn norm_sqr_is_exact_integer() {
    // |3 + 4i|² = 25 exactly.
    assert_eq!(c(3.0, 4.0).norm_sqr(), DBig::from(25u32));
    assert_eq!(c(3.0, 4.0).norm_sqr().to_string(), "25");
}

#[test]
fn conj_negates_imag() {
    let z = c(2.0, -3.0);
    assert_parts(&z.conj(), "2", "3");
    // conj of a real is unchanged.
    assert_parts(&c(5.0, 0.0).conj(), "5", "0");
}

#[test]
fn neg_owned_and_borrowed() {
    let z = c(2.0, -3.0);
    assert_parts(&(-&z), "-2", "3");
    assert_parts(&(-z), "-2", "3");
}

// ---------------------------------------------------------------------------
// From conversions
// ---------------------------------------------------------------------------

#[test]
fn from_dbig_pair_and_real() {
    let z: CBig = (DBig::from(7), DBig::from(-4)).into();
    assert_parts(&z, "7", "-4");

    let r: CBig = DBig::from(5).into();
    assert_parts(&r, "5", "0");
    assert!(r.is_real());
}

#[test]
fn from_integer_pair_and_scalar() {
    let z: CBig = (1i64, 2i64).into();
    assert_parts(&z, "1", "2");

    let r: CBig = 42i64.into();
    assert_parts(&r, "42", "0");
    assert!(r.is_real());
}

// ---------------------------------------------------------------------------
// Default / PartialEq
// ---------------------------------------------------------------------------

#[test]
fn default_equals_zero() {
    let d = CBig::default();
    assert!(d.is_zero());
    assert!(d == CBig::zero());
}

#[test]
fn partial_eq_component_wise() {
    assert!(c(1.5, -2.0) == c(1.5, -2.0));
    assert!(c(1.5, -2.0) != c(1.5, 2.0));
    assert!(c(1.5, -2.0) != c(-1.5, -2.0));
}

// ---------------------------------------------------------------------------
// Display: "a + bi" and "a - bi"
// ---------------------------------------------------------------------------

#[test]
fn display_positive_imag() {
    assert_eq!(c(2.0, 3.0).to_string(), "2 + 3i");
}

#[test]
fn display_negative_imag() {
    assert_eq!(c(2.0, -3.0).to_string(), "2 - 3i");
}

#[test]
fn display_fractional_negative_imag() {
    assert_eq!(c(1.5, -2.25).to_string(), "1.5 - 2.25i");
}

#[test]
fn display_real_shows_zero_imag() {
    let z: CBig = DBig::from(5).into();
    assert_eq!(z.to_string(), "5 + 0i");
}
